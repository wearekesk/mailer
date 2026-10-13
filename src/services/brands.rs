//! Brand (app) management and CRUD.
//!
//! The `apps` table stores Mailer brands — each brand has its own SMTP,
//! its own from_name/from_email/reply_to, its own monthly quota and tracking
//! preferences. Lists, campaigns, autoresponders all hang off a brand id.

use sea_orm::{
    sea_query::{Expr, Query},
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseBackend,
    DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Statement,
};

use crate::database::entities::{apps, lists, subscribers};
use crate::utils::errors::AppResult;

// ── DTOs ─────────────────────────────────────────────────────────────────────

/// Lightweight summary used by list-views and the JSON API.
#[derive(Debug, Clone)]
pub struct BrandSummary {
    pub id: i32,
    pub name: String,
    pub from_email: Option<String>,
    pub logo_filename: Option<String>,
    pub list_count: i64,
    pub subscriber_count: i64,
}

/// All editable fields on a brand. Optional `id` — `None` ⇒ insert,
/// `Some(_)` ⇒ update.
#[derive(Debug, Clone, Default)]
pub struct BrandInput {
    pub app_name: String,
    pub from_name: Option<String>,
    pub from_email: Option<String>,
    pub reply_to: Option<String>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<String>,
    pub smtp_ssl: Option<String>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub allowed_attachments: Option<String>,
    pub query_string: Option<String>,
    pub gdpr_only: bool,
    pub gdpr_only_ar: bool,
    pub gdpr_options: bool,
    pub recaptcha_sitekey: Option<String>,
    pub recaptcha_secretkey: Option<String>,
    pub test_email_prefix: Option<String>,
    pub custom_domain: Option<String>,
    pub custom_domain_protocol: Option<String>,
    pub custom_domain_enabled: bool,
    pub templates_lists_sorting: Option<String>,
    pub opens_tracking: i16,
    pub links_tracking: i16,
    pub hide_lists: i16,
    pub opt_in: i16,
    pub allocated_quota: Option<i32>,
    pub campaign_report_rows: Option<i32>,
    pub currency: Option<String>,
    pub delivery_fee: Option<String>,
    pub cost_per_recipient: Option<String>,
    /// Sub-user UI restrictions: each true means "when a sub-user is
    /// acting on this brand, hide everything except <X>".
    pub campaigns_only: bool,
    pub templates_only: bool,
    pub lists_only: bool,
}

// ── Public API ───────────────────────────────────────────────────────────────

/// List all brands with aggregated list and subscriber counts.
pub async fn list_brands(db: &DatabaseConnection) -> AppResult<Vec<BrandSummary>> {
    // We do this with two follow-up COUNT queries instead of a single GROUP BY
    // join — easier to read and only runs on the brand list page.
    let rows = apps::Entity::find()
        .order_by_asc(apps::Column::Id)
        .all(db)
        .await?;

    let mut summaries = Vec::with_capacity(rows.len());
    for a in rows {
        let list_count = lists::Entity::find()
            .filter(lists::Column::App.eq(a.id))
            .count(db)
            .await? as i64;
        let subscriber_count = subscriber_count_for_brand(db, a.id).await? as i64;
        summaries.push(BrandSummary {
            id: a.id,
            name: a.app_name.unwrap_or_default(),
            from_email: a.from_email,
            logo_filename: a.brand_logo_filename.filter(|s| !s.is_empty()),
            list_count,
            subscriber_count,
        });
    }

    Ok(summaries)
}

async fn subscriber_count_for_brand(db: &DatabaseConnection, brand_id: i32) -> AppResult<u64> {
    // SELECT COUNT(*) FROM subscribers s JOIN lists l ON s.list = l.id
    // WHERE l.app = brand_id
    let stmt = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        r#"SELECT COUNT(*) AS c FROM subscribers s
           JOIN lists l ON s.list = l.id
           WHERE l.app = $1"#,
        [brand_id.into()],
    );
    let row = db.query_one(stmt).await?;
    let count: i64 = row
        .map(|r| r.try_get::<i64>("", "c").unwrap_or(0))
        .unwrap_or(0);
    Ok(count.max(0) as u64)
}

/// Look up a single brand by id.
pub async fn find_brand(db: &DatabaseConnection, brand_id: i32) -> AppResult<Option<apps::Model>> {
    Ok(apps::Entity::find_by_id(brand_id).one(db).await?)
}

/// Insert a new brand. Returns the new row id.
pub async fn create_brand(
    db: &DatabaseConnection,
    user_id: i32,
    input: BrandInput,
) -> AppResult<i32> {
    let app_key = random_app_key();
    let active = apps::ActiveModel {
        user_id: Set(Some(user_id)),
        app_name: Set(Some(input.app_name)),
        from_name: Set(input.from_name),
        from_email: Set(input.from_email),
        reply_to: Set(input.reply_to),
        smtp_host: Set(input.smtp_host),
        smtp_port: Set(input.smtp_port),
        smtp_ssl: Set(input.smtp_ssl),
        smtp_username: Set(input.smtp_username),
        smtp_password: Set(input.smtp_password),
        allowed_attachments: Set(input.allowed_attachments),
        query_string: Set(input.query_string),
        gdpr_only: Set(Some(if input.gdpr_only { 1 } else { 0 })),
        gdpr_only_ar: Set(Some(if input.gdpr_only_ar { 1 } else { 0 })),
        gdpr_options: Set(Some(if input.gdpr_options { 1 } else { 0 })),
        recaptcha_sitekey: Set(input.recaptcha_sitekey),
        recaptcha_secretkey: Set(input.recaptcha_secretkey),
        test_email_prefix: Set(input.test_email_prefix),
        custom_domain: Set(input.custom_domain),
        custom_domain_protocol: Set(input.custom_domain_protocol),
        custom_domain_enabled: Set(Some(if input.custom_domain_enabled { 1 } else { 0 })),
        templates_lists_sorting: Set(input.templates_lists_sorting),
        opens_tracking: Set(Some(input.opens_tracking)),
        links_tracking: Set(Some(input.links_tracking)),
        hide_lists: Set(Some(input.hide_lists)),
        opt_in: Set(Some(input.opt_in)),
        allocated_quota: Set(input.allocated_quota),
        current_quota: Set(Some(0)),
        campaign_report_rows: Set(input.campaign_report_rows),
        currency: Set(input.currency),
        delivery_fee: Set(input.delivery_fee),
        cost_per_recipient: Set(input.cost_per_recipient),
        campaigns_only: Set(Some(if input.campaigns_only { 1 } else { 0 })),
        templates_only: Set(Some(if input.templates_only { 1 } else { 0 })),
        lists_only: Set(Some(if input.lists_only { 1 } else { 0 })),
        app_key: Set(Some(app_key)),
        ..Default::default()
    };
    let inserted = active.insert(db).await?;
    Ok(inserted.id)
}

/// Update an existing brand in place.
pub async fn update_brand(
    db: &DatabaseConnection,
    brand_id: i32,
    input: BrandInput,
) -> AppResult<()> {
    let row = apps::Entity::find_by_id(brand_id)
        .one(db)
        .await?
        .ok_or_else(|| crate::utils::errors::AppError::NotFound(format!("brand {brand_id}")))?;

    let mut active: apps::ActiveModel = row.into();
    active.app_name = Set(Some(input.app_name));
    active.from_name = Set(input.from_name);
    active.from_email = Set(input.from_email);
    active.reply_to = Set(input.reply_to);
    active.smtp_host = Set(input.smtp_host);
    active.smtp_port = Set(input.smtp_port);
    active.smtp_ssl = Set(input.smtp_ssl);
    active.smtp_username = Set(input.smtp_username);
    active.smtp_password = Set(input.smtp_password);
    active.allowed_attachments = Set(input.allowed_attachments);
    active.query_string = Set(input.query_string);
    active.gdpr_only = Set(Some(if input.gdpr_only { 1 } else { 0 }));
    active.gdpr_only_ar = Set(Some(if input.gdpr_only_ar { 1 } else { 0 }));
    active.gdpr_options = Set(Some(if input.gdpr_options { 1 } else { 0 }));
    active.recaptcha_sitekey = Set(input.recaptcha_sitekey);
    active.recaptcha_secretkey = Set(input.recaptcha_secretkey);
    active.test_email_prefix = Set(input.test_email_prefix);
    active.custom_domain = Set(input.custom_domain);
    active.custom_domain_protocol = Set(input.custom_domain_protocol);
    active.custom_domain_enabled = Set(Some(if input.custom_domain_enabled { 1 } else { 0 }));
    active.templates_lists_sorting = Set(input.templates_lists_sorting);
    active.opens_tracking = Set(Some(input.opens_tracking));
    active.links_tracking = Set(Some(input.links_tracking));
    active.hide_lists = Set(Some(input.hide_lists));
    active.opt_in = Set(Some(input.opt_in));
    active.allocated_quota = Set(input.allocated_quota);
    active.campaign_report_rows = Set(input.campaign_report_rows);
    active.currency = Set(input.currency);
    active.delivery_fee = Set(input.delivery_fee);
    active.cost_per_recipient = Set(input.cost_per_recipient);
    active.campaigns_only = Set(Some(if input.campaigns_only { 1 } else { 0 }));
    active.templates_only = Set(Some(if input.templates_only { 1 } else { 0 }));
    active.lists_only = Set(Some(if input.lists_only { 1 } else { 0 }));
    active.update(db).await?;
    Ok(())
}

/// Delete a brand and every list/subscriber that hangs off it.
pub async fn delete_brand(db: &DatabaseConnection, brand_id: i32) -> AppResult<()> {
    // Find every list belonging to this brand, then delete subscribers /
    // segments / autoresponders / campaigns for those lists, then the
    // lists themselves, then the brand.
    let list_ids: Vec<i32> = lists::Entity::find()
        .filter(lists::Column::App.eq(brand_id))
        .all(db)
        .await?
        .into_iter()
        .map(|l| l.id)
        .collect();

    if !list_ids.is_empty() {
        subscribers::Entity::delete_many()
            .filter(subscribers::Column::List.is_in(list_ids.clone()))
            .exec(db)
            .await?;
    }

    lists::Entity::delete_many()
        .filter(lists::Column::App.eq(brand_id))
        .exec(db)
        .await?;

    apps::Entity::delete_by_id(brand_id).exec(db).await?;
    Ok(())
}

/// Duplicate a brand row (apps row only — does NOT clone its lists). Returns
/// the new row id.
pub async fn duplicate_brand(db: &DatabaseConnection, brand_id: i32) -> AppResult<i32> {
    let row = apps::Entity::find_by_id(brand_id)
        .one(db)
        .await?
        .ok_or_else(|| crate::utils::errors::AppError::NotFound(format!("brand {brand_id}")))?;

    let original_name = row.app_name.clone().unwrap_or_default();
    let mut new_active: apps::ActiveModel = row.into();
    new_active.id = sea_orm::ActiveValue::NotSet;
    new_active.app_name = Set(Some(format!("{original_name} (copy)")));
    new_active.app_key = Set(Some(random_app_key()));
    new_active.current_quota = Set(Some(0));
    let inserted = new_active.insert(db).await?;
    Ok(inserted.id)
}

fn random_app_key() -> String {
    use rand::Rng;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..30)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}

// PaginatorTrait import for `.count(db)` above.
use sea_orm::PaginatorTrait;
// keep the unused-import warnings quiet if we add helpers later
#[allow(unused_imports)]
use Expr as _Expr;
#[allow(unused_imports)]
use Query as _Query;
