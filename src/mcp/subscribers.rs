use rmcp::{
    handler::server::wrapper::{Json as McpJson, Parameters},
    schemars::JsonSchema,
    ErrorData,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

use crate::database::entities::subscribers as db_subscribers;
use crate::routes::api::subscribers::{
    nonempty, AckResponse, CountResponse, StatusResponse, SubscribeBody, SubscribeResponse,
};
use crate::services::{lists as lists_svc, subscribers as subscribers_svc};
use crate::utils::opaque_id::{List, OpaqueId};
use crate::AppState;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SubscribeParams {
    #[schemars(description = "Opaque identifier of the target mailing list")]
    pub list_id: OpaqueId<List>,
    #[serde(flatten)]
    pub body: SubscribeBody,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SubscriberListEmailParams {
    #[schemars(description = "Opaque identifier of the mailing list")]
    pub list_id: OpaqueId<List>,
    #[schemars(description = "Email address of the subscriber")]
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SubscriberCountParams {
    #[schemars(description = "Opaque identifier of the mailing list")]
    pub list_id: OpaqueId<List>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct QuerySubscribersParams {
    #[schemars(description = "Database ID or list ID")]
    pub list_id: i32,
    #[schemars(description = "Maximum number of subscribers to return (default: 50, max: 500)")]
    pub limit: Option<u64>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SubscriberItem {
    pub id: i32,
    pub email: String,
    pub name: Option<String>,
    pub unsubscribed: i16,
    pub bounced: i16,
}

pub async fn subscribe(
    state: &AppState,
    Parameters(params): Parameters<SubscribeParams>,
) -> Result<McpJson<SubscribeResponse>, ErrorData> {
    let body = params.body;
    if body.email.trim().is_empty() {
        return Err(ErrorData::invalid_params("Email is required", None));
    }
    if body.hp.as_deref().map(|s| !s.is_empty()).unwrap_or(false)
        || !subscribers_svc::is_valid_email(&body.email)
    {
        return Err(ErrorData::invalid_params("Invalid email address", None));
    }

    let subscribe_params = subscribers_svc::SubscribeParams {
        email: body.email,
        name: nonempty(body.name),
        list_id: params.list_id.raw(),
        ip: nonempty(body.ipaddress),
        country: nonempty(body.country),
        referrer: nonempty(body.referrer),
        notes: nonempty(body.notes),
        gdpr: body.gdpr,
        silent: body.silent,
    };

    let outcome = subscribers_svc::subscribe(&state.db, subscribe_params)
        .await
        .map_err(|e| ErrorData::internal_error(format!("subscribe failed: {e}"), None))?;

    Ok(McpJson(SubscribeResponse { outcome }))
}

pub async fn get_subscriber_status(
    state: &AppState,
    Parameters(params): Parameters<SubscriberListEmailParams>,
) -> Result<McpJson<StatusResponse>, ErrorData> {
    let status = subscribers_svc::get_status(&state.db, &params.email, params.list_id.raw())
        .await
        .map_err(|e| ErrorData::internal_error(format!("get_status failed: {e}"), None))?;

    Ok(McpJson(StatusResponse { status }))
}

pub async fn get_subscriber_count(
    state: &AppState,
    Parameters(params): Parameters<SubscriberCountParams>,
) -> Result<McpJson<CountResponse>, ErrorData> {
    let raw = params.list_id.raw();
    match lists_svc::find_list(&state.db, raw).await {
        Ok(Some(_)) => {}
        Ok(None) => return Err(ErrorData::invalid_params("List does not exist", None)),
        Err(e) => return Err(ErrorData::internal_error(format!("find_list error: {e}"), None)),
    }

    let active = subscribers_svc::active_count(&state.db, raw)
        .await
        .map_err(|e| ErrorData::internal_error(format!("active_count failed: {e}"), None))?;

    Ok(McpJson(CountResponse {
        list_id: params.list_id,
        active,
    }))
}

pub async fn unsubscribe(
    state: &AppState,
    Parameters(params): Parameters<SubscriberListEmailParams>,
) -> Result<McpJson<AckResponse>, ErrorData> {
    subscribers_svc::unsubscribe(&state.db, &params.email, params.list_id.raw())
        .await
        .map_err(|e| ErrorData::internal_error(format!("unsubscribe failed: {e}"), None))?;

    Ok(McpJson(AckResponse { ok: true }))
}

pub async fn delete_subscriber(
    state: &AppState,
    Parameters(params): Parameters<SubscriberListEmailParams>,
) -> Result<McpJson<AckResponse>, ErrorData> {
    let raw = params.list_id.raw();
    match lists_svc::find_list(&state.db, raw).await {
        Ok(Some(_)) => {}
        Ok(None) => return Err(ErrorData::invalid_params("List does not exist", None)),
        Err(e) => return Err(ErrorData::internal_error(format!("find_list error: {e}"), None)),
    }

    subscribers_svc::delete(&state.db, &params.email, raw)
        .await
        .map_err(|e| ErrorData::internal_error(format!("delete subscriber failed: {e}"), None))?;

    Ok(McpJson(AckResponse { ok: true }))
}

pub async fn query_subscribers(
    state: &AppState,
    Parameters(params): Parameters<QuerySubscribersParams>,
) -> Result<McpJson<Vec<SubscriberItem>>, ErrorData> {
    let limit = params.limit.unwrap_or(50).min(500);
    let rows = db_subscribers::Entity::find()
        .filter(db_subscribers::Column::List.eq(params.list_id))
        .order_by_desc(db_subscribers::Column::Id)
        .all(&state.db)
        .await
        .map_err(|e| ErrorData::internal_error(format!("db error: {e}"), None))?;

    let items = rows
        .into_iter()
        .take(limit as usize)
        .map(|r| SubscriberItem {
            id: r.id,
            email: r.email.unwrap_or_default(),
            name: r.name,
            unsubscribed: r.unsubscribed.unwrap_or(0),
            bounced: r.bounced.unwrap_or(0),
        })
        .collect();

    Ok(McpJson(items))
}
