//! Provider webhook receivers for handling bounces, complaints, and delivery events
//! across AWS SES, Elastic Email, SendGrid, and Mailjet.
//!
//! The Rust handlers all share three primitives in
//! [`crate::services::subscribers`]:
//!
//! - [`mark_bounced`](crate::services::subscribers::mark_bounced)
//! - [`record_soft_bounce`](crate::services::subscribers::record_soft_bounce)
//! - [`mark_complaint`](crate::services::subscribers::mark_complaint)
//!
//! For the SNS endpoints we additionally honour the
//! `SubscriptionConfirmation` message type by GET-ting the `SubscribeURL` to
//! complete the topic subscription handshake.

use poem::{
    handler,
    http::StatusCode,
    web::{Data, Form, Json, Query},
    IntoResponse,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::services::subscribers;
use crate::utils::opaque_id::{List, OpaqueId};
use crate::AppState;

/// Default threshold before soft-bounces are promoted to hard-bounce.
const HARD_BOUNCE_THRESHOLD: i32 = 3;

/// One-shot HTTP client for completing SNS subscription confirmations.
async fn fetch_url(url: &str) -> bool {
    match reqwest::Client::new()
        .get(url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(e) => {
            tracing::warn!(error = ?e, url, "SNS SubscribeURL fetch failed");
            false
        }
    }
}

// ── AWS SNS ──────────────────────────────────────────────────────────────────

/// `POST /webhooks/sns/bounce`
#[handler]
pub async fn sns_bounce(
    Json(payload): Json<Value>,
    Data(state): Data<&AppState>,
) -> impl IntoResponse {
    if handle_sns_subscription_confirmation(&payload).await {
        return StatusCode::OK;
    }

    let Some(message) = parse_sns_message(&payload) else {
        return StatusCode::OK;
    };

    let bounce = match message.get("bounce") {
        Some(b) => b,
        None => return StatusCode::OK,
    };

    let bounce_type = bounce.get("bounceType").and_then(|v| v.as_str()).unwrap_or("");
    let bounce_subtype = bounce
        .get("bounceSubType")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let problem_email = bounce
        .get("bouncedRecipients")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|r| r.get("emailAddress"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if !subscribers::is_valid_email(problem_email) {
        return StatusCode::OK;
    }

    let result = match bounce_type {
        "Permanent" => subscribers::mark_bounced(&state.db, problem_email).await.map(|_| ()),
        "Transient" if bounce_subtype != "General" => {
            subscribers::record_soft_bounce(&state.db, problem_email, HARD_BOUNCE_THRESHOLD).await
        }
        _ => Ok(()),
    };
    if let Err(e) = result {
        tracing::error!(error = ?e, "sns_bounce update failed");
    }
    StatusCode::OK
}

/// `POST /webhooks/sns/complaint`
#[handler]
pub async fn sns_complaint(
    Json(payload): Json<Value>,
    Data(state): Data<&AppState>,
) -> impl IntoResponse {
    if handle_sns_subscription_confirmation(&payload).await {
        return StatusCode::OK;
    }

    let Some(message) = parse_sns_message(&payload) else {
        return StatusCode::OK;
    };

    let complaint = match message.get("complaint") {
        Some(c) => c,
        None => return StatusCode::OK,
    };

    let problem_email = complaint
        .get("complainedRecipients")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|r| r.get("emailAddress"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if !subscribers::is_valid_email(problem_email) {
        return StatusCode::OK;
    }

    if let Err(e) = subscribers::mark_complaint(&state.db, problem_email).await {
        tracing::error!(error = ?e, "sns_complaint update failed");
    }
    StatusCode::OK
}

/// `POST /webhooks/sns/confirm` — explicit endpoint for the topic
/// subscription handshake. (The bounce/complaint handlers above also handle
/// the handshake, so this is a convenience.)
#[handler]
pub async fn sns_confirm(Json(payload): Json<Value>) -> impl IntoResponse {
    handle_sns_subscription_confirmation(&payload).await;
    StatusCode::OK
}

/// If `payload.Type == "SubscriptionConfirmation"` and `SubscribeURL` lives
/// under `*.amazonaws.com`, GET the URL to complete the handshake.
async fn handle_sns_subscription_confirmation(payload: &Value) -> bool {
    if payload.get("Type").and_then(|v| v.as_str()) != Some("SubscriptionConfirmation") {
        return false;
    }
    let Some(url) = payload.get("SubscribeURL").and_then(|v| v.as_str()) else {
        return false;
    };
    if !is_amazonaws_url(url) {
        tracing::warn!(url, "SNS SubscribeURL not from amazonaws.com — ignoring");
        return true; // Still "handled" (we refuse) — caller shouldn't process further.
    }
    fetch_url(url).await;
    true
}

fn is_amazonaws_url(url: &str) -> bool {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .map(|host| {
            host.ends_with(".amazonaws.com") || host == "amazonaws.com"
        })
        .unwrap_or(false)
}

/// SNS wraps the inner JSON inside a string `Message` field.
fn parse_sns_message(payload: &Value) -> Option<Value> {
    let raw = payload.get("Message").and_then(|v| v.as_str())?;
    serde_json::from_str(raw).ok()
}

// ── Elastic Email ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ElasticEmailQuery {
    /// Event status: `AbuseReport`, `Error`, `Unsubscribed`, etc.
    #[serde(default)]
    pub status: Option<String>,
    /// Bounce subcategory used to detect hard bounces.
    #[serde(default)]
    pub category: Option<String>,
    /// Recipient email address.
    #[serde(default)]
    pub to: Option<String>,
    /// Encrypted list id, sent back as the `postback` parameter.
    #[serde(default)]
    pub postback: Option<String>,
}

const ELASTICEMAIL_HARD_BOUNCE_CATEGORIES: &[&str] = &[
    "BlackListed",
    "AccountProblem",
    "DNSProblem",
    "NoMailbox",
];

/// `POST /webhooks/elasticemail` — Elastic Email sends events as GET-style
/// query parameters.
#[handler]
pub async fn elasticemail(
    Query(q): Query<ElasticEmailQuery>,
    Data(state): Data<&AppState>,
) -> impl IntoResponse {
    let problem_email = q.to.as_deref().unwrap_or("");
    if !subscribers::is_valid_email(problem_email) {
        return StatusCode::OK;
    }
    let status = q.status.as_deref().unwrap_or("");
    let category = q.category.as_deref().unwrap_or("");
    let list_id = q.postback.as_deref().and_then(parse_list_token);

    if ELASTICEMAIL_HARD_BOUNCE_CATEGORIES.contains(&category) {
        if let Err(e) = subscribers::mark_bounced(&state.db, problem_email).await {
            tracing::error!(error = ?e, "elasticemail mark_bounced failed");
        }
        return StatusCode::OK;
    }

    match status {
        "AbuseReport" => {
            if let Err(e) = subscribers::mark_complaint(&state.db, problem_email).await {
                tracing::error!(error = ?e, "elasticemail complaint failed");
            }
        }
        "Unsubscribed" => {
            if let Some(list_id) = list_id {
                let _ = subscribers::unsubscribe(&state.db, problem_email, list_id).await;
            }
        }
        _ => {}
    }
    StatusCode::OK
}

// ── Mailjet ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct MailjetEvent {
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default, rename = "CustomID")]
    pub custom_id: Option<String>,
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default)]
    pub hard_bounce: Option<Value>,
}

/// `POST /webhooks/mailjet`
#[handler]
pub async fn mailjet(
    Json(events): Json<Vec<MailjetEvent>>,
    Data(state): Data<&AppState>,
) -> impl IntoResponse {
    for evt in events {
        let Some(email) = evt.email.as_deref() else { continue };
        if !subscribers::is_valid_email(email) {
            continue;
        }
        let list_id = evt.custom_id.as_deref().and_then(parse_list_token);
        let event = evt.event.as_deref().unwrap_or("");
        let hard_bounce = matches!(evt.hard_bounce, Some(Value::Bool(true)) | Some(Value::Number(_)) | Some(Value::String(_)));

        match event {
            "bounce" if hard_bounce => {
                let _ = subscribers::mark_bounced(&state.db, email).await;
            }
            "spam" => {
                let _ = subscribers::mark_complaint(&state.db, email).await;
            }
            "unsub" => {
                if let Some(list_id) = list_id {
                    let _ = subscribers::unsubscribe(&state.db, email, list_id).await;
                }
            }
            _ => {}
        }
    }
    StatusCode::OK
}

// ── SendGrid ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SendGridEvent {
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub event: Option<String>,
    /// SendGrid sends an array of category strings.
    #[serde(default)]
    pub category: Vec<String>,
}

/// `POST /webhooks/sendgrid`
#[handler]
pub async fn sendgrid(
    Json(events): Json<Vec<SendGridEvent>>,
    Data(state): Data<&AppState>,
) -> impl IntoResponse {
    for evt in events {
        let Some(email) = evt.email.as_deref() else { continue };
        if !subscribers::is_valid_email(email) {
            continue;
        }
        let list_id = evt.category.first().and_then(|s| parse_list_token(s.as_str()));
        let event = evt.event.as_deref().unwrap_or("");

        match event {
            "bounce" => {
                let _ = subscribers::mark_bounced(&state.db, email).await;
            }
            "spamreport" => {
                let _ = subscribers::mark_complaint(&state.db, email).await;
            }
            "unsubscribe" => {
                if let Some(list_id) = list_id {
                    let _ = subscribers::unsubscribe(&state.db, email, list_id).await;
                }
            }
            _ => {}
        }
    }
    StatusCode::OK
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn parse_list_token(token: &str) -> Option<i32> {
    token.parse::<OpaqueId<List>>().ok().map(|id| id.raw())
}

/// Convenience for poem `Form<HashMap>` extraction in case any provider sends
/// `application/x-www-form-urlencoded` we don't model — allows future
/// reactive handling without changing signatures.
#[allow(dead_code)]
fn form_value<'a>(params: &'a HashMap<String, String>, key: &str) -> &'a str {
    params.get(key).map(String::as_str).unwrap_or("")
}

#[allow(dead_code)]
async fn _unused_form_extractor(_: Form<HashMap<String, String>>) {}
