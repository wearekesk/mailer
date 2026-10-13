//! User-account settings, suppression list, and blocked-domain management.
//!
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder,
};

use crate::database::entities::{blocked_domains, login, suppression_list};
use crate::services::auth;
use crate::utils::errors::{AppError, AppResult};

// ── Profile / account settings ───────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct ProfileInput {
    pub name: Option<String>,
    pub company: Option<String>,
    pub username: String,
    pub timezone: Option<String>,
    pub language: Option<String>,
    pub send_rate: Option<i32>,
    pub ses_endpoint: Option<String>,
    pub s3_key: Option<String>,
    pub s3_secret: Option<String>,
    pub paypal: Option<String>,
    pub brands_rows: Option<i32>,
    pub strict_delete: i16,
    pub dark_mode: i16,
    /// Optional new password — when `Some`, gets re-hashed with argon2.
    pub new_password: Option<String>,
}

pub async fn save_profile(
    db: &DatabaseConnection,
    user_id: i32,
    input: ProfileInput,
) -> AppResult<()> {
    let row = login::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("user {user_id}")))?;

    let mut active: login::ActiveModel = row.into();
    active.name = Set(input.name);
    active.company = Set(input.company);
    active.username = Set(Some(input.username));
    active.timezone = Set(input.timezone);
    active.language = Set(input.language);
    active.send_rate = Set(input.send_rate);
    active.ses_endpoint = Set(input.ses_endpoint);
    active.s3_key = Set(input.s3_key);
    active.s3_secret = Set(input.s3_secret);
    active.paypal = Set(input.paypal);
    active.brands_rows = Set(input.brands_rows);
    active.strict_delete = Set(Some(input.strict_delete));
    active.dark_mode = Set(Some(input.dark_mode));
    if let Some(pw) = input.new_password.as_deref() {
        if !pw.is_empty() {
            let hashed = auth::hash_password(pw)?;
            active.password = Set(Some(hashed));
        }
    }
    active.update(db).await?;
    Ok(())
}

pub async fn regenerate_api_key(db: &DatabaseConnection, user_id: i32) -> AppResult<String> {
    let row = login::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("user {user_id}")))?;
    let prev = row.api_key.clone();
    let new_key = random_alphanumeric(40);
    let mut active: login::ActiveModel = row.into();
    active.api_key = Set(Some(new_key.clone()));
    active.api_key_prev = Set(prev);
    active.update(db).await?;
    Ok(new_key)
}

pub async fn restore_api_key(db: &DatabaseConnection, user_id: i32) -> AppResult<String> {
    let row = login::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("user {user_id}")))?;
    let prev = row.api_key_prev.clone().unwrap_or_default();
    if prev.is_empty() {
        return Err(AppError::BadRequest("no previous API key".into()));
    }
    let mut active: login::ActiveModel = row.into();
    active.api_key = Set(Some(prev.clone()));
    active.api_key_prev = Set(Some(String::new()));
    active.update(db).await?;
    Ok(prev)
}

// ── Suppression list ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SuppressionEntry {
    pub id: i32,
    pub email: String,
    pub timestamp: i64,
    pub block_attempts: i32,
}

pub async fn list_suppression(
    db: &DatabaseConnection,
    app_id: i32,
    page: u64,
    per_page: u64,
) -> AppResult<(Vec<SuppressionEntry>, u64)> {
    let paginator = suppression_list::Entity::find()
        .filter(suppression_list::Column::App.eq(app_id))
        .order_by_desc(suppression_list::Column::Id)
        .paginate(db, per_page);
    let total = paginator.num_items().await?;
    let rows = paginator
        .fetch_page(page)
        .await?
        .into_iter()
        .map(|r| SuppressionEntry {
            id: r.id,
            email: r.email.unwrap_or_default(),
            timestamp: r.timestamp.unwrap_or(0),
            block_attempts: r.block_attempts.unwrap_or(0),
        })
        .collect();
    Ok((rows, total))
}

pub async fn add_suppression(
    db: &DatabaseConnection,
    app_id: i32,
    email: &str,
) -> AppResult<()> {
    let email = email.trim().to_lowercase();
    if email.is_empty() {
        return Err(AppError::BadRequest("email required".into()));
    }
    let existing = suppression_list::Entity::find()
        .filter(suppression_list::Column::App.eq(app_id))
        .filter(suppression_list::Column::Email.eq(&email))
        .one(db)
        .await?;
    if existing.is_some() {
        return Ok(());
    }
    let active = suppression_list::ActiveModel {
        app: Set(Some(app_id)),
        email: Set(Some(email)),
        timestamp: Set(Some(chrono::Utc::now().timestamp())),
        block_attempts: Set(Some(0)),
        ..Default::default()
    };
    active.insert(db).await?;
    Ok(())
}

pub async fn delete_suppression(db: &DatabaseConnection, id: i32) -> AppResult<()> {
    suppression_list::Entity::delete_by_id(id).exec(db).await?;
    Ok(())
}

/// Wipe every suppression entry for a brand.
pub async fn clear_suppression_list(db: &DatabaseConnection, app_id: i32) -> AppResult<u64> {
    let result = suppression_list::Entity::delete_many()
        .filter(suppression_list::Column::App.eq(app_id))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

/// Bulk-import a suppression-list CSV. The CSV must have an `email` (or
/// just one) column; rows with invalid emails are skipped. Returns
/// (inserted, skipped).
pub async fn import_suppression_csv<R: std::io::Read>(
    db: &DatabaseConnection,
    app_id: i32,
    reader: &mut csv::Reader<R>,
) -> AppResult<(usize, usize)> {
    let now = chrono::Utc::now().timestamp();
    let mut inserted = 0usize;
    let mut skipped = 0usize;
    for record in reader.records() {
        let Ok(record) = record else { skipped += 1; continue };
        let email = record.get(0).unwrap_or("").trim().to_lowercase();
        if email.is_empty() || !email.contains('@') {
            skipped += 1;
            continue;
        }
        let exists = suppression_list::Entity::find()
            .filter(suppression_list::Column::App.eq(app_id))
            .filter(suppression_list::Column::Email.eq(&email))
            .one(db)
            .await?
            .is_some();
        if exists {
            skipped += 1;
            continue;
        }
        let active = suppression_list::ActiveModel {
            app: Set(Some(app_id)),
            email: Set(Some(email)),
            timestamp: Set(Some(now)),
            block_attempts: Set(Some(0)),
            ..Default::default()
        };
        active.insert(db).await?;
        inserted += 1;
    }
    Ok((inserted, skipped))
}

// ── Blocked domains ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BlockedDomainEntry {
    pub id: i32,
    pub domain: String,
    pub timestamp: i64,
    pub block_attempts: i32,
}

pub async fn list_blocked_domains(
    db: &DatabaseConnection,
    app_id: i32,
    page: u64,
    per_page: u64,
) -> AppResult<(Vec<BlockedDomainEntry>, u64)> {
    let paginator = blocked_domains::Entity::find()
        .filter(blocked_domains::Column::App.eq(app_id))
        .order_by_desc(blocked_domains::Column::Id)
        .paginate(db, per_page);
    let total = paginator.num_items().await?;
    let rows = paginator
        .fetch_page(page)
        .await?
        .into_iter()
        .map(|r| BlockedDomainEntry {
            id: r.id,
            domain: r.domain.unwrap_or_default(),
            timestamp: r.timestamp.unwrap_or(0),
            block_attempts: r.block_attempts.unwrap_or(0),
        })
        .collect();
    Ok((rows, total))
}

pub async fn add_blocked_domain(
    db: &DatabaseConnection,
    app_id: i32,
    domain: &str,
) -> AppResult<()> {
    let domain = domain.trim().trim_start_matches('@').to_lowercase();
    if domain.is_empty() {
        return Err(AppError::BadRequest("domain required".into()));
    }
    let existing = blocked_domains::Entity::find()
        .filter(blocked_domains::Column::App.eq(app_id))
        .filter(blocked_domains::Column::Domain.eq(&domain))
        .one(db)
        .await?;
    if existing.is_some() {
        return Ok(());
    }
    let active = blocked_domains::ActiveModel {
        app: Set(Some(app_id)),
        domain: Set(Some(domain)),
        timestamp: Set(Some(chrono::Utc::now().timestamp())),
        block_attempts: Set(Some(0)),
        ..Default::default()
    };
    active.insert(db).await?;
    Ok(())
}

pub async fn delete_blocked_domain(db: &DatabaseConnection, id: i32) -> AppResult<()> {
    blocked_domains::Entity::delete_by_id(id).exec(db).await?;
    Ok(())
}

/// Wipe every blocked-domain entry for a brand.
pub async fn clear_blocked_domains(db: &DatabaseConnection, app_id: i32) -> AppResult<u64> {
    let result = blocked_domains::Entity::delete_many()
        .filter(blocked_domains::Column::App.eq(app_id))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

/// Bulk-import a blocked-domain CSV. The CSV must have a `domain` (or
/// just one) column. Returns (inserted, skipped).
pub async fn import_blocked_domains_csv<R: std::io::Read>(
    db: &DatabaseConnection,
    app_id: i32,
    reader: &mut csv::Reader<R>,
) -> AppResult<(usize, usize)> {
    let now = chrono::Utc::now().timestamp();
    let mut inserted = 0usize;
    let mut skipped = 0usize;
    for record in reader.records() {
        let Ok(record) = record else { skipped += 1; continue };
        let domain = record.get(0).unwrap_or("").trim().to_lowercase();
        if domain.is_empty() || !domain.contains('.') {
            skipped += 1;
            continue;
        }
        let exists = blocked_domains::Entity::find()
            .filter(blocked_domains::Column::App.eq(app_id))
            .filter(blocked_domains::Column::Domain.eq(&domain))
            .one(db)
            .await?
            .is_some();
        if exists {
            skipped += 1;
            continue;
        }
        let active = blocked_domains::ActiveModel {
            app: Set(Some(app_id)),
            domain: Set(Some(domain)),
            timestamp: Set(Some(now)),
            block_attempts: Set(Some(0)),
            ..Default::default()
        };
        active.insert(db).await?;
        inserted += 1;
    }
    Ok((inserted, skipped))
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn random_alphanumeric(len: usize) -> String {
    use rand::Rng;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}
