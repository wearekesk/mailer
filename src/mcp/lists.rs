use rmcp::{
    handler::server::wrapper::{Json as McpJson, Parameters},
    schemars::JsonSchema,
    ErrorData,
};
use serde::{Deserialize, Serialize};

use crate::routes::api::{
    lists::{CreateListBody, ListResponse, UpdateListBody},
    subscribers::AckResponse,
};
use crate::services::{
    brands as brands_svc, lists as lists_svc, subscribers as subscribers_svc,
};
use crate::utils::opaque_id::{Brand, List, OpaqueId};
use crate::AppState;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ListForBrandParams {
    #[schemars(description = "Opaque identifier or base64 token of the brand")]
    pub brand_id: OpaqueId<Brand>,
    #[serde(default)]
    #[schemars(description = "Whether to include hidden lists")]
    pub include_hidden: bool,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ListParam {
    #[schemars(description = "Opaque identifier of the target mailing list")]
    pub list_id: OpaqueId<List>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct CreateListParams {
    #[schemars(description = "Opaque identifier or base64 token of the brand")]
    pub brand_id: OpaqueId<Brand>,
    #[serde(flatten)]
    pub body: CreateListBody,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct UpdateListParams {
    #[schemars(description = "Opaque identifier of the target mailing list")]
    pub list_id: OpaqueId<List>,
    #[serde(flatten)]
    pub body: UpdateListBody,
}

pub async fn list_lists_for_brand(
    state: &AppState,
    Parameters(params): Parameters<ListForBrandParams>,
) -> Result<McpJson<Vec<ListResponse>>, ErrorData> {
    let brand_raw = params.brand_id.raw();
    match brands_svc::find_brand(&state.db, brand_raw).await {
        Ok(Some(_)) => {}
        Ok(None) => return Err(ErrorData::invalid_params("Brand does not exist", None)),
        Err(e) => return Err(ErrorData::internal_error(format!("find_brand error: {e}"), None)),
    }

    let rows = lists_svc::list_brand_lists(&state.db, brand_raw, params.include_hidden)
        .await
        .map_err(|e| ErrorData::internal_error(format!("list_brand_lists error: {e}"), None))?;

    let items = rows
        .into_iter()
        .map(|l| ListResponse {
            id: OpaqueId::new(l.id),
            name: l.name,
            subscriber_count: l.subscriber_count,
            hide: l.hide,
        })
        .collect();

    Ok(McpJson(items))
}

pub async fn get_list(
    state: &AppState,
    Parameters(params): Parameters<ListParam>,
) -> Result<McpJson<ListResponse>, ErrorData> {
    let raw = params.list_id.raw();
    match lists_svc::find_list(&state.db, raw).await {
        Ok(Some(l)) => {
            let count = subscribers_svc::active_count(&state.db, raw).await.unwrap_or(0);
            Ok(McpJson(ListResponse {
                id: OpaqueId::new(l.id),
                name: l.name.unwrap_or_default(),
                subscriber_count: count,
                hide: l.hide.unwrap_or(0) == 1,
            }))
        }
        Ok(None) => Err(ErrorData::invalid_params("List does not exist", None)),
        Err(e) => Err(ErrorData::internal_error(format!("find_list error: {e}"), None)),
    }
}

pub async fn create_list(
    state: &AppState,
    Parameters(params): Parameters<CreateListParams>,
) -> Result<McpJson<ListResponse>, ErrorData> {
    let brand_raw = params.brand_id.raw();
    match brands_svc::find_brand(&state.db, brand_raw).await {
        Ok(Some(_)) => {}
        Ok(None) => return Err(ErrorData::invalid_params("Brand does not exist", None)),
        Err(e) => return Err(ErrorData::internal_error(format!("find_brand error: {e}"), None)),
    }
    let body = params.body;
    if body.name.trim().is_empty() {
        return Err(ErrorData::invalid_params("List name cannot be empty", None));
    }
    let input = lists_svc::ListInput {
        name: body.name.trim().to_string(),
        opt_in: body.opt_in.unwrap_or(1),
        subscribed_url: body.subscribed_url,
        confirm_url: body.confirm_url,
        already_subscribed_url: body.already_subscribed_url,
        unsubscribed_url: body.unsubscribed_url,
        thankyou: body.thankyou.unwrap_or(0),
        thankyou_subject: body.thankyou_subject,
        thankyou_message: body.thankyou_message,
        goodbye: body.goodbye.unwrap_or(0),
        goodbye_subject: body.goodbye_subject,
        goodbye_message: body.goodbye_message,
        confirmation_subject: body.confirmation_subject,
        confirmation_email: body.confirmation_email,
        ..Default::default()
    };
    let new_id = lists_svc::create_list(&state.db, 1, brand_raw, input)
        .await
        .map_err(|e| ErrorData::internal_error(format!("create_list error: {e}"), None))?;

    Ok(McpJson(ListResponse {
        id: OpaqueId::new(new_id),
        name: body.name,
        subscriber_count: 0,
        hide: false,
    }))
}

pub async fn update_list(
    state: &AppState,
    Parameters(params): Parameters<UpdateListParams>,
) -> Result<McpJson<AckResponse>, ErrorData> {
    let raw = params.list_id.raw();
    let existing = match lists_svc::find_list(&state.db, raw).await {
        Ok(Some(l)) => l,
        Ok(None) => return Err(ErrorData::invalid_params("List does not exist", None)),
        Err(e) => return Err(ErrorData::internal_error(format!("find_list error: {e}"), None)),
    };
    let body = params.body;
    let input = lists_svc::ListInput {
        name: body.name.unwrap_or_else(|| existing.name.clone().unwrap_or_default()),
        opt_in: body.opt_in.unwrap_or_else(|| existing.opt_in.unwrap_or(1)),
        subscribed_url: body.subscribed_url.or(existing.subscribed_url),
        confirm_url: body.confirm_url.or(existing.confirm_url),
        already_subscribed_url: body.already_subscribed_url.or(existing.already_subscribed_url),
        unsubscribed_url: body.unsubscribed_url.or(existing.unsubscribed_url),
        thankyou: body.thankyou.unwrap_or_else(|| existing.thankyou.unwrap_or(0)),
        thankyou_subject: body.thankyou_subject.or(existing.thankyou_subject),
        thankyou_message: body.thankyou_message.or(existing.thankyou_message),
        goodbye: body.goodbye.unwrap_or_else(|| existing.goodbye.unwrap_or(0)),
        goodbye_subject: body.goodbye_subject.or(existing.goodbye_subject),
        goodbye_message: body.goodbye_message.or(existing.goodbye_message),
        confirmation_subject: body.confirmation_subject.or(existing.confirmation_subject),
        confirmation_email: body.confirmation_email.or(existing.confirmation_email),
        ..Default::default()
    };
    lists_svc::update_list(&state.db, raw, input)
        .await
        .map_err(|e| ErrorData::internal_error(format!("update_list error: {e}"), None))?;
    Ok(McpJson(AckResponse { ok: true }))
}

pub async fn delete_list(
    state: &AppState,
    Parameters(params): Parameters<ListParam>,
) -> Result<McpJson<AckResponse>, ErrorData> {
    lists_svc::delete_list(&state.db, params.list_id.raw())
        .await
        .map_err(|e| ErrorData::internal_error(format!("delete_list error: {e}"), None))?;
    Ok(McpJson(AckResponse { ok: true }))
}
