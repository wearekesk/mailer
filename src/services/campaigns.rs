use std::collections::HashSet;

use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

use crate::database::entities::{apps, campaigns, lists, seg, subscribers};
use crate::utils::errors::{AppError, AppResult};

// ── Parameter types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CreateCampaignParams {
    pub user_id: i32,
    /// Required when saving a draft (`send_campaign == false` and no schedule).
    /// When sending or scheduling, the brand is derived from the lists/segments
    /// instead and may be left as `None`.
    pub brand_id: Option<i32>,
    pub from_name: String,
    pub from_email: String,
    pub reply_to: String,
    pub title: Option<String>,
    pub subject: String,
    pub plain_text: Option<String>,
    pub html_text: String,
    pub list_ids: Vec<i32>,
    pub segment_ids: Vec<i32>,
    pub exclude_list_ids: Vec<i32>,
    pub exclude_segment_ids: Vec<i32>,
    pub track_opens: i16,
    pub track_clicks: i16,
    pub send_campaign: bool,
    /// Unix epoch seconds; `None` means send immediately or save as draft.
    pub schedule_date_time: Option<i64>,
    pub schedule_timezone: Option<String>,
    pub query_string: Option<String>,
}

/// Internal outcome enum. The route layer maps this to a typed response that
/// uses [`crate::utils::opaque_id::OpaqueId`] for the campaign id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CampaignOutcome {
    Draft { campaign_id: i32 },
    Scheduled { campaign_id: i32 },
    Sending { campaign_id: i32 },
}

#[derive(Debug, thiserror::Error)]
pub enum CampaignValidationError {
    #[error("One or more list IDs are invalid")]
    InvalidList,
    #[error("One or more segment IDs are invalid")]
    InvalidSegment,
    #[error("List or segment IDs does not belong to a single brand")]
    MultipleBrands,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Create a campaign:
///
/// - `send_campaign = true` → insert an immediate-send campaign and decrement
///   the brand quota counter.
/// - `schedule_date_time = Some(_)` → insert a scheduled campaign.
/// - otherwise → insert a draft campaign.
pub async fn create(
    db: &DatabaseConnection,
    params: CreateCampaignParams,
) -> AppResult<CampaignOutcome> {
    let now = chrono::Utc::now().timestamp();

    let mut brand_id = params.brand_id;
    let mut to_send: i32 = 0;

    if params.send_campaign || params.schedule_date_time.is_some() {
        let derived_brand = derive_brand_from_targets(
            db,
            &params.list_ids,
            &params.segment_ids,
            &params.exclude_list_ids,
            &params.exclude_segment_ids,
        )
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

        let derived_brand = derived_brand
            .ok_or_else(|| AppError::BadRequest("no targets supplied".to_string()))?;
        brand_id = Some(derived_brand);

        let gdpr_only = apps::Entity::find_by_id(derived_brand)
            .one(db)
            .await?
            .and_then(|a| a.gdpr_only)
            .unwrap_or(0)
            == 1;

        to_send = count_recipients(
            db,
            &params.list_ids,
            &params.segment_ids,
            &params.exclude_list_ids,
            &params.exclude_segment_ids,
            gdpr_only,
        )
        .await? as i32;
    }

    let app =
        brand_id.ok_or_else(|| AppError::BadRequest("brand id required for draft".to_string()))?;

    let lists_csv = ids_to_csv(&params.list_ids, "0");
    let segs_csv = ids_to_csv(&params.segment_ids, "0");
    let lists_excl_csv = ids_to_csv(&params.exclude_list_ids, "0");
    let segs_excl_csv = ids_to_csv(&params.exclude_segment_ids, "0");

    if let Some(scheduled_at) = params.schedule_date_time {
        let row = campaigns::ActiveModel {
            user_id: Set(Some(params.user_id)),
            app: Set(Some(app)),
            from_name: Set(Some(params.from_name)),
            from_email: Set(Some(params.from_email)),
            reply_to: Set(Some(params.reply_to)),
            subject: Set(Some(params.subject)),
            title: Set(params.title),
            plain_text: Set(params.plain_text),
            html_text: Set(Some(params.html_text)),
            wysiwyg: Set(Some(1)),
            send_date: Set(Some(scheduled_at.to_string())),
            timezone: Set(params.schedule_timezone),
            lists: Set(Some(lists_csv)),
            segs: Set(Some(segs_csv)),
            lists_excl: Set(Some(lists_excl_csv)),
            segs_excl: Set(Some(segs_excl_csv)),
            query_string: Set(params.query_string),
            opens_tracking: Set(Some(params.track_opens)),
            links_tracking: Set(Some(params.track_clicks)),
            quota_deducted: Set(Some(to_send)),
            ..Default::default()
        };
        let inserted = row.insert(db).await?;
        return Ok(CampaignOutcome::Scheduled {
            campaign_id: inserted.id,
        });
    }

    if params.send_campaign {
        let row = campaigns::ActiveModel {
            user_id: Set(Some(params.user_id)),
            app: Set(Some(app)),
            from_name: Set(Some(params.from_name)),
            from_email: Set(Some(params.from_email)),
            reply_to: Set(Some(params.reply_to)),
            subject: Set(Some(params.subject)),
            title: Set(params.title),
            plain_text: Set(params.plain_text),
            html_text: Set(Some(params.html_text)),
            wysiwyg: Set(Some(1)),
            sent: Set(Some(now.to_string())),
            to_send: Set(Some(to_send)),
            recipients: Set(Some(0)),
            send_date: Set(Some("0".to_string())),
            lists: Set(Some(lists_csv.clone())),
            to_send_lists: Set(Some(lists_csv)),
            segs: Set(Some(segs_csv)),
            lists_excl: Set(Some(lists_excl_csv)),
            segs_excl: Set(Some(segs_excl_csv)),
            timezone: Set(Some("0".to_string())),
            query_string: Set(params.query_string),
            opens_tracking: Set(Some(params.track_opens)),
            links_tracking: Set(Some(params.track_clicks)),
            ..Default::default()
        };
        let inserted = row.insert(db).await?;

        if let Some(brand) = apps::Entity::find_by_id(app).one(db).await? {
            if brand.allocated_quota.unwrap_or(-1) != -1 {
                let new_quota = brand.current_quota.unwrap_or(0) + to_send;
                let mut active: apps::ActiveModel = brand.into();
                active.current_quota = Set(Some(new_quota));
                active.update(db).await?;
            }
        }

        return Ok(CampaignOutcome::Sending {
            campaign_id: inserted.id,
        });
    }

    let row = campaigns::ActiveModel {
        user_id: Set(Some(params.user_id)),
        app: Set(Some(app)),
        from_name: Set(Some(params.from_name)),
        from_email: Set(Some(params.from_email)),
        reply_to: Set(Some(params.reply_to)),
        subject: Set(Some(params.subject)),
        title: Set(params.title),
        plain_text: Set(params.plain_text),
        html_text: Set(Some(params.html_text)),
        wysiwyg: Set(Some(1)),
        query_string: Set(params.query_string),
        opens_tracking: Set(Some(params.track_opens)),
        links_tracking: Set(Some(params.track_clicks)),
        ..Default::default()
    };
    let inserted = row.insert(db).await?;
    Ok(CampaignOutcome::Draft {
        campaign_id: inserted.id,
    })
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn derive_brand_from_targets(
    db: &DatabaseConnection,
    list_ids: &[i32],
    segment_ids: &[i32],
    exclude_list_ids: &[i32],
    exclude_segment_ids: &[i32],
) -> Result<Option<i32>, CampaignValidationError> {
    let mut brands: Vec<i32> = Vec::new();

    for &id in list_ids.iter().chain(exclude_list_ids.iter()) {
        let row = lists::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(|_| CampaignValidationError::InvalidList)?;
        let row = row.ok_or(CampaignValidationError::InvalidList)?;
        if let Some(b) = row.app {
            brands.push(b);
        }
    }

    for &id in segment_ids.iter().chain(exclude_segment_ids.iter()) {
        let row = seg::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(|_| CampaignValidationError::InvalidSegment)?;
        let row = row.ok_or(CampaignValidationError::InvalidSegment)?;
        if let Some(b) = row.app {
            brands.push(b);
        }
    }

    if brands.is_empty() {
        return Ok(None);
    }
    let first = brands[0];
    if brands.iter().any(|b| *b != first) {
        return Err(CampaignValidationError::MultipleBrands);
    }
    Ok(Some(first))
}

/// Walk every subscriber row that should receive the campaign. The result
/// is a `Vec<(subscriber_id, email)>` sorted by id ascending — duplicates
/// (same email on multiple lists) are deduplicated keeping the lowest id.
///
/// Both `count_recipients` and the scheduler use this so the displayed
/// `to_send` total exactly matches who's mailed.
pub async fn resolve_recipient_ids(
    db: &DatabaseConnection,
    list_ids: &[i32],
    segment_ids: &[i32],
    exclude_list_ids: &[i32],
    exclude_segment_ids: &[i32],
    gdpr_only: bool,
) -> AppResult<Vec<(i32, String)>> {
    use sea_orm::QueryOrder;

    if list_ids.is_empty() && segment_ids.is_empty() {
        return Ok(Vec::new());
    }

    use sea_orm::sea_query::Expr;

    let mut query = subscribers::Entity::find()
        .filter(
            Expr::col(subscribers::Column::Unsubscribed)
                .is_null()
                .or(Expr::col(subscribers::Column::Unsubscribed).eq(0i16)),
        )
        .filter(
            Expr::col(subscribers::Column::Bounced)
                .is_null()
                .or(Expr::col(subscribers::Column::Bounced).eq(0i16)),
        )
        .filter(
            Expr::col(subscribers::Column::Complaint)
                .is_null()
                .or(Expr::col(subscribers::Column::Complaint).eq(0i16)),
        )
        .filter(subscribers::Column::Confirmed.eq(1i16));

    if !list_ids.is_empty() {
        query = query.filter(subscribers::Column::List.is_in(list_ids.iter().copied()));
    }

    if !exclude_list_ids.is_empty() {
        let excluded_emails = subscribers::Entity::find()
            .filter(subscribers::Column::List.is_in(exclude_list_ids.iter().copied()))
            .all(db)
            .await?
            .into_iter()
            .filter_map(|s| s.email)
            .collect::<Vec<_>>();
        if !excluded_emails.is_empty() {
            query = query.filter(subscribers::Column::Email.is_not_in(excluded_emails));
        }
    }

    if gdpr_only {
        query = query.filter(subscribers::Column::Gdpr.eq(1i16));
    }

    // Segment includes — union of matching ids across all segs.
    if !segment_ids.is_empty() {
        let mut included: HashSet<i32> = HashSet::new();
        for seg_id in segment_ids {
            let seg = match crate::services::segments::find(db, *seg_id).await? {
                Some(s) => s,
                None => continue,
            };
            let list = seg.list.unwrap_or(0);
            for id in
                crate::services::segments::get_matching_subscriber_ids(db, *seg_id, list).await?
            {
                included.insert(id);
            }
        }
        if !included.is_empty() {
            query = query.filter(
                subscribers::Column::Id.is_in(included.iter().copied().collect::<Vec<_>>()),
            );
        }
    }

    // Segment excludes — union of matching ids; remove from result.
    if !exclude_segment_ids.is_empty() {
        let mut excluded: HashSet<i32> = HashSet::new();
        for seg_id in exclude_segment_ids {
            let seg = match crate::services::segments::find(db, *seg_id).await? {
                Some(s) => s,
                None => continue,
            };
            let list = seg.list.unwrap_or(0);
            for id in
                crate::services::segments::get_matching_subscriber_ids(db, *seg_id, list).await?
            {
                excluded.insert(id);
            }
        }
        if !excluded.is_empty() {
            query = query.filter(
                subscribers::Column::Id.is_not_in(excluded.iter().copied().collect::<Vec<_>>()),
            );
        }
    }

    let rows = query
        .order_by_asc(subscribers::Column::Id)
        .all(db)
        .await?;

    // Dedup by email (lowest id wins) so a subscriber on multiple lists
    // gets exactly one send.
    let mut seen_emails = HashSet::new();
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let Some(email) = r.email else { continue };
        if seen_emails.insert(email.clone()) {
            out.push((r.id, email));
        }
    }
    Ok(out)
}

async fn count_recipients(
    db: &DatabaseConnection,
    list_ids: &[i32],
    segment_ids: &[i32],
    exclude_list_ids: &[i32],
    exclude_segment_ids: &[i32],
    gdpr_only: bool,
) -> AppResult<u64> {
    let recipients = resolve_recipient_ids(
        db,
        list_ids,
        segment_ids,
        exclude_list_ids,
        exclude_segment_ids,
        gdpr_only,
    )
    .await?;
    Ok(recipients.len() as u64)
}

/// Scan a rendered HTML body for "suspicious" hrefs — empty hrefs,
/// `href="#"` placeholders, or URLs missing a `http://` / `https://`
/// / `mailto:` scheme. Returns the offending URLs (deduped).
pub fn check_broken_links(html: &str) -> Vec<String> {
    use std::collections::BTreeSet;
    let re = match regex::Regex::new(r#"(?i)href\s*=\s*["']([^"']*)["']"#) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut bad: BTreeSet<String> = BTreeSet::new();
    for caps in re.captures_iter(html) {
        let href = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
        // Personalization tags resolve at render time — leave them alone.
        if href.contains('[') && href.contains(']') {
            continue;
        }
        let lower = href.to_ascii_lowercase();
        let is_ok = href == "#"
            || href.is_empty()  // empty handled separately below
            || lower.starts_with("http://")
            || lower.starts_with("https://")
            || lower.starts_with("mailto:")
            || lower.starts_with("tel:")
            || lower.starts_with('/')
            || lower.starts_with('#')
            || lower.starts_with("//");
        if !is_ok || href.is_empty() || href == "#" {
            bad.insert(href.to_string());
        }
    }
    bad.into_iter().take(50).collect()
}

/// Flip `campaigns.ignore_checks = 1` so the editor stops nagging about
/// suspicious hrefs before send.
pub async fn dismiss_broken_link_check(
    db: &DatabaseConnection,
    campaign_id: i32,
) -> AppResult<()> {
    use sea_orm::ActiveModelTrait;
    let row = campaigns::Entity::find_by_id(campaign_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("campaign {campaign_id}")))?;
    let mut active: campaigns::ActiveModel = row.into();
    active.ignore_checks = Set(Some(1));
    active.update(db).await?;
    Ok(())
}

/// Calculate totals: preview how many subscribers will receive this campaign
/// before scheduling/sending.
pub async fn calculate_totals(
    db: &DatabaseConnection,
    brand_id: i32,
    list_ids: &[i32],
    segment_ids: &[i32],
    exclude_list_ids: &[i32],
    exclude_segment_ids: &[i32],
) -> AppResult<u64> {
    let gdpr_only = apps::Entity::find_by_id(brand_id)
        .one(db)
        .await?
        .and_then(|a| a.gdpr_only)
        .unwrap_or(0)
        == 1;
    count_recipients(
        db,
        list_ids,
        segment_ids,
        exclude_list_ids,
        exclude_segment_ids,
        gdpr_only,
    )
    .await
}

fn ids_to_csv(ids: &[i32], empty_fallback: &str) -> String {
    if ids.is_empty() {
        return empty_fallback.to_string();
    }
    ids.iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(",")
}
