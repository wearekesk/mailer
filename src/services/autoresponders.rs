//! Autoresponders service.

use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder,
};

use crate::database::entities::{ares, ares_emails, lists};
use crate::utils::errors::{AppError, AppResult};

pub async fn list_for_brand(
    db: &DatabaseConnection,
    brand_id: i32,
) -> AppResult<Vec<ares::Model>> {
    // ares is keyed by list, lists are keyed by app — join via list ids.
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
    Ok(ares::Entity::find()
        .filter(ares::Column::List.is_in(list_ids))
        .order_by_asc(ares::Column::Id)
        .all(db)
        .await?)
}

/// Every autoresponder attached to a single list.
pub async fn list_for_list(
    db: &DatabaseConnection,
    list_id: i32,
) -> AppResult<Vec<ares::Model>> {
    Ok(ares::Entity::find()
        .filter(ares::Column::List.eq(list_id))
        .order_by_asc(ares::Column::Id)
        .all(db)
        .await?)
}

pub async fn find(db: &DatabaseConnection, id: i32) -> AppResult<Option<ares::Model>> {
    Ok(ares::Entity::find_by_id(id).one(db).await?)
}

#[derive(Debug, Clone, Default)]
pub struct AutoresponderInput {
    pub name: String,
    pub list_id: i32,
    pub kind: i32, // 0 = signup, 1 = custom field date
    pub custom_field: Option<String>,
}

pub async fn create(
    db: &DatabaseConnection,
    input: AutoresponderInput,
) -> AppResult<i32> {
    let active = ares::ActiveModel {
        name: Set(Some(input.name)),
        list: Set(Some(input.list_id)),
        r#type: Set(Some(input.kind)),
        custom_field: Set(input.custom_field),
        ..Default::default()
    };
    Ok(active.insert(db).await?.id)
}

pub async fn update_title(
    db: &DatabaseConnection,
    id: i32,
    name: &str,
) -> AppResult<()> {
    let row = ares::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("autoresponder {id}")))?;
    let mut active: ares::ActiveModel = row.into();
    active.name = Set(Some(name.to_string()));
    active.update(db).await?;
    Ok(())
}

pub async fn delete(db: &DatabaseConnection, id: i32) -> AppResult<()> {
    ares_emails::Entity::delete_many()
        .filter(ares_emails::Column::AresId.eq(id))
        .exec(db)
        .await?;
    ares::Entity::delete_by_id(id).exec(db).await?;
    Ok(())
}

// ── Emails inside an autoresponder ──────────────────────────────────────────

pub async fn list_emails(
    db: &DatabaseConnection,
    autoresponder_id: i32,
) -> AppResult<Vec<ares_emails::Model>> {
    Ok(ares_emails::Entity::find()
        .filter(ares_emails::Column::AresId.eq(autoresponder_id))
        .order_by_asc(ares_emails::Column::Id)
        .all(db)
        .await?)
}

pub async fn find_email(
    db: &DatabaseConnection,
    id: i32,
) -> AppResult<Option<ares_emails::Model>> {
    Ok(ares_emails::Entity::find_by_id(id).one(db).await?)
}

#[derive(Debug, Clone, Default)]
pub struct AutoresponderEmailInput {
    pub ares_id: i32,
    pub title: String,
    pub from_name: Option<String>,
    pub from_email: Option<String>,
    pub reply_to: Option<String>,
    pub plain_text: Option<String>,
    pub html_text: Option<String>,
    pub time_condition: Option<String>,
    pub timezone: Option<String>,
    pub opens_tracking: bool,
    pub links_tracking: bool,
    /// CSV of segment ids — only subscribers in at least one of these
    /// segments receive the email. Empty = no include filter.
    pub segs: Option<String>,
    /// CSV of segment ids — anyone in any of these segments is skipped.
    pub segs_excl: Option<String>,
}

/// Create a new autoresponder email row, disabled by default so the AR
/// owner can review the body before turning it on.
pub async fn create_email(
    db: &DatabaseConnection,
    input: AutoresponderEmailInput,
) -> AppResult<i32> {
    let now = chrono::Utc::now().timestamp();
    let active = ares_emails::ActiveModel {
        ares_id: Set(Some(input.ares_id)),
        title: Set(Some(input.title)),
        from_name: Set(input.from_name),
        from_email: Set(input.from_email),
        reply_to: Set(input.reply_to),
        plain_text: Set(input.plain_text),
        html_text: Set(input.html_text),
        time_condition: Set(input.time_condition),
        timezone: Set(input.timezone),
        opens_tracking: Set(Some(if input.opens_tracking { 1 } else { 0 })),
        links_tracking: Set(Some(if input.links_tracking { 1 } else { 0 })),
        segs: Set(input.segs),
        segs_excl: Set(input.segs_excl),
        enabled: Set(Some(0)),
        created: Set(Some(now)),
        wysiwyg: Set(Some(1)),
        ..Default::default()
    };
    Ok(active.insert(db).await?.id)
}

/// Update body, subject, schedule, and tracking flags for an existing
/// autoresponder email.
pub async fn update_email(
    db: &DatabaseConnection,
    id: i32,
    input: AutoresponderEmailInput,
) -> AppResult<()> {
    let row = ares_emails::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("autoresponder email {id}")))?;
    let mut active: ares_emails::ActiveModel = row.into();
    active.title = Set(Some(input.title));
    active.from_name = Set(input.from_name);
    active.from_email = Set(input.from_email);
    active.reply_to = Set(input.reply_to);
    active.plain_text = Set(input.plain_text);
    active.html_text = Set(input.html_text);
    active.time_condition = Set(input.time_condition);
    active.timezone = Set(input.timezone);
    active.opens_tracking = Set(Some(if input.opens_tracking { 1 } else { 0 }));
    active.links_tracking = Set(Some(if input.links_tracking { 1 } else { 0 }));
    active.segs = Set(input.segs);
    active.segs_excl = Set(input.segs_excl);
    active.update(db).await?;
    Ok(())
}

/// Toggle the `enabled` flag on an autoresponder email; returns the new
/// state.
pub async fn toggle_email(db: &DatabaseConnection, id: i32) -> AppResult<bool> {
    let row = ares_emails::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("autoresponder email {id}")))?;
    let new_value: i16 = if row.enabled.unwrap_or(0) == 1 { 0 } else { 1 };
    let mut active: ares_emails::ActiveModel = row.into();
    active.enabled = Set(Some(new_value));
    active.update(db).await?;
    Ok(new_value == 1)
}

pub async fn delete_email(db: &DatabaseConnection, id: i32) -> AppResult<()> {
    ares_emails::Entity::delete_by_id(id).exec(db).await?;
    Ok(())
}

/// Clone an autoresponder email, returning the new id. The clone keeps
/// the same body / schedule / tracking flags but resets `recipients` and
/// `opens` and starts disabled, so the AR owner can review the copy
/// before enabling it.
pub async fn duplicate_email(db: &DatabaseConnection, id: i32) -> AppResult<i32> {
    let row = ares_emails::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("autoresponder email {id}")))?;
    let now = chrono::Utc::now().timestamp();
    let cloned = ares_emails::ActiveModel {
        ares_id: Set(row.ares_id),
        from_name: Set(row.from_name),
        from_email: Set(row.from_email),
        reply_to: Set(row.reply_to),
        title: Set(row.title.map(|t| format!("{t} (copy)"))),
        plain_text: Set(row.plain_text),
        html_text: Set(row.html_text),
        query_string: Set(row.query_string),
        time_condition: Set(row.time_condition),
        timezone: Set(row.timezone),
        created: Set(Some(now)),
        recipients: Set(Some(0)),
        opens: Set(None),
        wysiwyg: Set(row.wysiwyg),
        opens_tracking: Set(row.opens_tracking),
        links_tracking: Set(row.links_tracking),
        enabled: Set(Some(0)),
        segs: Set(row.segs),
        segs_excl: Set(row.segs_excl),
        ..Default::default()
    };
    Ok(cloned.insert(db).await?.id)
}

/// Every enabled autoresponder email across the install. Scheduler walks
/// these once per tick.
pub async fn enabled_emails(db: &DatabaseConnection) -> AppResult<Vec<ares_emails::Model>> {
    Ok(ares_emails::Entity::find()
        .filter(ares_emails::Column::Enabled.eq(1i16))
        .order_by_asc(ares_emails::Column::Id)
        .all(db)
        .await?)
}

/// Parse `time_condition` strings like `"3 days"`, `"2 hour"`, `"30 minutes"`,
/// `"1 week"`. Returns the offset in seconds, or `None` if the string is
/// unparseable. Empty string parses to 0 (send immediately on signup).
pub fn parse_time_condition(raw: &str) -> Option<i64> {
    let lower = raw.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return Some(0);
    }
    let mut parts = lower.split_whitespace();
    let n: i64 = parts.next()?.parse().ok()?;
    let unit = parts.next().unwrap_or("days");
    let secs = match unit.trim_end_matches('s') {
        "second" | "sec" => 1,
        "minute" | "min" => 60,
        "hour" | "hr" => 3_600,
        "day" => 86_400,
        "week" => 7 * 86_400,
        "month" => 30 * 86_400,
        "year" => 365 * 86_400,
        _ => return None,
    };
    Some(n * secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_time_conditions() {
        assert_eq!(parse_time_condition(""), Some(0));
        assert_eq!(parse_time_condition("3 days"), Some(3 * 86_400));
        assert_eq!(parse_time_condition("2 hour"), Some(2 * 3_600));
        assert_eq!(parse_time_condition("30 minutes"), Some(30 * 60));
        assert_eq!(parse_time_condition("1 week"), Some(7 * 86_400));
        assert_eq!(parse_time_condition("garbage"), None);
    }
}
