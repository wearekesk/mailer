//! Open / click tracking service.
//!
//! Event storage representation:
//!
//! - `links.clicks`      → CSV of subscriber ids: `"123,456,789"` (one
//!                         entry per click event — no dedup).
//! - `campaigns.opens`   → comma-separated `"<sid>:<country>"` pairs:
//!                         `"123:US,456:GB,123:US"` (one entry per open
//!                         event — no dedup; same sid may appear many
//!                         times). The country is the recipient's
//!                         `subscribers.country` at the moment of the
//!                         open. Empty country renders as `""` after the
//!                         colon.
//! - `ares_emails.opens` → same shape as campaigns.opens.
//!
//! Reports derive:
//! - **unique opens**  = count of distinct subscriber ids across entries
//! - **total opens**   = count of entries
//! - **top countries** = entries grouped by country (descending)
//!
//! Functions return the resolved redirect URL (for click tracking) or just
//! `Ok(())` for opens.

use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};

use crate::database::entities::{ares_emails, campaigns, links, subscribers};
use crate::utils::errors::{AppError, AppResult};

/// Result of a tracked click. `redirect_to` is the original URL stored on the
/// `links` row (with personalization tags resolved by the caller if desired).
#[derive(Debug, Clone)]
pub struct ClickResult {
    pub link_id: i32,
    pub subscriber_id: i32,
    pub campaign_id: i32,
    pub redirect_to: String,
    /// `Some(id)` when the link belongs to an autoresponder email rather than
    /// a normal campaign.
    pub ares_emails_id: Option<i32>,
}

/// Record a click and return the destination URL.
///
/// Appends `subscriber_id` to `links.clicks` (CSV), then triggers an open
/// if this is the subscriber's first interaction with the campaign.
pub async fn record_click(
    db: &DatabaseConnection,
    link_id: i32,
    subscriber_id: i32,
    campaign_id: i32,
) -> AppResult<ClickResult> {
    let link = links::Entity::find_by_id(link_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("link {link_id}")))?;

    let ares_emails_id = link.ares_emails_id;
    let redirect_to = link.link.clone().unwrap_or_default();

    // Append subscriber id to the CSV (no de-dup).
    let new_clicks = match link.clicks.as_deref() {
        None | Some("") => subscriber_id.to_string(),
        Some(existing) => format!("{existing},{subscriber_id}"),
    };
    let mut active: links::ActiveModel = link.into();
    active.clicks = Set(Some(new_clicks));
    active.update(db).await?;

    // First click implicitly counts as an open.
    if !is_already_opened(db, campaign_id, ares_emails_id, subscriber_id).await? {
        record_open(db, campaign_id, ares_emails_id, subscriber_id).await?;
    }

    // Bump the subscriber's `timestamp` if links_tracking != 2 ("links only").
    let track_engagement = match ares_emails_id {
        None => campaigns::Entity::find_by_id(campaign_id)
            .one(db)
            .await?
            .and_then(|c| c.links_tracking)
            .unwrap_or(0),
        Some(id) => ares_emails::Entity::find_by_id(id)
            .one(db)
            .await?
            .and_then(|c| c.links_tracking)
            .unwrap_or(0),
    } != 2;

    if track_engagement {
        if let Some(s) = subscribers::Entity::find_by_id(subscriber_id).one(db).await? {
            let mut active: subscribers::ActiveModel = s.into();
            active.timestamp = Set(Some(chrono::Utc::now().timestamp()));
            active.update(db).await?;
        }
    }

    Ok(ClickResult {
        link_id,
        subscriber_id,
        campaign_id,
        redirect_to,
        ares_emails_id,
    })
}

/// Record an open event. Appends `sid:country` to the opens CSV.
/// `ares_emails_id` is `Some` for autoresponder opens,
/// `None` for normal campaign opens. `country` falls back to the
/// subscriber's stored country when the caller doesn't already know it.
pub async fn record_open(
    db: &DatabaseConnection,
    campaign_id: i32,
    ares_emails_id: Option<i32>,
    subscriber_id: i32,
) -> AppResult<()> {
    let country = subscribers::Entity::find_by_id(subscriber_id)
        .one(db)
        .await?
        .and_then(|s| s.country)
        .map(|c| c.trim().to_ascii_uppercase())
        .filter(|c| !c.is_empty())
        .unwrap_or_default();
    let entry = format!("{subscriber_id}:{country}");

    let opens = load_opens(db, campaign_id, ares_emails_id).await?;
    let new_opens = if opens.is_empty() {
        entry
    } else {
        format!("{opens},{entry}")
    };
    save_opens(db, campaign_id, ares_emails_id, &new_opens).await?;
    Ok(())
}

// ── Internals ────────────────────────────────────────────────────────────────

async fn is_already_opened(
    db: &DatabaseConnection,
    campaign_id: i32,
    ares_emails_id: Option<i32>,
    subscriber_id: i32,
) -> AppResult<bool> {
    let opens = load_opens(db, campaign_id, ares_emails_id).await?;
    Ok(parse_opens(&opens)
        .iter()
        .any(|(sid, _)| *sid == subscriber_id))
}

async fn load_opens(
    db: &DatabaseConnection,
    campaign_id: i32,
    ares_emails_id: Option<i32>,
) -> AppResult<String> {
    Ok(match ares_emails_id {
        None => campaigns::Entity::find_by_id(campaign_id)
            .one(db)
            .await?
            .and_then(|c| c.opens)
            .unwrap_or_default(),
        Some(id) => ares_emails::Entity::find_by_id(id)
            .one(db)
            .await?
            .and_then(|c| c.opens)
            .unwrap_or_default(),
    })
}

async fn save_opens(
    db: &DatabaseConnection,
    campaign_id: i32,
    ares_emails_id: Option<i32>,
    new_value: &str,
) -> AppResult<()> {
    match ares_emails_id {
        None => {
            if let Some(c) = campaigns::Entity::find_by_id(campaign_id).one(db).await? {
                let mut active: campaigns::ActiveModel = c.into();
                active.opens = Set(Some(new_value.to_string()));
                active.update(db).await?;
            }
        }
        Some(id) => {
            if let Some(c) = ares_emails::Entity::find_by_id(id).one(db).await? {
                let mut active: ares_emails::ActiveModel = c.into();
                active.opens = Set(Some(new_value.to_string()));
                active.update(db).await?;
            }
        }
    }
    Ok(())
}

/// Parse an opens CSV in `sid:country,sid:country,...` shape.
/// Each entry is one open event; the same `sid` may appear multiple
/// times. The country half is whatever the caller wrote.
fn parse_opens(raw: &str) -> Vec<(i32, String)> {
    raw.split(',')
        .filter(|s| !s.is_empty())
        .filter_map(|entry| {
            let mut parts = entry.splitn(2, ':');
            let sid: i32 = parts.next()?.trim().parse().ok()?;
            let country = parts.next().unwrap_or("").trim().to_string();
            Some((sid, country))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_opens;

    #[test]
    fn parses_serialized_opens() {
        let raw = "12:US,34:GB,12:US,56:";
        let parsed = parse_opens(raw);
        assert_eq!(
            parsed,
            vec![
                (12, "US".to_string()),
                (34, "GB".to_string()),
                (12, "US".to_string()),
                (56, "".to_string()),
            ]
        );
    }

    #[test]
    fn empty_is_empty() {
        assert!(parse_opens("").is_empty());
        assert!(parse_opens(",").is_empty());
    }
}
