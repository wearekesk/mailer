use rmcp::{
    handler::server::wrapper::{Json as McpJson, Parameters},
    schemars::JsonSchema,
    ErrorData,
};
use serde::{Deserialize, Serialize};

use crate::routes::api::{
    brands::{BrandResponse, CreateBrandBody, UpdateBrandBody},
    subscribers::AckResponse,
};
use crate::services::brands as brands_svc;
use crate::utils::opaque_id::{Brand, OpaqueId};
use crate::AppState;

#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
pub struct EmptyParams {}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct BrandParam {
    #[schemars(description = "Opaque identifier or base64 token of the brand")]
    pub brand_id: OpaqueId<Brand>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct CreateBrandParams {
    #[serde(flatten)]
    pub body: CreateBrandBody,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct UpdateBrandParams {
    #[schemars(description = "Opaque identifier or base64 token of the brand")]
    pub brand_id: OpaqueId<Brand>,
    #[serde(flatten)]
    pub body: UpdateBrandBody,
}

pub async fn list_brands(
    state: &AppState,
    _params: Parameters<EmptyParams>,
) -> Result<McpJson<Vec<BrandResponse>>, ErrorData> {
    let rows = brands_svc::list_brands(&state.db)
        .await
        .map_err(|e| ErrorData::internal_error(format!("list_brands failed: {e}"), None))?;

    let items = rows
        .into_iter()
        .map(|b| BrandResponse {
            id: OpaqueId::new(b.id),
            name: b.name,
            from_email: b.from_email,
            list_count: Some(b.list_count),
            subscriber_count: Some(b.subscriber_count),
        })
        .collect();

    Ok(McpJson(items))
}

pub async fn get_brand(
    state: &AppState,
    Parameters(params): Parameters<BrandParam>,
) -> Result<McpJson<BrandResponse>, ErrorData> {
    match brands_svc::find_brand(&state.db, params.brand_id.raw()).await {
        Ok(Some(b)) => Ok(McpJson(BrandResponse {
            id: OpaqueId::new(b.id),
            name: b.app_name.unwrap_or_default(),
            from_email: b.from_email,
            list_count: None,
            subscriber_count: None,
        })),
        Ok(None) => Err(ErrorData::invalid_params("Brand does not exist", None)),
        Err(e) => Err(ErrorData::internal_error(format!("find_brand error: {e}"), None)),
    }
}

pub async fn create_brand(
    state: &AppState,
    Parameters(params): Parameters<CreateBrandParams>,
) -> Result<McpJson<BrandResponse>, ErrorData> {
    let body = params.body;
    if body.name.trim().is_empty() {
        return Err(ErrorData::invalid_params("Brand name cannot be empty", None));
    }
    let input = brands_svc::BrandInput {
        app_name: body.name.trim().to_string(),
        from_name: body.from_name,
        from_email: body.from_email,
        reply_to: body.reply_to,
        smtp_host: body.smtp_host,
        smtp_port: body.smtp_port,
        smtp_ssl: body.smtp_ssl,
        smtp_username: body.smtp_username,
        smtp_password: body.smtp_password,
        query_string: body.query_string,
        test_email_prefix: body.test_email_prefix,
        custom_domain: body.custom_domain,
        opens_tracking: 1,
        links_tracking: 1,
        ..Default::default()
    };
    let new_id = brands_svc::create_brand(&state.db, 1, input)
        .await
        .map_err(|e| ErrorData::internal_error(format!("create_brand error: {e}"), None))?;

    Ok(McpJson(BrandResponse {
        id: OpaqueId::new(new_id),
        name: body.name,
        from_email: None,
        list_count: Some(0),
        subscriber_count: Some(0),
    }))
}

pub async fn update_brand(
    state: &AppState,
    Parameters(params): Parameters<UpdateBrandParams>,
) -> Result<McpJson<AckResponse>, ErrorData> {
    let raw = params.brand_id.raw();
    let existing = match brands_svc::find_brand(&state.db, raw).await {
        Ok(Some(b)) => b,
        Ok(None) => return Err(ErrorData::invalid_params("Brand does not exist", None)),
        Err(e) => return Err(ErrorData::internal_error(format!("find_brand error: {e}"), None)),
    };
    let body = params.body;
    let input = brands_svc::BrandInput {
        app_name: body.name.unwrap_or_else(|| existing.app_name.clone().unwrap_or_default()),
        from_name: body.from_name.or(existing.from_name),
        from_email: body.from_email.or(existing.from_email),
        reply_to: body.reply_to.or(existing.reply_to),
        smtp_host: body.smtp_host.or(existing.smtp_host),
        smtp_port: body.smtp_port.or(existing.smtp_port),
        smtp_ssl: body.smtp_ssl.or(existing.smtp_ssl),
        smtp_username: body.smtp_username.or(existing.smtp_username),
        smtp_password: body.smtp_password.or(existing.smtp_password),
        query_string: body.query_string.or(existing.query_string),
        test_email_prefix: body.test_email_prefix.or(existing.test_email_prefix),
        custom_domain: body.custom_domain.or(existing.custom_domain),
        opens_tracking: existing.opens_tracking.unwrap_or(1),
        links_tracking: existing.links_tracking.unwrap_or(1),
        ..Default::default()
    };
    brands_svc::update_brand(&state.db, raw, input)
        .await
        .map_err(|e| ErrorData::internal_error(format!("update_brand error: {e}"), None))?;
    Ok(McpJson(AckResponse { ok: true }))
}

pub async fn delete_brand(
    state: &AppState,
    Parameters(params): Parameters<BrandParam>,
) -> Result<McpJson<AckResponse>, ErrorData> {
    brands_svc::delete_brand(&state.db, params.brand_id.raw())
        .await
        .map_err(|e| ErrorData::internal_error(format!("delete_brand error: {e}"), None))?;
    Ok(McpJson(AckResponse { ok: true }))
}
