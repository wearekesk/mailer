//! Housekeeping operations — bulk subscriber deletions, cleanup, and deduplication.

use std::collections::{HashMap, HashSet};

use sea_orm::{
    ColumnTrait, DatabaseBackend, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    Statement,
};

use crate::database::entities::{campaigns, links, lists, subscribers};
use crate::utils::errors::AppResult;

// ── Activity-based purges ───────────────────────────────────────────────────

/// Delete subscribers on `list_id` whose `timestamp` is older than
/// `days_inactive`. Returns the number of rows removed.
pub async fn purge_inactive(
    db: &DatabaseConnection,
    list_id: i32,
    days_inactive: i64,
) -> AppResult<u64> {
    let cutoff = chrono::Utc::now().timestamp() - days_inactive.max(0) * 86_400;
    let result = subscribers::Entity::delete_many()
        .filter(subscribers::Column::List.eq(list_id))
        .filter(subscribers::Column::Timestamp.lt(cutoff))
        .filter(subscribers::Column::Confirmed.eq(1i16))
        .filter(subscribers::Column::Unsubscribed.eq(0i16))
        .filter(subscribers::Column::Bounced.eq(0i16))
        .filter(subscribers::Column::Complaint.eq(0i16))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

/// Delete subscribers on `list_id` who never confirmed and signed up more
/// than `days_unconfirmed` days ago.
pub async fn purge_unconfirmed(
    db: &DatabaseConnection,
    list_id: i32,
    days_unconfirmed: i64,
) -> AppResult<u64> {
    let cutoff = chrono::Utc::now().timestamp() - days_unconfirmed.max(0) * 86_400;
    let result = subscribers::Entity::delete_many()
        .filter(subscribers::Column::List.eq(list_id))
        .filter(subscribers::Column::Confirmed.eq(0i16))
        .filter(subscribers::Column::Timestamp.lt(cutoff))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

/// Delete subscribers on `list_id` who appear in NO `campaigns.opens` blob
/// for any campaign sent in the last `days_threshold` days.
pub async fn purge_no_opens(
    db: &DatabaseConnection,
    list_id: i32,
    days_threshold: i64,
) -> AppResult<u64> {
    let cutoff = chrono::Utc::now().timestamp() - days_threshold.max(0) * 86_400;

    let recent_campaigns = campaigns_sent_since(db, cutoff).await?;
    if recent_campaigns.is_empty() {
        return Ok(0);
    }

    let mut openers: HashSet<i32> = HashSet::new();
    for c in &recent_campaigns {
        for entry in c.opens.as_deref().unwrap_or("").split(',') {
            if let Some(sid_str) = entry.split(':').next() {
                if let Ok(sid) = sid_str.trim().parse::<i32>() {
                    openers.insert(sid);
                }
            }
        }
    }

    delete_list_subscribers_excluding(db, list_id, &openers).await
}

/// Delete subscribers on `list_id` who appear in NO `links.clicks` blob for
/// any campaign sent in the last `days_threshold` days.
pub async fn purge_no_clicks(
    db: &DatabaseConnection,
    list_id: i32,
    days_threshold: i64,
) -> AppResult<u64> {
    let cutoff = chrono::Utc::now().timestamp() - days_threshold.max(0) * 86_400;

    let recent_campaigns = campaigns_sent_since(db, cutoff).await?;
    if recent_campaigns.is_empty() {
        return Ok(0);
    }

    let campaign_ids: Vec<i32> = recent_campaigns.iter().map(|c| c.id).collect();
    let link_rows = links::Entity::find()
        .filter(links::Column::CampaignId.is_in(campaign_ids))
        .all(db)
        .await?;

    let mut clickers: HashSet<i32> = HashSet::new();
    for link in &link_rows {
        for sid_str in link.clicks.as_deref().unwrap_or("").split(',') {
            if let Ok(sid) = sid_str.trim().parse::<i32>() {
                clickers.insert(sid);
            }
        }
    }

    delete_list_subscribers_excluding(db, list_id, &clickers).await
}

async fn campaigns_sent_since(
    db: &DatabaseConnection,
    cutoff: i64,
) -> AppResult<Vec<campaigns::Model>> {
    // `campaigns.sent` is stored as a stringified epoch — use raw SQL to
    // cast and filter.
    let stmt = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        r#"SELECT * FROM campaigns
           WHERE sent IS NOT NULL
             AND sent <> ''
             AND CAST(sent AS BIGINT) >= $1"#,
        [cutoff.into()],
    );
    let rows = campaigns::Entity::find().from_raw_sql(stmt).all(db).await?;
    Ok(rows)
}

async fn delete_list_subscribers_excluding(
    db: &DatabaseConnection,
    list_id: i32,
    keep: &HashSet<i32>,
) -> AppResult<u64> {
    if keep.is_empty() {
        let result = subscribers::Entity::delete_many()
            .filter(subscribers::Column::List.eq(list_id))
            .filter(subscribers::Column::Confirmed.eq(1i16))
            .filter(subscribers::Column::Unsubscribed.eq(0i16))
            .filter(subscribers::Column::Bounced.eq(0i16))
            .filter(subscribers::Column::Complaint.eq(0i16))
            .exec(db)
            .await?;
        return Ok(result.rows_affected);
    }
    let result = subscribers::Entity::delete_many()
        .filter(subscribers::Column::List.eq(list_id))
        .filter(subscribers::Column::Confirmed.eq(1i16))
        .filter(subscribers::Column::Unsubscribed.eq(0i16))
        .filter(subscribers::Column::Bounced.eq(0i16))
        .filter(subscribers::Column::Complaint.eq(0i16))
        .filter(subscribers::Column::Id.is_not_in(keep.iter().copied().collect::<Vec<_>>()))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

// ── Dedup ───────────────────────────────────────────────────────────────────

/// Within a list, collapse duplicate subscriber rows (same email) keeping
/// the lowest `id`. Returns the number of deleted rows.
pub async fn dedupe_list(db: &DatabaseConnection, list_id: i32) -> AppResult<u64> {
    let rows = subscribers::Entity::find()
        .filter(subscribers::Column::List.eq(list_id))
        .order_by_asc(subscribers::Column::Id)
        .all(db)
        .await?;

    let mut seen: HashSet<String> = HashSet::new();
    let mut to_delete: Vec<i32> = Vec::new();
    for s in rows {
        let key = s.email.unwrap_or_default().to_ascii_lowercase();
        if key.is_empty() {
            continue;
        }
        if !seen.insert(key) {
            to_delete.push(s.id);
        }
    }
    if to_delete.is_empty() {
        return Ok(0);
    }
    let result = subscribers::Entity::delete_many()
        .filter(subscribers::Column::Id.is_in(to_delete))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

// ── Detect conflicts ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ConflictRow {
    pub email: String,
    pub list_count: i64,
    pub list_names: Vec<String>,
}

/// Find emails that appear in more than one list belonging to `brand_id`.
/// Returned rows are sorted by `list_count` desc, capped at 200 rows.
pub async fn detect_conflicts(
    db: &DatabaseConnection,
    brand_id: i32,
) -> AppResult<Vec<ConflictRow>> {
    let list_ids: Vec<i32> = lists::Entity::find()
        .filter(lists::Column::App.eq(brand_id))
        .all(db)
        .await?
        .into_iter()
        .map(|l| l.id)
        .collect();
    if list_ids.is_empty() {
        return Ok(Vec::new());
    }

    let list_name_map: HashMap<i32, String> = lists::Entity::find()
        .filter(lists::Column::App.eq(brand_id))
        .all(db)
        .await?
        .into_iter()
        .map(|l| (l.id, l.name.unwrap_or_default()))
        .collect();

    let rows = subscribers::Entity::find()
        .filter(subscribers::Column::List.is_in(list_ids.iter().copied()))
        .all(db)
        .await?;

    // email -> Vec<list_id>
    let mut by_email: HashMap<String, Vec<i32>> = HashMap::new();
    for s in rows {
        let key = s.email.unwrap_or_default().to_ascii_lowercase();
        if key.is_empty() {
            continue;
        }
        by_email.entry(key).or_default().push(s.list.unwrap_or_default());
    }

    let mut out: Vec<ConflictRow> = by_email
        .into_iter()
        .filter(|(_, lists)| lists.len() > 1)
        .map(|(email, lists)| {
            let mut unique: Vec<i32> = {
                let s: HashSet<i32> = lists.into_iter().collect();
                s.into_iter().collect()
            };
            unique.sort();
            let names: Vec<String> = unique
                .iter()
                .map(|id| {
                    list_name_map
                        .get(id)
                        .cloned()
                        .unwrap_or_else(|| format!("list#{id}"))
                })
                .collect();
            ConflictRow {
                email,
                list_count: unique.len() as i64,
                list_names: names,
            }
        })
        .collect();

    out.sort_by(|a, b| b.list_count.cmp(&a.list_count).then(a.email.cmp(&b.email)));
    out.truncate(200);
    Ok(out)
}
