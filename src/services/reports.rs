//! Campaign performance reports — totals, rates, top-clicked links,
//! unsubscribe / bounce / complaint lists.
//!
//! Reads the same storage formats that `services::tracking` writes so the
//! numbers stay consistent with the live click/open counters.

use std::collections::HashSet;

use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};

use crate::database::entities::{campaigns, links, subscribers};
use crate::utils::errors::AppResult;

#[derive(Debug, Clone)]
pub struct CampaignReport {
    pub recipients: i64,
    pub to_send: i64,
    pub unique_opens: i64,
    pub total_opens: i64,
    pub unique_clickers: i64,
    pub total_clicks: i64,
    pub unsubscribed: i64,
    pub bounced: i64,
    pub complained: i64,
    pub top_links: Vec<LinkClickRow>,
    pub unsubscribed_subs: Vec<RecipientRow>,
    pub bounced_subs: Vec<RecipientRow>,
    pub complained_subs: Vec<RecipientRow>,
    /// Top recipient countries (ISO-3166 alpha-2 code, count). Up to
    /// 12 entries — drives the country pie chart on the report page.
    pub top_countries: Vec<(String, i64)>,
}

#[derive(Debug, Clone)]
pub struct LinkClickRow {
    pub url: String,
    pub clicks: i64,
}

#[derive(Debug, Clone)]
pub struct RecipientRow {
    pub email: String,
    pub name: Option<String>,
}

impl CampaignReport {
    pub fn open_rate(&self) -> f64 {
        if self.recipients == 0 {
            0.0
        } else {
            (self.unique_opens as f64 / self.recipients as f64) * 100.0
        }
    }

    pub fn click_rate(&self) -> f64 {
        if self.recipients == 0 {
            0.0
        } else {
            (self.unique_clickers as f64 / self.recipients as f64) * 100.0
        }
    }

    pub fn unsubscribe_rate(&self) -> f64 {
        if self.recipients == 0 {
            0.0
        } else {
            (self.unsubscribed as f64 / self.recipients as f64) * 100.0
        }
    }

    pub fn bounce_rate(&self) -> f64 {
        if self.recipients == 0 {
            0.0
        } else {
            (self.bounced as f64 / self.recipients as f64) * 100.0
        }
    }
}

/// Build the full report for a campaign id.
pub async fn build(db: &DatabaseConnection, campaign_id: i32) -> AppResult<CampaignReport> {
    let campaign = match campaigns::Entity::find_by_id(campaign_id).one(db).await? {
        Some(c) => c,
        None => return Ok(empty()),
    };

    let recipients = campaign.recipients.unwrap_or(0) as i64;
    let to_send = campaign.to_send.unwrap_or(0) as i64;

    // Opens are stored as `"sid:country,sid:country,..."` in
    // `campaigns.opens` — one entry per open event (no dedup).
    // Derive both engagement counts and the country
    // breakdown from the same scan.
    let raw_opens = campaign.opens.clone().unwrap_or_default();
    let (unique_opens, total_opens, top_countries) = summarize_opens(&raw_opens);

    // Clicks: each `links` row has a CSV of subscriber ids that clicked it.
    let link_rows = links::Entity::find()
        .filter(links::Column::CampaignId.eq(campaign_id))
        .order_by_asc(links::Column::Id)
        .all(db)
        .await?;

    let mut total_clicks = 0i64;
    let mut all_clickers: HashSet<i32> = HashSet::new();
    let mut top_links: Vec<LinkClickRow> = Vec::new();
    for row in &link_rows {
        let clicks_csv = row.clicks.clone().unwrap_or_default();
        let mut count_for_link = 0i64;
        for sid in clicks_csv.split(',').filter(|s| !s.is_empty()) {
            if let Ok(parsed) = sid.trim().parse::<i32>() {
                all_clickers.insert(parsed);
                count_for_link += 1;
                total_clicks += 1;
            }
        }
        if count_for_link > 0 {
            top_links.push(LinkClickRow {
                url: row.link.clone().unwrap_or_default(),
                clicks: count_for_link,
            });
        }
    }
    top_links.sort_by(|a, b| b.clicks.cmp(&a.clicks));
    top_links.truncate(10);

    // Per-status subscriber lists, scoped to last_campaign = this campaign.
    let unsubscribed_subs = scope_recipients(db, campaign_id, RecipientFilter::Unsubscribed).await?;
    let bounced_subs = scope_recipients(db, campaign_id, RecipientFilter::Bounced).await?;
    let complained_subs = scope_recipients(db, campaign_id, RecipientFilter::Complained).await?;

    let unsubscribed = unsubscribed_subs.len() as i64;
    let bounced = bounced_subs.len() as i64;
    let complained = complained_subs.len() as i64;

    Ok(CampaignReport {
        recipients,
        to_send,
        unique_opens,
        total_opens,
        unique_clickers: all_clickers.len() as i64,
        total_clicks,
        unsubscribed,
        bounced,
        complained,
        top_links,
        unsubscribed_subs,
        bounced_subs,
        complained_subs,
        top_countries,
    })
}

/// Walk a `sid:country,sid:country,...` opens CSV once and derive
/// (unique opens, total opens, top countries). Country is uppercased
/// and missing values bucket under `"—"` so the pie chart still
/// shows them.
fn summarize_opens(raw: &str) -> (i64, i64, Vec<(String, i64)>) {
    use std::collections::{HashMap, HashSet};
    let mut unique: HashSet<i32> = HashSet::new();
    let mut total: i64 = 0;
    let mut by_country: HashMap<String, i64> = HashMap::new();
    for entry in raw.split(',') {
        if entry.is_empty() {
            continue;
        }
        let mut parts = entry.splitn(2, ':');
        let Some(sid) = parts.next().and_then(|s| s.trim().parse::<i32>().ok()) else {
            continue;
        };
        let country = parts
            .next()
            .map(|c| c.trim().to_ascii_uppercase())
            .filter(|c| !c.is_empty())
            .unwrap_or_else(|| "—".to_string());
        unique.insert(sid);
        total += 1;
        *by_country.entry(country).or_insert(0) += 1;
    }
    let mut sorted: Vec<(String, i64)> = by_country.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted.truncate(12);
    (unique.len() as i64, total, sorted)
}

#[derive(Debug, Clone, Copy)]
enum RecipientFilter {
    Unsubscribed,
    Bounced,
    Complained,
}

async fn scope_recipients(
    db: &DatabaseConnection,
    campaign_id: i32,
    filter: RecipientFilter,
) -> AppResult<Vec<RecipientRow>> {
    let mut query = subscribers::Entity::find()
        .filter(subscribers::Column::LastCampaign.eq(campaign_id));
    query = match filter {
        RecipientFilter::Unsubscribed => query.filter(subscribers::Column::Unsubscribed.eq(1i16)),
        RecipientFilter::Bounced => query.filter(subscribers::Column::Bounced.eq(1i16)),
        RecipientFilter::Complained => query.filter(subscribers::Column::Complaint.eq(1i16)),
    };
    let rows = query
        .order_by_asc(subscribers::Column::Id)
        .paginate(db, 200)
        .fetch_page(0)
        .await?;
    Ok(rows
        .into_iter()
        .map(|s| RecipientRow {
            email: s.email.unwrap_or_default(),
            name: s.name,
        })
        .collect())
}

fn empty() -> CampaignReport {
    CampaignReport {
        recipients: 0,
        to_send: 0,
        unique_opens: 0,
        total_opens: 0,
        unique_clickers: 0,
        total_clicks: 0,
        unsubscribed: 0,
        bounced: 0,
        complained: 0,
        top_links: Vec::new(),
        unsubscribed_subs: Vec::new(),
        bounced_subs: Vec::new(),
        complained_subs: Vec::new(),
        top_countries: Vec::new(),
    }
}

// ── Autoresponder-email report ──────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct AresEmailReport {
    pub email_id: i32,
    pub title: String,
    pub recipients: i64,
    pub unique_opens: i64,
    pub total_opens: i64,
    pub unique_clickers: i64,
    pub total_clicks: i64,
    pub top_links: Vec<LinkClickRow>,
}

/// Build a per-AR-email report. Same opens/clicks math as
/// `CampaignReport`, but reading from `ares_emails.opens` and the
/// `links` rows scoped by `links.ares_email_id`.
pub async fn build_ares_email(
    db: &DatabaseConnection,
    email_id: i32,
) -> AppResult<AresEmailReport> {
    use crate::database::entities::ares_emails;
    let email = match ares_emails::Entity::find_by_id(email_id).one(db).await? {
        Some(e) => e,
        None => return Ok(AresEmailReport::default()),
    };
    let recipients = email.recipients.unwrap_or(0) as i64;

    let raw_opens = email.opens.clone().unwrap_or_default();
    let (unique_opens, total_opens, _ares_top_countries) = summarize_opens(&raw_opens);

    let link_rows = links::Entity::find()
        .filter(links::Column::AresEmailsId.eq(email_id))
        .order_by_asc(links::Column::Id)
        .all(db)
        .await?;

    let mut total_clicks = 0i64;
    let mut all_clickers: HashSet<i32> = HashSet::new();
    let mut top_links: Vec<LinkClickRow> = Vec::new();
    for row in &link_rows {
        let clicks_csv = row.clicks.clone().unwrap_or_default();
        let mut count_for_link = 0i64;
        for sid in clicks_csv.split(',').filter(|s| !s.is_empty()) {
            if let Ok(parsed) = sid.trim().parse::<i32>() {
                all_clickers.insert(parsed);
                count_for_link += 1;
                total_clicks += 1;
            }
        }
        if count_for_link > 0 {
            top_links.push(LinkClickRow {
                url: row.link.clone().unwrap_or_default(),
                clicks: count_for_link,
            });
        }
    }
    top_links.sort_by(|a, b| b.clicks.cmp(&a.clicks));
    top_links.truncate(10);

    Ok(AresEmailReport {
        email_id,
        title: email.title.unwrap_or_default(),
        recipients,
        unique_opens,
        total_opens,
        unique_clickers: all_clickers.len() as i64,
        total_clicks,
        top_links,
    })
}
