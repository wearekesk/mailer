use rmcp::{
    handler::server::wrapper::{Json as McpJson, Parameters},
    schemars::JsonSchema,
    ErrorData,
};
use serde::{Deserialize, Serialize};

use crate::routes::api::{
    subscribers::AckResponse,
    templates::{CreateTemplateBody, TemplateResponse, UpdateTemplateBody},
};
use crate::services::templates as templates_svc;
use crate::utils::opaque_id::{Brand, OpaqueId, Template};
use crate::AppState;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct CreateTemplateParams {
    #[schemars(description = "Opaque identifier of the brand")]
    pub brand_id: OpaqueId<Brand>,
    #[serde(flatten)]
    pub body: CreateTemplateBody,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct TemplateParam {
    #[schemars(description = "Opaque identifier of the template")]
    pub template_id: OpaqueId<Template>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct UpdateTemplateParams {
    #[schemars(description = "Opaque identifier of the template")]
    pub template_id: OpaqueId<Template>,
    #[serde(flatten)]
    pub body: UpdateTemplateBody,
}

pub async fn list_templates(
    state: &AppState,
    Parameters(params): Parameters<crate::mcp::brands::BrandParam>,
) -> Result<McpJson<Vec<TemplateResponse>>, ErrorData> {
    let brand_raw = params.brand_id.raw();
    let rows = templates_svc::list_for_brand(&state.db, brand_raw)
        .await
        .map_err(|e| ErrorData::internal_error(format!("list templates error: {e}"), None))?;

    let items = rows
        .into_iter()
        .map(|t| TemplateResponse {
            id: OpaqueId::new(t.id),
            brand_id: t.app.map(OpaqueId::new),
            name: t.template_name.unwrap_or_default(),
            from_name: t.from_name,
            from_email: t.from_email,
            reply_to: t.reply_to,
            html_text: t.html_text,
            plain_text: t.plain_text,
        })
        .collect();

    Ok(McpJson(items))
}

pub async fn create_template(
    state: &AppState,
    Parameters(params): Parameters<CreateTemplateParams>,
) -> Result<McpJson<TemplateResponse>, ErrorData> {
    let brand_raw = params.brand_id.raw();
    let body = params.body;
    if body.name.trim().is_empty() {
        return Err(ErrorData::invalid_params("Template name cannot be empty", None));
    }
    let input = templates_svc::TemplateInput {
        template_name: body.name.trim().to_string(),
        html_text: body.html_text,
        plain_text: body.plain_text,
        from_name: body.from_name,
        from_email: body.from_email,
        reply_to: body.reply_to,
    };
    let new_id = templates_svc::create(&state.db, 1, brand_raw, input)
        .await
        .map_err(|e| ErrorData::internal_error(format!("create template error: {e}"), None))?;

    Ok(McpJson(TemplateResponse {
        id: OpaqueId::new(new_id),
        brand_id: Some(params.brand_id),
        name: body.name,
        from_name: None,
        from_email: None,
        reply_to: None,
        html_text: None,
        plain_text: None,
    }))
}

pub async fn update_template(
    state: &AppState,
    Parameters(params): Parameters<UpdateTemplateParams>,
) -> Result<McpJson<AckResponse>, ErrorData> {
    let raw = params.template_id.raw();
    let existing = match templates_svc::find(&state.db, raw).await {
        Ok(Some(t)) => t,
        Ok(None) => return Err(ErrorData::invalid_params("Template does not exist", None)),
        Err(e) => return Err(ErrorData::internal_error(format!("find template error: {e}"), None)),
    };
    let body = params.body;
    let input = templates_svc::TemplateInput {
        template_name: body.name.unwrap_or_else(|| existing.template_name.clone().unwrap_or_default()),
        html_text: body.html_text.or(existing.html_text),
        plain_text: body.plain_text.or(existing.plain_text),
        from_name: body.from_name.or(existing.from_name),
        from_email: body.from_email.or(existing.from_email),
        reply_to: body.reply_to.or(existing.reply_to),
    };
    templates_svc::update(&state.db, raw, input)
        .await
        .map_err(|e| ErrorData::internal_error(format!("update template error: {e}"), None))?;
    Ok(McpJson(AckResponse { ok: true }))
}

pub async fn delete_template(
    state: &AppState,
    Parameters(params): Parameters<TemplateParam>,
) -> Result<McpJson<AckResponse>, ErrorData> {
    templates_svc::delete(&state.db, params.template_id.raw())
        .await
        .map_err(|e| ErrorData::internal_error(format!("delete template error: {e}"), None))?;
    Ok(McpJson(AckResponse { ok: true }))
}
