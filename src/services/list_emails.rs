//! List-event emails — confirmation (double opt-in), thank-you (after
//! confirm or single-opt-in), and goodbye (after unsubscribe).
//!
//! Resolves the brand SMTP configuration, renders personalized tags
//! (`[Name]`, `[Email]`, `[unsubscribe]`, `[confirmation_link]`), and sends via lettre.

use sea_orm::{DatabaseConnection, EntityTrait};

use crate::database::entities::{apps, lists, subscribers};
use crate::services::mailer::{mailer_for_brand, OutboundEmail, StubMailer};
use crate::services::personalization::{render, Context};
use crate::utils::errors::AppResult;
use crate::utils::opaque_id::{List, OpaqueId, Subscriber};

/// Build the canonical `/confirm?e=…&l=…` URL for a subscriber + list.
pub fn confirmation_link(app_url: &str, subscriber_id: i32, list_id: i32) -> String {
    format!(
        "{}/confirm?e={}&l={}",
        app_url.trim_end_matches('/'),
        OpaqueId::<Subscriber>::new(subscriber_id),
        OpaqueId::<List>::new(list_id),
    )
}

/// Default body when `lists.confirmation_email` is empty.
fn default_confirmation_body(confirm_url: &str) -> String {
    format!(
        "<p>Please confirm your subscription by clicking the link below.</p>\
         <p><a href=\"{u}\">{u}</a></p>",
        u = confirm_url
    )
}

fn default_thankyou_body() -> &'static str {
    "<p>Thank you for subscribing — we're glad to have you.</p>"
}

fn default_goodbye_body() -> &'static str {
    "<p>You've been unsubscribed. Sorry to see you go.</p>"
}

async fn dispatch(
    db: &DatabaseConnection,
    brand: Option<&apps::Model>,
    subscriber: &subscribers::Model,
    subject: String,
    html: String,
) -> AppResult<()> {
    let to_email = match subscriber.email.clone() {
        Some(e) if !e.is_empty() => e,
        _ => return Ok(()),
    };

    let from_name = brand
        .and_then(|b| b.from_name.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Mailer".to_string());
    let from_email = brand
        .and_then(|b| b.from_email.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "no-reply@mailer.local".to_string());
    let reply_to = brand
        .and_then(|b| b.reply_to.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| from_email.clone());

    let outbound = OutboundEmail {
        from_name,
        from_email,
        to_email,
        to_name: subscriber.name.clone(),
        reply_to,
        subject,
        html,
        plain: String::new(),
        attachments: Vec::new(),
    };

    let mailer: Box<dyn crate::services::mailer::EmailProvider> = brand
        .map(mailer_for_brand)
        .unwrap_or_else(|| Box::new(StubMailer));

    if let Err(e) = mailer.send(outbound).await {
        tracing::warn!(error = ?e, subscriber = subscriber.id, "list-email send failed");
    }
    let _ = db; // db not needed for sending; reserved for future logging
    Ok(())
}

/// Send the double-opt-in confirmation email.
pub async fn send_confirmation(
    db: &DatabaseConnection,
    subscriber: &subscribers::Model,
    list: &lists::Model,
    app_url: &str,
) -> AppResult<()> {
    let brand_id = list.app.unwrap_or_default();
    let brand = apps::Entity::find_by_id(brand_id).one(db).await?;

    let confirm_url = confirmation_link(app_url, subscriber.id, list.id);
    let subject_template = list
        .confirmation_subject
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Confirm your subscription".to_string());
    let body_template = list
        .confirmation_email
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default_confirmation_body(&confirm_url));

    // Resolve [confirmation_link] before generic personalization.
    let body_with_link = body_template.replace("[confirmation_link]", &confirm_url);

    let cf_packed = list.custom_fields.clone().unwrap_or_default();
    let ctx = Context::build(subscriber, list.id, 0, &cf_packed, app_url);
    let subject = render(&subject_template, &ctx);
    let html = render(&body_with_link, &ctx);

    dispatch(db, brand.as_ref(), subscriber, subject, html).await
}

/// Send the thank-you / welcome email (only if the list has it enabled).
pub async fn send_thank_you(
    db: &DatabaseConnection,
    subscriber: &subscribers::Model,
    list: &lists::Model,
    app_url: &str,
) -> AppResult<()> {
    if list.thankyou.unwrap_or(0) != 1 {
        return Ok(());
    }
    let brand_id = list.app.unwrap_or_default();
    let brand = apps::Entity::find_by_id(brand_id).one(db).await?;

    let subject_template = list
        .thankyou_subject
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Thanks for subscribing".to_string());
    let body_template = list
        .thankyou_message
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default_thankyou_body().to_string());

    let cf_packed = list.custom_fields.clone().unwrap_or_default();
    let ctx = Context::build(subscriber, list.id, 0, &cf_packed, app_url);
    let subject = render(&subject_template, &ctx);
    let html = render(&body_template, &ctx);

    dispatch(db, brand.as_ref(), subscriber, subject, html).await
}

/// Send the goodbye email (only if the list has it enabled).
pub async fn send_goodbye(
    db: &DatabaseConnection,
    subscriber: &subscribers::Model,
    list: &lists::Model,
    app_url: &str,
) -> AppResult<()> {
    if list.goodbye.unwrap_or(0) != 1 {
        return Ok(());
    }
    let brand_id = list.app.unwrap_or_default();
    let brand = apps::Entity::find_by_id(brand_id).one(db).await?;

    let subject_template = list
        .goodbye_subject
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "You've been unsubscribed".to_string());
    let body_template = list
        .goodbye_message
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default_goodbye_body().to_string());

    let cf_packed = list.custom_fields.clone().unwrap_or_default();
    let ctx = Context::build(subscriber, list.id, 0, &cf_packed, app_url);
    let subject = render(&subject_template, &ctx);
    let html = render(&body_template, &ctx);

    dispatch(db, brand.as_ref(), subscriber, subject, html).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmation_link_contains_opaque_tokens() {
        crate::utils::crypto::init_secret("test-secret");
        let url = confirmation_link("http://app.test/", 42, 7);
        assert!(url.starts_with("http://app.test/confirm?e="));
        assert!(url.contains("&l="));
        // tokens should not be plain integers
        assert!(!url.contains("?e=42"));
        assert!(!url.contains("&l=7"));
    }

    #[test]
    fn confirmation_link_strips_trailing_slash() {
        crate::utils::crypto::init_secret("test-secret");
        let with = confirmation_link("http://app.test/", 1, 1);
        let without = confirmation_link("http://app.test", 1, 1);
        assert_eq!(with, without);
    }

    #[test]
    fn default_bodies_contain_expected_phrases() {
        let url = "http://app.test/confirm?e=abc&l=def";
        let conf = default_confirmation_body(url);
        assert!(conf.contains("confirm your subscription"));
        assert!(conf.contains(url));

        assert!(default_thankyou_body().contains("Thank you"));
        assert!(default_goodbye_body().contains("unsubscribed"));
    }
}
