//! List CRUD and custom-field management helpers.
//!
//! `lists.custom_fields` is stored in a packed format: `Field1:Type%s%Field2:Type%s%`
//! where `Type` is `Text` or `Date`.

use sea_orm::{
    sea_query::Expr, ActiveModelTrait, ActiveValue::Set, ColumnTrait, Condition,
    DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};

use crate::database::entities::{lists, subscribers};
use crate::utils::errors::{AppError, AppResult};

// ── DTOs ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct ListSummary {
    pub id: i32,
    pub name: String,
    pub subscriber_count: i64,
    pub hide: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ListInput {
    pub name: String,
    pub opt_in: i16,
    pub subscribed_url: Option<String>,
    pub confirm_url: Option<String>,
    pub already_subscribed_url: Option<String>,
    pub no_consent_url: Option<String>,
    pub reconsent_success_url: Option<String>,
    pub thankyou: i16,
    pub thankyou_subject: Option<String>,
    pub thankyou_message: Option<String>,
    pub unsubscribe_all_list: i16,
    pub unsubscribe_confirm: i16,
    pub unsubscribed_url: Option<String>,
    pub goodbye: i16,
    pub goodbye_subject: Option<String>,
    pub goodbye_message: Option<String>,
    pub confirmation_subject: Option<String>,
    pub confirmation_email: Option<String>,
    pub gdpr_enabled: i16,
    pub marketing_permission: Option<String>,
    pub what_to_expect: Option<String>,
    pub gdpr: i16,
}

/// One custom-field column on a list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomField {
    pub name: String,
    pub kind: CustomFieldType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomFieldType {
    Text,
    Date,
}

impl CustomFieldType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "Text",
            Self::Date => "Date",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "Date" | "date" => Self::Date,
            _ => Self::Text,
        }
    }
}

// ── Query API ────────────────────────────────────────────────────────────────

/// Look up one list row.
pub async fn find_list(db: &DatabaseConnection, list_id: i32) -> AppResult<Option<lists::Model>> {
    Ok(lists::Entity::find_by_id(list_id).one(db).await?)
}

/// All lists for a brand, with subscriber-count for each.
pub async fn list_for_brand(
    db: &DatabaseConnection,
    brand_id: i32,
    include_hidden: bool,
) -> AppResult<Vec<ListSummary>> {
    let mut hide_filter = Condition::any()
        .add(lists::Column::Hide.eq(0i16))
        .add(lists::Column::Hide.is_null());
    if include_hidden {
        hide_filter = hide_filter.add(lists::Column::Hide.eq(1i16));
    }
    let rows = lists::Entity::find()
        .filter(lists::Column::App.eq(brand_id))
        .filter(hide_filter)
        .order_by_asc(Expr::cust("LOWER(name)"))
        .all(db)
        .await?;

    let mut out = Vec::with_capacity(rows.len());
    for l in rows {
        let count = subscribers::Entity::find()
            .filter(subscribers::Column::List.eq(l.id))
            .filter(subscribers::Column::Unsubscribed.eq(0i16))
            .filter(subscribers::Column::Bounced.eq(0i16))
            .filter(subscribers::Column::Complaint.eq(0i16))
            .filter(subscribers::Column::Confirmed.eq(1i16))
            .count(db)
            .await? as i64;
        out.push(ListSummary {
            id: l.id,
            name: l.name.unwrap_or_default(),
            subscriber_count: count,
            hide: l.hide.unwrap_or(0) == 1,
        });
    }
    Ok(out)
}

/// Backwards-compat alias used by older route handlers + the JSON API.
pub async fn list_brand_lists(
    db: &DatabaseConnection,
    brand_id: i32,
    include_hidden: bool,
) -> AppResult<Vec<ListSummary>> {
    list_for_brand(db, brand_id, include_hidden).await
}

/// Total active list count across all brands (used by the dashboard).
pub async fn total_active_count(db: &DatabaseConnection) -> AppResult<i64> {
    Ok(lists::Entity::find()
        .filter(
            Condition::any()
                .add(lists::Column::Hide.eq(0i16))
                .add(lists::Column::Hide.is_null()),
        )
        .count(db)
        .await? as i64)
}

// ── Mutating API ─────────────────────────────────────────────────────────────

pub async fn create_list(
    db: &DatabaseConnection,
    user_id: i32,
    brand_id: i32,
    input: ListInput,
) -> AppResult<i32> {
    let active = lists::ActiveModel {
        app: Set(Some(brand_id)),
        user_id: Set(Some(user_id)),
        name: Set(Some(input.name)),
        opt_in: Set(Some(input.opt_in)),
        subscribed_url: Set(input.subscribed_url),
        confirm_url: Set(input.confirm_url),
        already_subscribed_url: Set(input.already_subscribed_url),
        no_consent_url: Set(input.no_consent_url),
        reconsent_success_url: Set(input.reconsent_success_url),
        thankyou: Set(Some(input.thankyou)),
        thankyou_subject: Set(input.thankyou_subject),
        thankyou_message: Set(input.thankyou_message),
        unsubscribe_all_list: Set(Some(input.unsubscribe_all_list)),
        unsubscribe_confirm: Set(Some(input.unsubscribe_confirm)),
        unsubscribed_url: Set(input.unsubscribed_url),
        goodbye: Set(Some(input.goodbye)),
        goodbye_subject: Set(input.goodbye_subject),
        goodbye_message: Set(input.goodbye_message),
        confirmation_subject: Set(input.confirmation_subject),
        confirmation_email: Set(input.confirmation_email),
        gdpr_enabled: Set(Some(input.gdpr_enabled)),
        marketing_permission: Set(input.marketing_permission),
        what_to_expect: Set(input.what_to_expect),
        gdpr: Set(Some(input.gdpr)),
        hide: Set(Some(0)),
        custom_fields: Set(Some(String::new())),
        prev_count: Set(Some(0)),
        currently_processing: Set(Some(0)),
        total_records: Set(Some(0)),
        ..Default::default()
    };
    Ok(active.insert(db).await?.id)
}

pub async fn update_list(
    db: &DatabaseConnection,
    list_id: i32,
    input: ListInput,
) -> AppResult<()> {
    let row = lists::Entity::find_by_id(list_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("list {list_id}")))?;
    let mut active: lists::ActiveModel = row.into();
    active.name = Set(Some(input.name));
    active.opt_in = Set(Some(input.opt_in));
    active.subscribed_url = Set(input.subscribed_url);
    active.confirm_url = Set(input.confirm_url);
    active.already_subscribed_url = Set(input.already_subscribed_url);
    active.no_consent_url = Set(input.no_consent_url);
    active.reconsent_success_url = Set(input.reconsent_success_url);
    active.thankyou = Set(Some(input.thankyou));
    active.thankyou_subject = Set(input.thankyou_subject);
    active.thankyou_message = Set(input.thankyou_message);
    active.unsubscribe_all_list = Set(Some(input.unsubscribe_all_list));
    active.unsubscribe_confirm = Set(Some(input.unsubscribe_confirm));
    active.unsubscribed_url = Set(input.unsubscribed_url);
    active.goodbye = Set(Some(input.goodbye));
    active.goodbye_subject = Set(input.goodbye_subject);
    active.goodbye_message = Set(input.goodbye_message);
    active.confirmation_subject = Set(input.confirmation_subject);
    active.confirmation_email = Set(input.confirmation_email);
    active.gdpr_enabled = Set(Some(input.gdpr_enabled));
    active.marketing_permission = Set(input.marketing_permission);
    active.what_to_expect = Set(input.what_to_expect);
    active.gdpr = Set(Some(input.gdpr));
    active.update(db).await?;
    Ok(())
}

pub async fn delete_list(db: &DatabaseConnection, list_id: i32) -> AppResult<()> {
    subscribers::Entity::delete_many()
        .filter(subscribers::Column::List.eq(list_id))
        .exec(db)
        .await?;
    lists::Entity::delete_by_id(list_id).exec(db).await?;
    Ok(())
}

pub async fn toggle_hidden(db: &DatabaseConnection, list_id: i32) -> AppResult<bool> {
    let row = lists::Entity::find_by_id(list_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("list {list_id}")))?;
    let new_value = if row.hide.unwrap_or(0) == 1 { 0 } else { 1 };
    let mut active: lists::ActiveModel = row.into();
    active.hide = Set(Some(new_value));
    active.update(db).await?;
    Ok(new_value == 1)
}

/// Bulk-set the `hide` flag on a vector of list ids — used by the brand-lists table's
/// "Hide selected" / "Show selected" bulk-action buttons.
pub async fn bulk_set_hidden(
    db: &DatabaseConnection,
    list_ids: &[i32],
    hidden: bool,
) -> AppResult<u64> {
    if list_ids.is_empty() {
        return Ok(0);
    }
    let result = lists::Entity::update_many()
        .col_expr(
            lists::Column::Hide,
            sea_orm::sea_query::Expr::value(if hidden { 1i16 } else { 0i16 }),
        )
        .filter(lists::Column::Id.is_in(list_ids.iter().copied()))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

// ── Custom fields ────────────────────────────────────────────────────────────

/// Decode a packed `custom_fields` string into a `Vec<CustomField>`.
pub fn parse_custom_fields(packed: &str) -> Vec<CustomField> {
    packed
        .split("%s%")
        .filter(|s| !s.is_empty())
        .filter_map(|entry| {
            let mut parts = entry.splitn(2, ':');
            let name = parts.next()?.trim().to_string();
            let kind = parts.next().map(CustomFieldType::parse).unwrap_or(CustomFieldType::Text);
            if name.is_empty() {
                None
            } else {
                Some(CustomField { name, kind })
            }
        })
        .collect()
}

/// Encode a `Vec<CustomField>` back into the packed format.
pub fn encode_custom_fields(fields: &[CustomField]) -> String {
    fields
        .iter()
        .map(|f| format!("{}:{}", f.name, f.kind.as_str()))
        .collect::<Vec<_>>()
        .join("%s%")
}

pub async fn add_custom_field(
    db: &DatabaseConnection,
    list_id: i32,
    name: &str,
    kind: CustomFieldType,
) -> AppResult<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("custom field name required".into()));
    }
    let lower = name.to_ascii_lowercase();
    if lower == "name" || lower == "email" {
        return Err(AppError::BadRequest("name and email are reserved fields".into()));
    }

    let row = lists::Entity::find_by_id(list_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("list {list_id}")))?;

    let mut fields = parse_custom_fields(row.custom_fields.as_deref().unwrap_or(""));
    if fields.iter().any(|f| f.name.eq_ignore_ascii_case(name)) {
        return Err(AppError::BadRequest("custom field already exists".into()));
    }
    fields.push(CustomField {
        name: name.to_string(),
        kind,
    });
    let encoded = encode_custom_fields(&fields);

    let mut active: lists::ActiveModel = row.into();
    active.custom_fields = Set(Some(encoded));
    active.update(db).await?;
    Ok(())
}

pub async fn delete_custom_field(
    db: &DatabaseConnection,
    list_id: i32,
    name: &str,
) -> AppResult<()> {
    let row = lists::Entity::find_by_id(list_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("list {list_id}")))?;

    let mut fields = parse_custom_fields(row.custom_fields.as_deref().unwrap_or(""));
    fields.retain(|f| !f.name.eq_ignore_ascii_case(name));
    let encoded = encode_custom_fields(&fields);

    let mut active: lists::ActiveModel = row.into();
    active.custom_fields = Set(Some(encoded));
    active.update(db).await?;
    Ok(())
}

pub async fn rename_custom_field(
    db: &DatabaseConnection,
    list_id: i32,
    old_name: &str,
    new_name: &str,
    kind: CustomFieldType,
) -> AppResult<()> {
    let new_trimmed = new_name.trim();
    if new_trimmed.is_empty() {
        return Err(AppError::BadRequest("new name required".into()));
    }
    let row = lists::Entity::find_by_id(list_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("list {list_id}")))?;

    let mut fields = parse_custom_fields(row.custom_fields.as_deref().unwrap_or(""));
    let mut renamed = false;
    for f in fields.iter_mut() {
        if f.name.eq_ignore_ascii_case(old_name) {
            f.name = new_trimmed.to_string();
            f.kind = kind;
            renamed = true;
        }
    }
    if !renamed {
        return Err(AppError::NotFound(format!("custom field {old_name}")));
    }

    let encoded = encode_custom_fields(&fields);
    let mut active: lists::ActiveModel = row.into();
    active.custom_fields = Set(Some(encoded));
    active.update(db).await?;
    Ok(())
}
