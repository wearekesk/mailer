use poem::{
    handler,
    web::{Data, Json, Path},
    IntoResponse, Response,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::routes::api::{auth::ApiAuth, bad_request, internal_error, not_found};
use crate::services::{brands, rules::{self, RuleInput}};
use crate::utils::opaque_id::{Brand, OpaqueId};
use crate::AppState;

#[derive(Debug, Serialize, JsonSchema)]
pub struct RuleResponse {
    pub id: i32,
    pub brand_id: Option<OpaqueId<Brand>>,
    pub trigger: String,
    pub action: String,
    pub notification_email: Option<String>,
    pub webhook_url: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
pub struct CreateRuleBody {
    pub trigger: String,
    pub action: String,
    #[serde(default)]
    pub notification_email: Option<String>,
    #[serde(default)]
    pub webhook_url: Option<String>,
    #[serde(default)]
    pub list_id: Option<i32>,
    #[serde(default)]
    pub ares_id: Option<i32>,
}

/// `GET /api/brands/:brand_id/rules` — list automation rules for brand.
#[handler]
pub async fn list_for_brand(
    _auth: ApiAuth,
    Path(brand_id): Path<OpaqueId<Brand>>,
    Data(state): Data<&AppState>,
) -> Response {
    let brand_raw = brand_id.raw();
    match brands::find_brand(&state.db, brand_raw).await {
        Ok(Some(_)) => {}
        Ok(None) => return not_found("Brand does not exist."),
        Err(e) => {
            tracing::error!(error = ?e, "find_brand failed");
            return internal_error("Database error.");
        }
    }

    match rules::list_for_brand(&state.db, brand_raw).await {
        Ok(rows) => {
            let body: Vec<RuleResponse> = rows
                .into_iter()
                .map(|r| RuleResponse {
                    id: r.id,
                    brand_id: Some(brand_id),
                    trigger: r.trigger_col.unwrap_or_default(),
                    action: r.action.unwrap_or_default(),
                    notification_email: r.notification_email,
                    webhook_url: r.endpoint,
                    enabled: r.enabled.unwrap_or(0) == 1,
                })
                .collect();
            Json(body).into_response()
        }
        Err(e) => {
            tracing::error!(error = ?e, "list rules failed");
            internal_error("Database error.")
        }
    }
}

/// `POST /api/brands/:brand_id/rules` — create a new automation rule.
#[handler]
pub async fn create(
    _auth: ApiAuth,
    Path(brand_id): Path<OpaqueId<Brand>>,
    Json(body): Json<CreateRuleBody>,
    Data(state): Data<&AppState>,
) -> Response {
    let brand_raw = brand_id.raw();
    match brands::find_brand(&state.db, brand_raw).await {
        Ok(Some(_)) => {}
        Ok(None) => return not_found("Brand does not exist."),
        Err(e) => {
            tracing::error!(error = ?e, "find_brand failed");
            return internal_error("Database error.");
        }
    }

    if body.trigger.trim().is_empty() || body.action.trim().is_empty() {
        return bad_request("Trigger and action are required.");
    }

    let input = RuleInput {
        brand_id: brand_raw,
        app_id: brand_raw,
        trigger: body.trigger.trim().to_string(),
        action: body.action.trim().to_string(),
        notification_email: body.notification_email,
        webhook_url: body.webhook_url,
        list_id: body.list_id,
        ares_id: body.ares_id,
    };

    match rules::save(&state.db, input).await {
        Ok(new_id) => {
            let resp = RuleResponse {
                id: new_id,
                brand_id: Some(brand_id),
                trigger: body.trigger,
                action: body.action,
                notification_email: None,
                webhook_url: None,
                enabled: true,
            };
            Json(resp).into_response()
        }
        Err(e) => {
            tracing::error!(error = ?e, "create rule failed");
            internal_error("Failed to create rule.")
        }
    }
}

/// `POST /api/rules/:rule_id/toggle` — toggle rule on or off.
#[handler]
pub async fn toggle(
    _auth: ApiAuth,
    Path(rule_id): Path<i32>,
    Data(state): Data<&AppState>,
) -> Response {
    match rules::toggle(&state.db, rule_id).await {
        Ok(is_enabled) => Json(serde_json::json!({ "enabled": is_enabled })).into_response(),
        Err(crate::utils::errors::AppError::NotFound(_)) => not_found("Rule does not exist."),
        Err(e) => {
            tracing::error!(error = ?e, "toggle rule failed");
            internal_error("Failed to toggle rule.")
        }
    }
}

/// `DELETE /api/rules/:rule_id` — delete rule.
#[handler]
pub async fn delete(
    _auth: ApiAuth,
    Path(rule_id): Path<i32>,
    Data(state): Data<&AppState>,
) -> Response {
    match rules::delete(&state.db, rule_id).await {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "delete rule failed");
            internal_error("Failed to delete rule.")
        }
    }
}
