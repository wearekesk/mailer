use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter,
};

use crate::database::entities::{blocked_domains, lists, subscribers, suppression_list};
use crate::utils::errors::{AppError, AppResult};

// ── Parameter types ───────────────────────────────────────────────────────────

/// Parameters required to subscribe an email address to a mailing list.
pub struct SubscribeParams {
    pub email: String,
    pub name: Option<String>,
    pub list_id: i32,
    pub ip: Option<String>,
    pub country: Option<String>,
    pub referrer: Option<String>,
    pub notes: Option<String>,
    pub gdpr: bool,
    /// When true the caller has asked for single opt-in regardless of the
    /// list's `opt_in` setting.
    pub silent: bool,
}

/// Outcome of a `subscribe` call.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubscribeOutcome {
    Subscribed,
    AlreadySubscribed,
    ConfirmationSent,
    Suppressed,
    Bounced,
    InvalidList,
}

/// Typed subscription status for an email/list pair.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Subscribed,
    Unsubscribed,
    Unconfirmed,
    Bounced,
    Complained,
    SoftBounced,
    NotInList,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Subscribe an email address to a list.
///
/// The handler is intentionally synchronous from the caller's perspective —
/// confirmation/thank-you emails are dispatched via the mailer service after
/// the database write completes.
pub async fn subscribe(
    db: &DatabaseConnection,
    params: SubscribeParams,
) -> AppResult<SubscribeOutcome> {
    let SubscribeParams {
        email,
        name,
        list_id,
        ip,
        country,
        referrer,
        notes,
        gdpr,
        silent,
    } = params;

    // Backfill country from IP via the GeoIP database if the caller didn't supply one.
    let country = match (country, ip.as_deref()) {
        (Some(c), _) if !c.is_empty() => Some(c),
        (_, Some(ip)) => crate::services::geo::country_for_ip(ip),
        _ => None,
    };

    let email = email.trim().to_lowercase();

    let list = match lists::Entity::find_by_id(list_id).one(db).await? {
        Some(l) => l,
        None => return Ok(SubscribeOutcome::InvalidList),
    };

    let app_id = list.app.unwrap_or_default();
    let opt_in = !silent && list.opt_in.unwrap_or(0) == 1;
    let user_id = list.user_id.unwrap_or_default();

    if is_email_bounced_globally(db, &email).await? {
        return Ok(SubscribeOutcome::Bounced);
    }

    let now = chrono::Utc::now().timestamp();

    let existing = subscribers::Entity::find()
        .filter(subscribers::Column::Email.eq(&email))
        .filter(subscribers::Column::List.eq(list_id))
        .one(db)
        .await?;

    if let Some(row) = existing {
        let was_unsubscribed = row.unsubscribed.unwrap_or(0) == 1;
        let was_confirmed = row.confirmed.unwrap_or(0) == 1;
        let already_subscribed = !was_unsubscribed && was_confirmed;

        let confirmed_after = if opt_in {
            if was_unsubscribed && was_confirmed {
                0i16
            } else {
                row.confirmed.unwrap_or(0)
            }
        } else {
            1
        };

        let mut active: subscribers::ActiveModel = row.into();
        active.unsubscribed = Set(Some(0));
        active.last_campaign = Set(None);
        active.timestamp = Set(Some(now));
        active.confirmed = Set(Some(confirmed_after));
        if let Some(n) = name.clone() {
            active.name = Set(Some(n));
        }
        active.gdpr = Set(Some(if gdpr { 1 } else { 0 }));
        if let Some(n) = notes.clone() {
            active.notes = Set(Some(n));
        }
        if let Some(ip) = ip.clone() {
            active.ip = Set(Some(ip));
        }
        if let Some(c) = country.clone() {
            active.country = Set(Some(c));
        }
        if let Some(r) = referrer.clone() {
            active.referrer = Set(Some(r));
        }
        active.update(db).await?;

        if already_subscribed {
            return Ok(SubscribeOutcome::AlreadySubscribed);
        }
        return Ok(if opt_in && confirmed_after != 1 {
            SubscribeOutcome::ConfirmationSent
        } else {
            SubscribeOutcome::Subscribed
        });
    }

    if is_email_suppressed(db, app_id, &email).await? {
        return Ok(SubscribeOutcome::Suppressed);
    }

    let added_via = 2i16; // 2 = API
    let confirmed = if opt_in { 0i16 } else { 1i16 };
    let method = if opt_in { 2i16 } else { 1i16 };
    let join_date = if opt_in { None } else { Some((now / 60) * 60) };

    let new_row = subscribers::ActiveModel {
        user_id: Set(Some(user_id)),
        email: Set(Some(email.clone())),
        name: Set(name),
        list: Set(Some(list_id)),
        timestamp: Set(Some(now)),
        join_date: Set(join_date),
        confirmed: Set(Some(confirmed)),
        method: Set(Some(method)),
        added_via: Set(Some(added_via)),
        ip: Set(ip),
        country: Set(country),
        referrer: Set(referrer),
        gdpr: Set(Some(if gdpr { 1 } else { 0 })),
        notes: Set(notes),
        ..Default::default()
    };
    let inserted = new_row.insert(db).await?;
    let app_url = std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000".into());

    if opt_in {
        // Double opt-in: send confirmation email.
        let _ = crate::services::list_emails::send_confirmation(db, &inserted, &list, &app_url).await;
    } else {
        // Single opt-in: fire rule + thank-you (if list has it enabled).
        let _ = crate::services::rules::dispatch_subscriber_event(db, "subscribe", &inserted).await;
        let _ = crate::services::list_emails::send_thank_you(db, &inserted, &list, &app_url).await;
    }

    Ok(if opt_in {
        SubscribeOutcome::ConfirmationSent
    } else {
        SubscribeOutcome::Subscribed
    })
}

/// Mark a subscriber as unsubscribed on a specific list.
pub async fn unsubscribe(db: &DatabaseConnection, email: &str, list_id: i32) -> AppResult<()> {
    let email = email.trim().to_lowercase();
    let row = subscribers::Entity::find()
        .filter(subscribers::Column::Email.eq(&email))
        .filter(subscribers::Column::List.eq(list_id))
        .one(db)
        .await?;

    let row = match row {
        Some(r) => r,
        None => return Err(AppError::NotFound("Subscriber not found".to_string())),
    };

    let now = chrono::Utc::now().timestamp();
    let mut active: subscribers::ActiveModel = row.into();
    active.unsubscribed = Set(Some(1));
    active.timestamp = Set(Some(now));
    let updated = active.update(db).await?;
    let _ = crate::services::rules::dispatch_subscriber_event(db, "unsubscribe", &updated).await;

    // Fire goodbye email if the list has it enabled.
    if let Some(list_row) = lists::Entity::find_by_id(list_id).one(db).await? {
        let app_url = std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000".into());
        let _ = crate::services::list_emails::send_goodbye(db, &updated, &list_row, &app_url).await;
    }
    Ok(())
}

/// Unsubscribe `email` from every list owned by the same brand.
pub async fn unsubscribe_all_in_brand(
    db: &DatabaseConnection,
    email: &str,
    list_id: i32,
) -> AppResult<u64> {
    let email = email.trim().to_lowercase();
    let now = chrono::Utc::now().timestamp();

    let list = match lists::Entity::find_by_id(list_id).one(db).await? {
        Some(l) => l,
        None => return Ok(0),
    };
    let Some(app_id) = list.app else { return Ok(0) };

    let sibling_lists = lists::Entity::find()
        .filter(lists::Column::App.eq(app_id))
        .all(db)
        .await?;
    let list_ids: Vec<i32> = sibling_lists.iter().map(|l| l.id).collect();

    let result = subscribers::Entity::update_many()
        .col_expr(
            subscribers::Column::Unsubscribed,
            sea_orm::sea_query::Expr::value(1i16),
        )
        .col_expr(
            subscribers::Column::Timestamp,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(subscribers::Column::Email.eq(&email))
        .filter(subscribers::Column::List.is_in(list_ids))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

/// Confirm a subscription (double-opt-in completion). Returns `true` if the
/// row was updated, `false` if it was already confirmed or no row existed.
pub async fn confirm_subscription(
    db: &DatabaseConnection,
    subscriber_id: i32,
    list_id: i32,
) -> AppResult<bool> {
    let row = subscribers::Entity::find_by_id(subscriber_id).one(db).await?;
    let Some(row) = row else { return Ok(false) };
    if row.list != Some(list_id) {
        return Ok(false);
    }
    if row.confirmed.unwrap_or(0) == 1 {
        return Ok(false);
    }

    let now = chrono::Utc::now().timestamp();
    let mut active: subscribers::ActiveModel = row.into();
    active.confirmed = Set(Some(1));
    active.timestamp = Set(Some(now));
    if active.join_date.as_ref() == &None {
        active.join_date = Set(Some((now / 60) * 60));
    }
    let updated = active.update(db).await?;
    let _ = crate::services::rules::dispatch_subscriber_event(db, "subscribe", &updated).await;

    // Fire thank-you email if the list has it enabled.
    if let Some(list_row) = lists::Entity::find_by_id(list_id).one(db).await? {
        let app_url = std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000".into());
        let _ = crate::services::list_emails::send_thank_you(db, &updated, &list_row, &app_url).await;
    }
    Ok(true)
}

/// Mark every subscriber row matching `email` as hard-bounced.
pub async fn mark_bounced(db: &DatabaseConnection, email: &str) -> AppResult<u64> {
    let email = email.trim().to_lowercase();
    let now = chrono::Utc::now().timestamp();
    let result = subscribers::Entity::update_many()
        .col_expr(
            subscribers::Column::Bounced,
            sea_orm::sea_query::Expr::value(1i16),
        )
        .col_expr(
            subscribers::Column::Timestamp,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(subscribers::Column::Email.eq(&email))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

/// Increment `bounce_soft` for every row matching `email`. When the counter
/// reaches `hard_bounce_threshold` (default: 3) the rows are promoted to
/// hard-bounce.
pub async fn record_soft_bounce(
    db: &DatabaseConnection,
    email: &str,
    hard_bounce_threshold: i32,
) -> AppResult<()> {
    let email = email.trim().to_lowercase();

    subscribers::Entity::update_many()
        .col_expr(
            subscribers::Column::BounceSoft,
            sea_orm::sea_query::Expr::col(subscribers::Column::BounceSoft).add(1),
        )
        .filter(subscribers::Column::Email.eq(&email))
        .exec(db)
        .await?;

    let highest_soft = subscribers::Entity::find()
        .filter(subscribers::Column::Email.eq(&email))
        .one(db)
        .await?
        .and_then(|s| s.bounce_soft)
        .unwrap_or(0);

    if highest_soft >= hard_bounce_threshold {
        mark_bounced(db, &email).await?;
    }
    Ok(())
}

/// Mark every subscriber row matching `email` (within the lists owned by the
/// brand identified by `from_email`) as a complaint.
pub async fn mark_complaint(db: &DatabaseConnection, email: &str) -> AppResult<u64> {
    let email = email.trim().to_lowercase();
    let now = chrono::Utc::now().timestamp();
    let result = subscribers::Entity::update_many()
        .col_expr(
            subscribers::Column::Complaint,
            sea_orm::sea_query::Expr::value(1i16),
        )
        .col_expr(
            subscribers::Column::Unsubscribed,
            sea_orm::sea_query::Expr::value(0i16),
        )
        .col_expr(
            subscribers::Column::Bounced,
            sea_orm::sea_query::Expr::value(0i16),
        )
        .col_expr(
            subscribers::Column::Timestamp,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(subscribers::Column::Email.eq(&email))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

/// Return the typed subscription status for a given email/list pair.
pub async fn get_status(
    db: &DatabaseConnection,
    email: &str,
    list_id: i32,
) -> AppResult<SubscriptionStatus> {
    let email = email.trim().to_lowercase();
    let row = subscribers::Entity::find()
        .filter(subscribers::Column::Email.eq(&email))
        .filter(subscribers::Column::List.eq(list_id))
        .one(db)
        .await?;

    let Some(s) = row else {
        return Ok(SubscriptionStatus::NotInList);
    };

    let confirmed = s.confirmed.unwrap_or(0) == 1;
    let unsubscribed = s.unsubscribed.unwrap_or(0) == 1;
    let bounced = s.bounced.unwrap_or(0) == 1;
    let complaint = s.complaint.unwrap_or(0) == 1;
    let bounce_soft = s.bounce_soft.unwrap_or(0);

    Ok(if confirmed && !unsubscribed && !bounced && !complaint && bounce_soft == 0 {
        SubscriptionStatus::Subscribed
    } else if !confirmed {
        SubscriptionStatus::Unconfirmed
    } else if unsubscribed {
        SubscriptionStatus::Unsubscribed
    } else if bounced {
        SubscriptionStatus::Bounced
    } else if bounce_soft > 0 {
        SubscriptionStatus::SoftBounced
    } else if complaint {
        SubscriptionStatus::Complained
    } else {
        SubscriptionStatus::Subscribed
    })
}

/// Return the count of active (confirmed, non-bounced, non-unsubscribed) subscribers.
pub async fn active_count(db: &DatabaseConnection, list_id: i32) -> AppResult<i64> {
    use sea_orm::prelude::*;

    let count = subscribers::Entity::find()
        .filter(subscribers::Column::List.eq(list_id))
        .filter(subscribers::Column::Unsubscribed.eq(0i16))
        .filter(subscribers::Column::Bounced.eq(0i16))
        .filter(subscribers::Column::Complaint.eq(0i16))
        .filter(subscribers::Column::Confirmed.eq(1i16))
        .count(db)
        .await?;

    Ok(count as i64)
}

/// Paginated subscriber row.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SubscriberRow {
    pub id: i32,
    pub email: String,
    pub name: Option<String>,
    pub status: SubscriptionStatus,
    pub timestamp: i64,
}

/// Status filter for the paginated subscribers view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusFilter {
    #[default]
    All,
    Active,
    Unconfirmed,
    Unsubscribed,
    Bounced,
    Complained,
}

impl StatusFilter {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "active" | "subscribed" => Self::Active,
            "unconfirmed" => Self::Unconfirmed,
            "unsubscribed" => Self::Unsubscribed,
            "bounced" => Self::Bounced,
            "complained" | "spam" => Self::Complained,
            _ => Self::All,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Active => "active",
            Self::Unconfirmed => "unconfirmed",
            Self::Unsubscribed => "unsubscribed",
            Self::Bounced => "bounced",
            Self::Complained => "complained",
        }
    }
}

/// Page of subscribers for a list, ordered by id desc. `status` and `q`
/// are optional filters: status restricts by subscription state, `q` is
/// a case-insensitive substring match on email + name.
pub async fn paginated(
    db: &DatabaseConnection,
    list_id: i32,
    page: u64,
    per_page: u64,
    status: StatusFilter,
    q: Option<&str>,
) -> AppResult<(Vec<SubscriberRow>, u64)> {
    use sea_orm::sea_query::Condition;
    use sea_orm::QueryOrder;
    let mut query =
        subscribers::Entity::find().filter(subscribers::Column::List.eq(list_id));
    query = match status {
        StatusFilter::All => query,
        StatusFilter::Active => query
            .filter(subscribers::Column::Confirmed.eq(1i16))
            .filter(subscribers::Column::Unsubscribed.ne(1i16))
            .filter(subscribers::Column::Bounced.ne(1i16))
            .filter(subscribers::Column::Complaint.ne(1i16)),
        StatusFilter::Unconfirmed => query.filter(subscribers::Column::Confirmed.ne(1i16)),
        StatusFilter::Unsubscribed => query.filter(subscribers::Column::Unsubscribed.eq(1i16)),
        StatusFilter::Bounced => query.filter(subscribers::Column::Bounced.eq(1i16)),
        StatusFilter::Complained => query.filter(subscribers::Column::Complaint.eq(1i16)),
    };
    if let Some(needle) = q.map(str::trim).filter(|s| !s.is_empty()) {
        let pat = format!("%{}%", needle.to_lowercase());
        query = query.filter(
            Condition::any()
                .add(subscribers::Column::Email.contains(&pat))
                .add(subscribers::Column::Name.contains(&pat)),
        );
    }
    let paginator = query
        .order_by_desc(subscribers::Column::Id)
        .paginate(db, per_page);
    let total = paginator.num_items().await?;
    let items = paginator.fetch_page(page).await?;
    let rows = items
        .into_iter()
        .map(|s| {
            let confirmed = s.confirmed.unwrap_or(0) == 1;
            let unsubscribed = s.unsubscribed.unwrap_or(0) == 1;
            let bounced = s.bounced.unwrap_or(0) == 1;
            let complaint = s.complaint.unwrap_or(0) == 1;
            let bounce_soft = s.bounce_soft.unwrap_or(0);
            let status = if confirmed && !unsubscribed && !bounced && !complaint && bounce_soft == 0 {
                SubscriptionStatus::Subscribed
            } else if !confirmed {
                SubscriptionStatus::Unconfirmed
            } else if unsubscribed {
                SubscriptionStatus::Unsubscribed
            } else if bounced {
                SubscriptionStatus::Bounced
            } else if bounce_soft > 0 {
                SubscriptionStatus::SoftBounced
            } else if complaint {
                SubscriptionStatus::Complained
            } else {
                SubscriptionStatus::Subscribed
            };
            SubscriberRow {
                id: s.id,
                email: s.email.unwrap_or_default(),
                name: s.name,
                status,
                timestamp: s.timestamp.unwrap_or(0),
            }
        })
        .collect();
    Ok((rows, total))
}

/// Total subscribers across all lists (active only).
pub async fn total_active(db: &DatabaseConnection) -> AppResult<i64> {
    Ok(subscribers::Entity::find()
        .filter(subscribers::Column::Unsubscribed.eq(0i16))
        .filter(subscribers::Column::Bounced.eq(0i16))
        .filter(subscribers::Column::Complaint.eq(0i16))
        .filter(subscribers::Column::Confirmed.eq(1i16))
        .count(db)
        .await? as i64)
}

/// Mark a single subscriber row by id as unsubscribed.
pub async fn unsubscribe_by_id(db: &DatabaseConnection, subscriber_id: i32) -> AppResult<()> {
    use sea_orm::ActiveModelTrait;
    let row = subscribers::Entity::find_by_id(subscriber_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("subscriber {subscriber_id}")))?;
    let mut active: subscribers::ActiveModel = row.into();
    active.unsubscribed = Set(Some(1));
    active.timestamp = Set(Some(chrono::Utc::now().timestamp()));
    active.update(db).await?;
    Ok(())
}

/// Hard-delete a subscriber row by id.
pub async fn delete_by_id(db: &DatabaseConnection, subscriber_id: i32) -> AppResult<()> {
    subscribers::Entity::delete_by_id(subscriber_id).exec(db).await?;
    Ok(())
}

/// Bulk-delete a vector of subscriber ids on a single list. Scoped to
/// `list_id` so a checkbox on a different list can't be smuggled in.
pub async fn bulk_delete(
    db: &DatabaseConnection,
    list_id: i32,
    ids: &[i32],
) -> AppResult<u64> {
    if ids.is_empty() {
        return Ok(0);
    }
    let result = subscribers::Entity::delete_many()
        .filter(subscribers::Column::List.eq(list_id))
        .filter(subscribers::Column::Id.is_in(ids.iter().copied()))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

/// Bulk-unsubscribe a vector of subscriber ids on a single list.
pub async fn bulk_unsubscribe(
    db: &DatabaseConnection,
    list_id: i32,
    ids: &[i32],
) -> AppResult<u64> {
    if ids.is_empty() {
        return Ok(0);
    }
    let now = chrono::Utc::now().timestamp();
    let result = subscribers::Entity::update_many()
        .col_expr(
            subscribers::Column::Unsubscribed,
            sea_orm::sea_query::Expr::value(1i16),
        )
        .col_expr(
            subscribers::Column::Timestamp,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(subscribers::Column::List.eq(list_id))
        .filter(subscribers::Column::Id.is_in(ids.iter().copied()))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

/// Apply an inline edit. `custom_field_values` mirrors the order of the
/// list's `custom_fields` packed schema and re-encodes it on save.
pub async fn update_inline(
    db: &DatabaseConnection,
    subscriber_id: i32,
    name: Option<String>,
    gdpr: bool,
    custom_field_values: &[String],
) -> AppResult<()> {
    use sea_orm::ActiveModelTrait;
    let row = subscribers::Entity::find_by_id(subscriber_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("subscriber {subscriber_id}")))?;
    let packed = custom_field_values.join("%s%");
    let mut active: subscribers::ActiveModel = row.into();
    active.name = Set(name);
    active.gdpr = Set(Some(if gdpr { 1 } else { 0 }));
    active.custom_fields = Set(Some(packed));
    active.timestamp = Set(Some(chrono::Utc::now().timestamp()));
    active.update(db).await?;
    Ok(())
}

/// CSV bulk-import action — what the CSV row should DO to the matching subscriber.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportMode {
    /// Insert the row if it doesn't already exist on the list. Skips dupes.
    Add,
    /// Update name + custom fields for an existing row; ignore rows that
    /// don't match an existing subscriber.
    Update,
    /// Permanently delete matching `(email, list)` rows.
    Delete,
    /// Mark matching rows as `unsubscribed = 1`.
    Unsubscribe,
}

impl ImportMode {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "update" => Self::Update,
            "delete" => Self::Delete,
            "unsubscribe" => Self::Unsubscribe,
            _ => Self::Add,
        }
    }
}

/// Bulk-import subscribers from a CSV iterator. CSV columns:
/// `email[,name][,custom...]`. Returns (affected, skipped) counts. The
/// `affected` semantics depend on `mode`: rows inserted (Add), updated
/// (Update), deleted (Delete), or marked unsubscribed (Unsubscribe).
pub async fn import_csv_rows<R: std::io::Read>(
    db: &DatabaseConnection,
    list_id: i32,
    user_id: i32,
    reader: &mut csv::Reader<R>,
    mode: ImportMode,
) -> AppResult<(usize, usize)> {
    use sea_orm::ActiveModelTrait;
    let now = chrono::Utc::now().timestamp();
    let mut affected = 0usize;
    let mut skipped = 0usize;

    for record in reader.records() {
        let Ok(record) = record else { skipped += 1; continue };
        let email = record.get(0).unwrap_or("").trim().to_lowercase();
        if email.is_empty() || !is_valid_email(&email) {
            skipped += 1;
            continue;
        }
        let name = record.get(1).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

        let existing = subscribers::Entity::find()
            .filter(subscribers::Column::Email.eq(&email))
            .filter(subscribers::Column::List.eq(list_id))
            .one(db)
            .await?;

        match mode {
            ImportMode::Add => {
                if existing.is_some() {
                    skipped += 1;
                    continue;
                }
                let active = subscribers::ActiveModel {
                    user_id: Set(Some(user_id)),
                    email: Set(Some(email)),
                    name: Set(name),
                    list: Set(Some(list_id)),
                    timestamp: Set(Some(now)),
                    join_date: Set(Some((now / 60) * 60)),
                    confirmed: Set(Some(1)),
                    method: Set(Some(1)),
                    added_via: Set(Some(1)),
                    ..Default::default()
                };
                active.insert(db).await?;
                affected += 1;
            }
            ImportMode::Update => {
                let Some(row) = existing else {
                    skipped += 1;
                    continue;
                };
                let mut active: subscribers::ActiveModel = row.into();
                if let Some(n) = name {
                    active.name = Set(Some(n));
                }
                active.timestamp = Set(Some(now));
                active.update(db).await?;
                affected += 1;
            }
            ImportMode::Delete => {
                if existing.is_none() {
                    skipped += 1;
                    continue;
                }
                subscribers::Entity::delete_many()
                    .filter(subscribers::Column::Email.eq(&email))
                    .filter(subscribers::Column::List.eq(list_id))
                    .exec(db)
                    .await?;
                affected += 1;
            }
            ImportMode::Unsubscribe => {
                let Some(row) = existing else {
                    skipped += 1;
                    continue;
                };
                let mut active: subscribers::ActiveModel = row.into();
                active.unsubscribed = Set(Some(1));
                active.timestamp = Set(Some(now));
                active.update(db).await?;
                affected += 1;
            }
        }
    }

    Ok((affected, skipped))
}

/// Permanently delete a subscriber from a list (GDPR right-to-erasure).
pub async fn delete(db: &DatabaseConnection, email: &str, list_id: i32) -> AppResult<()> {
    let email = email.trim().to_lowercase();
    subscribers::Entity::delete_many()
        .filter(subscribers::Column::Email.eq(&email))
        .filter(subscribers::Column::List.eq(list_id))
        .exec(db)
        .await?;
    Ok(())
}

/// Check whether a subscriber row with this email is marked bounced anywhere.
async fn is_email_bounced_globally(db: &DatabaseConnection, email: &str) -> AppResult<bool> {
    let row = subscribers::Entity::find()
        .filter(subscribers::Column::Email.eq(email))
        .filter(subscribers::Column::Bounced.eq(1i16))
        .one(db)
        .await?;
    Ok(row.is_some())
}

/// Returns true when `email` is on the suppression list for `app_id` or its
/// domain is on the blocked-domains list. When suppressed the row's
/// `block_attempts` counter is incremented.
async fn is_email_suppressed(
    db: &DatabaseConnection,
    app_id: i32,
    email: &str,
) -> AppResult<bool> {
    let now = chrono::Utc::now().timestamp();
    let domain = email.split('@').nth(1).unwrap_or("").to_lowercase();

    if let Some(row) = suppression_list::Entity::find()
        .filter(suppression_list::Column::Email.eq(email))
        .filter(suppression_list::Column::App.eq(app_id))
        .one(db)
        .await?
    {
        let next_attempts = row.block_attempts.unwrap_or(0) + 1;
        let mut active: suppression_list::ActiveModel = row.into();
        active.block_attempts = Set(Some(next_attempts));
        active.timestamp = Set(Some(now));
        active.update(db).await?;
        return Ok(true);
    }

    if !domain.is_empty() {
        if let Some(row) = blocked_domains::Entity::find()
            .filter(blocked_domains::Column::Domain.eq(&domain))
            .filter(blocked_domains::Column::App.eq(app_id))
            .one(db)
            .await?
        {
            let next_attempts = row.block_attempts.unwrap_or(0) + 1;
            let mut active: blocked_domains::ActiveModel = row.into();
            active.block_attempts = Set(Some(next_attempts));
            active.timestamp = Set(Some(now));
            active.update(db).await?;
            return Ok(true);
        }
    }
    Ok(false)
}

/// Fast email syntax validator to weed out empty or obvious-typo addresses.
/// Strict RFC-compliant validation is intentionally handled by the SMTP server at send time.
pub fn is_valid_email(email: &str) -> bool {
    let email = email.trim();
    if email.is_empty() || email.len() > 254 {
        return false;
    }
    let mut parts = email.splitn(2, '@');
    let local = parts.next().unwrap_or("");
    let domain = parts.next().unwrap_or("");
    if local.is_empty() || domain.is_empty() {
        return false;
    }
    if !domain.contains('.') {
        return false;
    }
    if email.contains(' ') {
        return false;
    }
    true
}
