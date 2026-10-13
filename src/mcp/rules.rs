use rmcp::{
    handler::server::wrapper::{Json as McpJson, Parameters},
    schemars::JsonSchema,
    ErrorData,
};
use serde::{Deserialize, Serialize};

use crate::routes::api::{
    rules::{CreateRuleBody, RuleResponse},
    subscribers::AckResponse,
};
use crate::services::rules as rules_svc;
use crate::utils::opaque_id::{Brand, OpaqueId};
use crate::AppState;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct CreateRuleParams {
    #[schemars(description = "Opaque identifier of the brand")]
    pub brand_id: OpaqueId<Brand>,
    #[serde(flatten)]
    pub body: CreateRuleBody,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct RuleParam {
    #[schemars(description = "Database ID of the rule")]
    pub rule_id: i32,
}

pub async fn list_rules(
    state: &AppState,
    Parameters(params): Parameters<crate::mcp::brands::BrandParam>,
) -> Result<McpJson<Vec<RuleResponse>>, ErrorData> {
    let rows = rules_svc::list_for_brand(&state.db, params.brand_id.raw())
        .await
        .map_err(|e| ErrorData::internal_error(format!("list rules error: {e}"), None))?;

    let items = rows
        .into_iter()
        .map(|r| RuleResponse {
            id: r.id,
            brand_id: Some(params.brand_id),
            trigger: r.trigger_col.unwrap_or_default(),
            action: r.action.unwrap_or_default(),
            notification_email: r.notification_email,
            webhook_url: r.endpoint,
            enabled: r.enabled.unwrap_or(0) == 1,
        })
        .collect();

    Ok(McpJson(items))
}

pub async fn create_rule(
    state: &AppState,
    Parameters(params): Parameters<CreateRuleParams>,
) -> Result<McpJson<RuleResponse>, ErrorData> {
    let brand_raw = params.brand_id.raw();
    let body = params.body;
    if body.trigger.trim().is_empty() || body.action.trim().is_empty() {
        return Err(ErrorData::invalid_params("Trigger and action are required", None));
    }
    let input = rules_svc::RuleInput {
        brand_id: brand_raw,
        app_id: brand_raw,
        trigger: body.trigger.trim().to_string(),
        action: body.action.trim().to_string(),
        notification_email: body.notification_email,
        webhook_url: body.webhook_url,
        list_id: body.list_id,
        ares_id: body.ares_id,
    };
    let new_id = rules_svc::save(&state.db, input)
        .await
        .map_err(|e| ErrorData::internal_error(format!("create rule error: {e}"), None))?;

    Ok(McpJson(RuleResponse {
        id: new_id,
        brand_id: Some(params.brand_id),
        trigger: body.trigger,
        action: body.action,
        notification_email: None,
        webhook_url: None,
        enabled: true,
    }))
}

pub async fn toggle_rule(
    state: &AppState,
    Parameters(params): Parameters<RuleParam>,
) -> Result<McpJson<AckResponse>, ErrorData> {
    let is_enabled = rules_svc::toggle(&state.db, params.rule_id)
        .await
        .map_err(|e| ErrorData::internal_error(format!("toggle rule error: {e}"), None))?;
    Ok(McpJson(AckResponse { ok: is_enabled }))
}

pub async fn delete_rule(
    state: &AppState,
    Parameters(params): Parameters<RuleParam>,
) -> Result<McpJson<AckResponse>, ErrorData> {
    rules_svc::delete(&state.db, params.rule_id)
        .await
        .map_err(|e| ErrorData::internal_error(format!("delete rule error: {e}"), None))?;
    Ok(McpJson(AckResponse { ok: true }))
}
