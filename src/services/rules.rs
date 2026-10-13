//! Rules service for event triggers and automated actions.
//!
//! A rule subscribes to a trigger (e.g. `subscribe`, `unsubscribe`,
//! `campaign_sent`) and runs an action (notify email, webhook). Stored in
//! the `rules` table.

use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder,
};

use crate::database::entities::rules;
use crate::utils::errors::{AppError, AppResult};

#[derive(Debug, Clone, Default)]
pub struct RuleInput {
    pub brand_id: i32,
    pub app_id: i32,
    pub trigger: String,
    pub action: String,
    pub notification_email: Option<String>,
    pub webhook_url: Option<String>,
    pub list_id: Option<i32>,
    pub ares_id: Option<i32>,
}

pub async fn list_for_brand(
    db: &DatabaseConnection,
    brand_id: i32,
) -> AppResult<Vec<rules::Model>> {
    Ok(rules::Entity::find()
        .filter(rules::Column::Brand.eq(brand_id))
        .order_by_asc(rules::Column::Id)
        .all(db)
        .await?)
}

pub async fn find(db: &DatabaseConnection, id: i32) -> AppResult<Option<rules::Model>> {
    Ok(rules::Entity::find_by_id(id).one(db).await?)
}

pub async fn save(db: &DatabaseConnection, input: RuleInput) -> AppResult<i32> {
    let active = rules::ActiveModel {
        brand: Set(Some(input.brand_id)),
        app: Set(Some(input.app_id)),
        trigger_col: Set(Some(input.trigger)),
        action: Set(Some(input.action)),
        notification_email: Set(input.notification_email),
        endpoint: Set(input.webhook_url),
        list: Set(input.list_id),
        ares_id: Set(input.ares_id),
        enabled: Set(Some(1)),
        ..Default::default()
    };
    Ok(active.insert(db).await?.id)
}

pub async fn update(db: &DatabaseConnection, id: i32, input: RuleInput) -> AppResult<()> {
    let row = rules::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("rule {id}")))?;
    let mut active: rules::ActiveModel = row.into();
    active.trigger_col = Set(Some(input.trigger));
    active.action = Set(Some(input.action));
    active.notification_email = Set(input.notification_email);
    active.endpoint = Set(input.webhook_url);
    active.list = Set(input.list_id);
    active.ares_id = Set(input.ares_id);
    active.update(db).await?;
    Ok(())
}

pub async fn toggle(db: &DatabaseConnection, id: i32) -> AppResult<bool> {
    let row = rules::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("rule {id}")))?;
    let new_value = if row.enabled.unwrap_or(0) == 1 { 0 } else { 1 };
    let mut active: rules::ActiveModel = row.into();
    active.enabled = Set(Some(new_value));
    active.update(db).await?;
    Ok(new_value == 1)
}

pub async fn delete(db: &DatabaseConnection, id: i32) -> AppResult<()> {
    rules::Entity::delete_by_id(id).exec(db).await?;
    Ok(())
}

pub async fn count_for_brand(db: &DatabaseConnection, brand_id: i32) -> AppResult<i64> {
    Ok(rules::Entity::find()
        .filter(rules::Column::Brand.eq(brand_id))
        .count(db)
        .await? as i64)
}

// ── Event dispatch ──────────────────────────────────────────────────────────

use std::collections::HashMap;

use crate::database::entities::{apps, campaigns, lists, subscribers};

/// Fire every enabled rule attached to `brand_id` whose trigger matches.
/// `context` is a flat key/value bag substituted into notification email
/// bodies and posted as JSON to webhook URLs.
pub async fn dispatch(
    db: &DatabaseConnection,
    brand_id: i32,
    trigger: &str,
    context: HashMap<String, String>,
) -> AppResult<()> {
    let matching = rules::Entity::find()
        .filter(rules::Column::Brand.eq(brand_id))
        .filter(rules::Column::TriggerCol.eq(trigger))
        .all(db)
        .await?;

    for rule in matching {
        if rule.enabled.unwrap_or(0) == 0 {
            continue;
        }
        match rule.action.as_deref() {
            Some("notify") => fire_notify(db, brand_id, &rule, trigger, &context).await,
            Some("webhook") => fire_webhook(&rule, trigger, &context).await,
            other => {
                tracing::debug!(?other, "rules::dispatch: unknown action");
            }
        }
    }
    Ok(())
}

async fn fire_notify(
    db: &DatabaseConnection,
    brand_id: i32,
    rule: &rules::Model,
    trigger: &str,
    context: &HashMap<String, String>,
) {
    let Some(to) = rule.notification_email.as_deref().filter(|s| !s.is_empty()) else {
        return;
    };

    let brand = apps::Entity::find_by_id(brand_id).one(db).await.ok().flatten();
    let from_name = brand
        .as_ref()
        .and_then(|b| b.from_name.clone())
        .unwrap_or_else(|| "Mailer".to_string());
    let from_email = brand
        .as_ref()
        .and_then(|b| b.from_email.clone())
        .unwrap_or_else(|| "no-reply@mailer.local".to_string());
    let reply_to = brand
        .as_ref()
        .and_then(|b| b.reply_to.clone())
        .unwrap_or(from_email.clone());

    let subject = format!("[{trigger}] notification");
    let mut body = format!("<h2>Mailer event: {trigger}</h2><ul>");
    for (k, v) in context {
        body.push_str(&format!(
            "<li><strong>{}:</strong> {}</li>",
            html_escape::encode_safe(k),
            html_escape::encode_safe(v)
        ));
    }
    body.push_str("</ul>");

    let outbound = crate::services::mailer::OutboundEmail {
        from_name,
        from_email,
        to_email: to.to_string(),
        to_name: None,
        reply_to,
        subject,
        html: body.clone(),
        plain: format!("Mailer event: {trigger}\n{context:?}"),
        attachments: Vec::new(),
    };

    let mailer: Box<dyn crate::services::mailer::EmailProvider> = brand
        .as_ref()
        .map(crate::services::mailer::mailer_for_brand)
        .unwrap_or_else(|| Box::new(crate::services::mailer::StubMailer));

    if let Err(e) = mailer.send(outbound).await {
        tracing::warn!(error = ?e, rule = rule.id, "rule notify send failed");
    }
}

async fn fire_webhook(rule: &rules::Model, trigger: &str, context: &HashMap<String, String>) {
    let Some(url) = rule.endpoint.as_deref().filter(|s| !s.is_empty()) else {
        return;
    };
    let mut body = serde_json::Map::new();
    body.insert("trigger".into(), serde_json::Value::String(trigger.to_string()));
    for (k, v) in context {
        body.insert(k.clone(), serde_json::Value::String(v.clone()));
    }
    let body = serde_json::Value::Object(body);
    let client = reqwest::Client::new();
    if let Err(e) = client
        .post(url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
    {
        tracing::warn!(error = ?e, rule = rule.id, %url, "rule webhook failed");
    }
}

// ── Convenience wrappers for the common triggers ────────────────────────────

/// Build a context for subscribe/unsubscribe events and dispatch.
pub async fn dispatch_subscriber_event(
    db: &DatabaseConnection,
    trigger: &str,
    subscriber: &subscribers::Model,
) -> AppResult<()> {
    let list_id = subscriber.list.unwrap_or_default();
    let list = lists::Entity::find_by_id(list_id).one(db).await?;
    let brand_id = list.as_ref().and_then(|l| l.app).unwrap_or_default();
    if brand_id == 0 {
        return Ok(());
    }

    let mut ctx = HashMap::new();
    ctx.insert(
        "name".into(),
        subscriber.name.clone().unwrap_or_default(),
    );
    ctx.insert(
        "email".into(),
        subscriber.email.clone().unwrap_or_default(),
    );
    if let Some(l) = list {
        ctx.insert("list_name".into(), l.name.unwrap_or_default());
        ctx.insert("list_id".into(), l.id.to_string());
    }
    dispatch(db, brand_id, trigger, ctx).await
}

/// Build a context for campaign events and dispatch.
pub async fn dispatch_campaign(
    db: &DatabaseConnection,
    brand_id: i32,
    campaign: &campaigns::Model,
    trigger: &str,
) -> AppResult<()> {
    let mut ctx = HashMap::new();
    ctx.insert(
        "campaign_title".into(),
        campaign.title.clone().unwrap_or_default(),
    );
    ctx.insert("subject".into(), campaign.subject.clone().unwrap_or_default());
    ctx.insert(
        "from_name".into(),
        campaign.from_name.clone().unwrap_or_default(),
    );
    ctx.insert(
        "from_email".into(),
        campaign.from_email.clone().unwrap_or_default(),
    );
    ctx.insert("brand_id".into(), brand_id.to_string());
    ctx.insert("campaign_id".into(), campaign.id.to_string());
    ctx.insert(
        "no_of_recipients".into(),
        campaign.recipients.unwrap_or(0).to_string(),
    );
    dispatch(db, brand_id, trigger, ctx).await
}
