use rmcp::{
    handler::server::wrapper::{Json as McpJson, Parameters},
    schemars::JsonSchema,
    ErrorData,
};
use serde::{Deserialize, Serialize};

use crate::routes::api::{
    settings::{BlockedDomainResponse, SuppressionResponse},
    subscribers::AckResponse,
};
use crate::services::settings as settings_svc;
use crate::utils::opaque_id::{Brand, OpaqueId};
use crate::AppState;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AddSuppressionParams {
    #[schemars(description = "Opaque identifier of the brand")]
    pub brand_id: OpaqueId<Brand>,
    #[schemars(description = "Email address to suppress")]
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SuppressionParam {
    #[schemars(description = "Suppression entry ID")]
    pub id: i32,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AddBlockedDomainParams {
    #[schemars(description = "Opaque identifier of the brand")]
    pub brand_id: OpaqueId<Brand>,
    #[schemars(description = "Domain name to block (e.g. spammer.com)")]
    pub domain: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct BlockedDomainParam {
    #[schemars(description = "Blocked domain entry ID")]
    pub id: i32,
}

pub async fn list_suppression(
    state: &AppState,
    Parameters(params): Parameters<crate::mcp::brands::BrandParam>,
) -> Result<McpJson<Vec<SuppressionResponse>>, ErrorData> {
    let (rows, _) = settings_svc::list_suppression(&state.db, params.brand_id.raw(), 0, 100)
        .await
        .map_err(|e| ErrorData::internal_error(format!("list suppression error: {e}"), None))?;

    let items = rows
        .into_iter()
        .map(|s| SuppressionResponse {
            id: s.id,
            email: s.email,
            timestamp: Some(s.timestamp),
        })
        .collect();

    Ok(McpJson(items))
}

pub async fn add_suppression(
    state: &AppState,
    Parameters(params): Parameters<AddSuppressionParams>,
) -> Result<McpJson<AckResponse>, ErrorData> {
    if params.email.trim().is_empty() {
        return Err(ErrorData::invalid_params("Email cannot be empty", None));
    }
    settings_svc::add_suppression(&state.db, params.brand_id.raw(), &params.email)
        .await
        .map_err(|e| ErrorData::internal_error(format!("add suppression error: {e}"), None))?;
    Ok(McpJson(AckResponse { ok: true }))
}

pub async fn delete_suppression(
    state: &AppState,
    Parameters(params): Parameters<SuppressionParam>,
) -> Result<McpJson<AckResponse>, ErrorData> {
    settings_svc::delete_suppression(&state.db, params.id)
        .await
        .map_err(|e| ErrorData::internal_error(format!("delete suppression error: {e}"), None))?;
    Ok(McpJson(AckResponse { ok: true }))
}

pub async fn list_blocked_domains(
    state: &AppState,
    Parameters(params): Parameters<crate::mcp::brands::BrandParam>,
) -> Result<McpJson<Vec<BlockedDomainResponse>>, ErrorData> {
    let (rows, _) = settings_svc::list_blocked_domains(&state.db, params.brand_id.raw(), 0, 100)
        .await
        .map_err(|e| ErrorData::internal_error(format!("list blocked domains error: {e}"), None))?;

    let items = rows
        .into_iter()
        .map(|d| BlockedDomainResponse {
            id: d.id,
            domain: d.domain,
            timestamp: Some(d.timestamp),
        })
        .collect();

    Ok(McpJson(items))
}

pub async fn add_blocked_domain(
    state: &AppState,
    Parameters(params): Parameters<AddBlockedDomainParams>,
) -> Result<McpJson<AckResponse>, ErrorData> {
    if params.domain.trim().is_empty() {
        return Err(ErrorData::invalid_params("Domain cannot be empty", None));
    }
    settings_svc::add_blocked_domain(&state.db, params.brand_id.raw(), &params.domain)
        .await
        .map_err(|e| ErrorData::internal_error(format!("add blocked domain error: {e}"), None))?;
    Ok(McpJson(AckResponse { ok: true }))
}

pub async fn delete_blocked_domain(
    state: &AppState,
    Parameters(params): Parameters<BlockedDomainParam>,
) -> Result<McpJson<AckResponse>, ErrorData> {
    settings_svc::delete_blocked_domain(&state.db, params.id)
        .await
        .map_err(|e| ErrorData::internal_error(format!("delete blocked domain error: {e}"), None))?;
    Ok(McpJson(AckResponse { ok: true }))
}
