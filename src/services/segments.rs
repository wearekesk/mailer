//! Segments service for filtering subscribers dynamically by conditions.
//!
//! Conditions live in `seg_cons` (one row per condition) and are composed by
//! `group_id` / `operator`:
//!
//! - all conditions inside one `group_id` are AND-ed
//! - groups are OR-ed together
//!
//! Built-in subscriber columns (`email`, `name`, `country`, `ip`, `referrer`,
//! `gdpr`, `timestamp`, `join_date`, `last_campaign`, `last_ares`,
//! `added_via`) are translated into a real SQL WHERE clause. Custom fields
//! are evaluated in-memory after the SQL fetch by decoding the packed
//! `subscribers.custom_fields` blob using the list's field schema.

use std::collections::BTreeMap;

use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseBackend,
    DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Statement,
};

use crate::database::entities::{lists as lists_entity, seg, seg_cons};
use crate::services::lists::{parse_custom_fields, CustomFieldType};
use crate::utils::errors::{AppError, AppResult};

#[derive(Debug, Clone, Default)]
pub struct SegmentInput {
    pub name: String,
    pub list_id: i32,
}

#[derive(Debug, Clone, Default)]
pub struct ConditionInput {
    pub group_id: i32,
    pub operator: String, // "AND" | "OR"
    pub field: String,
    pub comparison: String,
    pub val: String,
}

// ── CRUD ────────────────────────────────────────────────────────────────────

pub async fn list_for_brand(
    db: &DatabaseConnection,
    brand_id: i32,
) -> AppResult<Vec<seg::Model>> {
    Ok(seg::Entity::find()
        .filter(seg::Column::App.eq(brand_id))
        .order_by_asc(seg::Column::Id)
        .all(db)
        .await?)
}

/// Every segment attached to a single list.
pub async fn list_for_list(
    db: &DatabaseConnection,
    list_id: i32,
) -> AppResult<Vec<seg::Model>> {
    Ok(seg::Entity::find()
        .filter(seg::Column::List.eq(list_id))
        .order_by_asc(seg::Column::Id)
        .all(db)
        .await?)
}

pub async fn find(db: &DatabaseConnection, id: i32) -> AppResult<Option<seg::Model>> {
    Ok(seg::Entity::find_by_id(id).one(db).await?)
}

pub async fn create(
    db: &DatabaseConnection,
    brand_id: i32,
    input: SegmentInput,
) -> AppResult<i32> {
    let active = seg::ActiveModel {
        name: Set(Some(input.name)),
        app: Set(Some(brand_id)),
        list: Set(Some(input.list_id)),
        last_updated: Set(Some(chrono::Utc::now().timestamp().to_string())),
        ..Default::default()
    };
    Ok(active.insert(db).await?.id)
}

pub async fn delete(db: &DatabaseConnection, id: i32) -> AppResult<()> {
    seg_cons::Entity::delete_many()
        .filter(seg_cons::Column::SegId.eq(id))
        .exec(db)
        .await?;
    seg::Entity::delete_by_id(id).exec(db).await?;
    Ok(())
}

pub async fn add_condition(
    db: &DatabaseConnection,
    seg_id: i32,
    input: ConditionInput,
) -> AppResult<i32> {
    let active = seg_cons::ActiveModel {
        seg_id: Set(Some(seg_id)),
        group_id: Set(Some(input.group_id)),
        operator: Set(Some(input.operator)),
        field: Set(Some(input.field)),
        comparison: Set(Some(input.comparison)),
        val: Set(Some(input.val)),
        ..Default::default()
    };
    Ok(active.insert(db).await?.id)
}

pub async fn list_conditions(
    db: &DatabaseConnection,
    seg_id: i32,
) -> AppResult<Vec<seg_cons::Model>> {
    Ok(seg_cons::Entity::find()
        .filter(seg_cons::Column::SegId.eq(seg_id))
        .order_by_asc(seg_cons::Column::Id)
        .all(db)
        .await?)
}

pub async fn delete_condition(db: &DatabaseConnection, id: i32) -> AppResult<()> {
    seg_cons::Entity::delete_by_id(id).exec(db).await?;
    Ok(())
}

// ── Matching ────────────────────────────────────────────────────────────────

/// Resolve subscriber ids that match `seg_id`, restricted to `list_id`.
///
/// Empty segments (no conditions) match every confirmed/active subscriber on
/// the list.
pub async fn get_matching_subscriber_ids(
    db: &DatabaseConnection,
    seg_id: i32,
    list_id: i32,
) -> AppResult<Vec<i32>> {
    let conditions = list_conditions(db, seg_id).await?;

    // Bucket by group_id.
    let mut groups: BTreeMap<i32, Vec<seg_cons::Model>> = BTreeMap::new();
    for c in conditions {
        groups
            .entry(c.group_id.unwrap_or(0))
            .or_default()
            .push(c);
    }

    let mut bind_values: Vec<sea_orm::Value> = Vec::new();
    let mut bind_idx: usize = 1;
    let mut group_sqls: Vec<String> = Vec::new();
    let mut group_custom: Vec<Vec<seg_cons::Model>> = Vec::new();

    for (_, conds) in groups {
        let mut parts = Vec::new();
        let mut customs = Vec::new();
        for c in conds {
            match translate_builtin(&c, &mut bind_idx) {
                Some((frag, vals)) => {
                    parts.push(frag);
                    bind_values.extend(vals);
                }
                None => customs.push(c),
            }
        }
        let sql = if parts.is_empty() {
            // A group made entirely of custom-field conditions has no SQL
            // contribution — every row may pass and we filter in-memory.
            "TRUE".to_string()
        } else {
            format!("({})", parts.join(" AND "))
        };
        group_sqls.push(sql);
        group_custom.push(customs);
    }

    // List filter binding.
    let list_placeholder = next_bind(&mut bind_idx);
    bind_values.push(sea_orm::Value::from(list_id));

    let group_clause = if group_sqls.is_empty() {
        String::new()
    } else {
        format!(" AND ({})", group_sqls.join(" OR "))
    };

    let need_custom = group_custom.iter().any(|g| !g.is_empty());
    let select_cols = if need_custom {
        "id, custom_fields"
    } else {
        "id, NULL::text AS custom_fields"
    };

    let sql = format!(
        "SELECT {select_cols} \
         FROM subscribers \
         WHERE list = {list_placeholder} \
           AND unsubscribed = 0 AND bounced = 0 AND complaint = 0 AND confirmed = 1\
         {group_clause} \
         ORDER BY id ASC"
    );

    let stmt = Statement::from_sql_and_values(DatabaseBackend::Postgres, &sql, bind_values);
    let rows = db.query_all(stmt).await?;

    if !need_custom {
        return Ok(rows
            .into_iter()
            .filter_map(|r| r.try_get::<i32>("", "id").ok())
            .collect());
    }

    // In-memory filter for groups with custom-field conditions.
    let list_custom_fields: String = lists_entity::Entity::find_by_id(list_id)
        .one(db)
        .await?
        .and_then(|l| l.custom_fields)
        .unwrap_or_default();
    let cf_meta = parse_custom_fields(&list_custom_fields);
    let cf_index: BTreeMap<String, (usize, CustomFieldType)> = cf_meta
        .iter()
        .enumerate()
        .map(|(i, f)| (f.name.to_ascii_lowercase(), (i, f.kind)))
        .collect();

    let mut out = Vec::new();
    for row in rows {
        let id: i32 = match row.try_get("", "id") {
            Ok(v) => v,
            Err(_) => continue,
        };
        let cf: String = row.try_get::<String>("", "custom_fields").unwrap_or_default();
        let values: Vec<&str> = cf.split("%s%").collect();

        // Row passes if ANY group's custom conditions all hold (mirrors the
        // OR-of-groups SQL combination). Groups with zero custom conditions
        // already passed the SQL filter.
        let mut any_match = false;
        for group in &group_custom {
            if group.is_empty() {
                any_match = true;
                break;
            }
            let mut all_hold = true;
            for cond in group {
                let field = cond.field.as_deref().unwrap_or("");
                let comparison = cond.comparison.as_deref().unwrap_or("");
                let target = cond.val.as_deref().unwrap_or("");
                let key = field.to_ascii_lowercase();
                let actual = cf_index
                    .get(&key)
                    .and_then(|(idx, _)| values.get(*idx).copied())
                    .unwrap_or("");
                if !text_compare(actual, comparison, target) {
                    all_hold = false;
                    break;
                }
            }
            if all_hold {
                any_match = true;
                break;
            }
        }
        if any_match {
            out.push(id);
        }
    }
    Ok(out)
}

// ── SQL translation helpers ─────────────────────────────────────────────────

fn translate_builtin(
    c: &seg_cons::Model,
    bind_idx: &mut usize,
) -> Option<(String, Vec<sea_orm::Value>)> {
    let field = c.field.as_deref().unwrap_or("").to_ascii_lowercase();
    let comparison = c.comparison.as_deref().unwrap_or("");
    let raw_val = c.val.as_deref().unwrap_or("");
    let column = builtin_column(&field)?;

    match comparison {
        "is" | "equals" | "=" => Some(eq(column, raw_val, bind_idx, false)),
        "is_not" | "not_equals" | "!=" => Some(eq(column, raw_val, bind_idx, true)),
        "contains" => Some(like(column, raw_val, bind_idx, false, false, false)),
        "not_contains" => Some(like(column, raw_val, bind_idx, false, false, true)),
        "starts_with" => Some(like(column, raw_val, bind_idx, true, false, false)),
        "ends_with" => Some(like(column, raw_val, bind_idx, false, true, false)),
        "is_empty" => Some((
            format!("({column} IS NULL OR {column} = '')"),
            Vec::new(),
        )),
        "is_not_empty" => Some((
            format!("({column} IS NOT NULL AND {column} <> '')"),
            Vec::new(),
        )),
        "greater_than" | ">" => num_compare(column, raw_val, ">", bind_idx),
        "less_than" | "<" => num_compare(column, raw_val, "<", bind_idx),
        _ => None,
    }
}

fn builtin_column(field: &str) -> Option<&'static str> {
    Some(match field {
        "email" => "email",
        "name" => "name",
        "country" => "country",
        "ip" => "ip",
        "referrer" => "referrer",
        "gdpr" => "gdpr",
        "timestamp" => "timestamp",
        "join_date" => "join_date",
        "last_campaign" => "last_campaign",
        "last_ares" => "last_ares",
        "added_via" => "added_via",
        _ => return None,
    })
}

fn next_bind(bind_idx: &mut usize) -> String {
    let s = format!("${bind_idx}");
    *bind_idx += 1;
    s
}

fn eq(
    column: &str,
    val: &str,
    bind_idx: &mut usize,
    negate: bool,
) -> (String, Vec<sea_orm::Value>) {
    let placeholder = next_bind(bind_idx);
    let op = if negate { "<>" } else { "=" };
    (
        format!("{column} {op} {placeholder}"),
        vec![sea_orm::Value::from(val.to_string())],
    )
}

fn like(
    column: &str,
    val: &str,
    bind_idx: &mut usize,
    starts: bool,
    ends: bool,
    negate: bool,
) -> (String, Vec<sea_orm::Value>) {
    let placeholder = next_bind(bind_idx);
    let pattern = match (starts, ends) {
        (true, false) => format!("{val}%"),
        (false, true) => format!("%{val}"),
        _ => format!("%{val}%"),
    };
    let op = if negate { "NOT ILIKE" } else { "ILIKE" };
    (
        format!("{column} {op} {placeholder}"),
        vec![sea_orm::Value::from(pattern)],
    )
}

fn num_compare(
    column: &str,
    val: &str,
    op: &str,
    bind_idx: &mut usize,
) -> Option<(String, Vec<sea_orm::Value>)> {
    let n: i64 = val.parse().ok()?;
    let placeholder = next_bind(bind_idx);
    Some((
        format!("{column} {op} {placeholder}"),
        vec![sea_orm::Value::from(n)],
    ))
}

fn text_compare(actual: &str, comparison: &str, target: &str) -> bool {
    let a = actual.to_ascii_lowercase();
    let t = target.to_ascii_lowercase();
    match comparison {
        "is" | "equals" | "=" => a == t,
        "is_not" | "not_equals" | "!=" => a != t,
        "contains" => a.contains(&t),
        "not_contains" => !a.contains(&t),
        "starts_with" => a.starts_with(&t),
        "ends_with" => a.ends_with(&t),
        "is_empty" => actual.is_empty(),
        "is_not_empty" => !actual.is_empty(),
        _ => false,
    }
}

#[allow(dead_code)]
fn _ensure_app_error_used(_e: AppError) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_compare_equality_case_insensitive() {
        assert!(text_compare("Foo", "is", "FOO"));
        assert!(text_compare("foo", "equals", "foo"));
        assert!(text_compare("foo", "=", "foo"));
        assert!(!text_compare("foo", "is", "bar"));
    }

    #[test]
    fn text_compare_inequality() {
        assert!(text_compare("foo", "is_not", "bar"));
        assert!(text_compare("foo", "not_equals", "bar"));
        assert!(text_compare("foo", "!=", "bar"));
        assert!(!text_compare("foo", "is_not", "foo"));
    }

    #[test]
    fn text_compare_substring_ops() {
        assert!(text_compare("Hello, World", "contains", "world"));
        assert!(!text_compare("Hello", "contains", "world"));
        assert!(text_compare("Hello", "not_contains", "world"));
        assert!(text_compare("Hello", "starts_with", "he"));
        assert!(text_compare("Hello", "ends_with", "LO"));
        assert!(!text_compare("Hello", "starts_with", "lo"));
    }

    #[test]
    fn text_compare_empty_checks_use_raw_value() {
        assert!(text_compare("", "is_empty", ""));
        assert!(!text_compare(" ", "is_empty", ""));
        assert!(text_compare("x", "is_not_empty", ""));
        assert!(!text_compare("", "is_not_empty", ""));
    }

    #[test]
    fn text_compare_unknown_operator_is_false() {
        assert!(!text_compare("foo", "wat", "foo"));
    }

    #[test]
    fn num_compare_parses_integer() {
        let mut idx = 0;
        let (sql, vals) = num_compare("subs.id", "42", ">", &mut idx).unwrap();
        assert!(sql.contains("subs.id > "));
        assert_eq!(vals.len(), 1);
        assert_eq!(idx, 1);
    }

    #[test]
    fn num_compare_rejects_non_numeric() {
        let mut idx = 0;
        assert!(num_compare("subs.id", "x", ">", &mut idx).is_none());
        assert_eq!(idx, 0);
    }
}
