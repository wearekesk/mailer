use poem::{
    handler,
    web::{Data, Json, Path, Query},
    IntoResponse, Response,
};
use schemars::JsonSchema;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

use crate::database::entities::subscribers;
use crate::routes::api::{auth::ApiAuth, bad_request, internal_error, not_found};
use crate::services::{lists, subscribers as subscribers_svc};
use crate::utils::opaque_id::{List, OpaqueId};
use crate::AppState;

// ── Request bodies ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
pub struct SubscribeBody {
    pub email: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub ipaddress: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub referrer: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub gdpr: bool,
    #[serde(default)]
    pub silent: bool,
    /// Honeypot field; non-empty means the request looks bot-like.
    #[serde(default)]
    pub hp: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
pub struct QuerySubscribersQuery {
    #[serde(default)]
    pub page: Option<u64>,
    #[serde(default)]
    pub limit: Option<u64>,
}

// ── Response bodies ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SubscribeResponse {
    pub outcome: subscribers_svc::SubscribeOutcome,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct StatusResponse {
    pub status: subscribers_svc::SubscriptionStatus,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CountResponse {
    pub list_id: OpaqueId<List>,
    pub active: i64,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AckResponse {
    pub ok: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SubscriberItemResponse {
    pub id: i32,
    pub email: String,
    pub name: Option<String>,
    pub unsubscribed: i16,
    pub bounced: i16,
    pub complaint: i16,
    pub confirmed: i16,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// `GET /api/lists/:list_id/subscribers` — list subscribers in list with pagination.
#[handler]
pub async fn list(
    _auth: ApiAuth,
    Path(list_id): Path<OpaqueId<List>>,
    Query(q): Query<QuerySubscribersQuery>,
    Data(state): Data<&AppState>,
) -> Response {
    let raw = list_id.raw();
    match lists::find_list(&state.db, raw).await {
        Ok(Some(_)) => {}
        Ok(None) => return not_found("List does not exist"),
        Err(e) => {
            tracing::error!(error = ?e, "find_list failed");
            return internal_error("Database error.");
        }
    }

    let page = q.page.unwrap_or(0);
    let limit = q.limit.unwrap_or(50).min(500);

    let paginator = subscribers::Entity::find()
        .filter(subscribers::Column::List.eq(raw))
        .order_by_desc(subscribers::Column::Id)
        .paginate(&state.db, limit);

    match paginator.fetch_page(page).await {
        Ok(rows) => {
            let body: Vec<SubscriberItemResponse> = rows
                .into_iter()
                .map(|r| SubscriberItemResponse {
                    id: r.id,
                    email: r.email.unwrap_or_default(),
                    name: r.name,
                    unsubscribed: r.unsubscribed.unwrap_or(0),
                    bounced: r.bounced.unwrap_or(0),
                    complaint: r.complaint.unwrap_or(0),
                    confirmed: r.confirmed.unwrap_or(0),
                })
                .collect();
            Json(body).into_response()
        }
        Err(e) => {
            tracing::error!(error = ?e, "fetch subscribers page failed");
            internal_error("Database error.")
        }
    }
}

/// `POST /api/lists/:list_id/subscribers`
#[handler]
pub async fn subscribe(
    _auth: ApiAuth,
    Path(list_id): Path<OpaqueId<List>>,
    Json(body): Json<SubscribeBody>,
    Data(state): Data<&AppState>,
) -> Response {
    if body.email.trim().is_empty() {
        return bad_request("Email is required.");
    }
    if body.hp.as_deref().map(|s| !s.is_empty()).unwrap_or(false)
        || !subscribers_svc::is_valid_email(&body.email)
    {
        return bad_request("Invalid email address.");
    }

    let params = subscribers_svc::SubscribeParams {
        email: body.email,
        name: nonempty(body.name),
        list_id: list_id.raw(),
        ip: nonempty(body.ipaddress),
        country: nonempty(body.country),
        referrer: nonempty(body.referrer),
        notes: nonempty(body.notes),
        gdpr: body.gdpr,
        silent: body.silent,
    };

    match subscribers_svc::subscribe(&state.db, params).await {
        Ok(outcome) => Json(SubscribeResponse { outcome }).into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "subscribe failed");
            internal_error("Subscription failed.")
        }
    }
}

/// `POST /api/lists/:list_id/subscribers/:email/unsubscribe`
#[handler]
pub async fn unsubscribe(
    _auth: ApiAuth,
    Path((list_id, email)): Path<(OpaqueId<List>, String)>,
    Data(state): Data<&AppState>,
) -> Response {
    match subscribers_svc::unsubscribe(&state.db, &email, list_id.raw()).await {
        Ok(()) => Json(AckResponse { ok: true }).into_response(),
        Err(crate::utils::errors::AppError::NotFound(msg)) => not_found(msg),
        Err(e) => {
            tracing::error!(error = ?e, "unsubscribe failed");
            internal_error("Unsubscribe failed.")
        }
    }
}

/// `GET /api/lists/:list_id/subscribers/:email/status`
#[handler]
pub async fn status(
    _auth: ApiAuth,
    Path((list_id, email)): Path<(OpaqueId<List>, String)>,
    Data(state): Data<&AppState>,
) -> Response {
    match subscribers_svc::get_status(&state.db, &email, list_id.raw()).await {
        Ok(status) => Json(StatusResponse { status }).into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "get_status failed");
            internal_error("Status lookup failed.")
        }
    }
}

/// `GET /api/lists/:list_id/subscribers/count`
#[handler]
pub async fn count(
    _auth: ApiAuth,
    Path(list_id): Path<OpaqueId<List>>,
    Data(state): Data<&AppState>,
) -> Response {
    let raw = list_id.raw();
    match lists::find_list(&state.db, raw).await {
        Ok(Some(_)) => {}
        Ok(None) => return not_found("List does not exist"),
        Err(e) => {
            tracing::error!(error = ?e, "find_list failed");
            return internal_error("Database error.");
        }
    }

    match subscribers_svc::active_count(&state.db, raw).await {
        Ok(active) => Json(CountResponse { list_id, active }).into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "active_count failed");
            internal_error("Count failed.")
        }
    }
}

/// `DELETE /api/lists/:list_id/subscribers/:email` (GDPR erasure).
#[handler]
pub async fn delete(
    _auth: ApiAuth,
    Path((list_id, email)): Path<(OpaqueId<List>, String)>,
    Data(state): Data<&AppState>,
) -> Response {
    let raw = list_id.raw();
    match lists::find_list(&state.db, raw).await {
        Ok(Some(_)) => {}
        Ok(None) => return not_found("List does not exist"),
        Err(e) => {
            tracing::error!(error = ?e, "find_list failed");
            return internal_error("Database error.");
        }
    }

    match subscribers_svc::delete(&state.db, &email, raw).await {
        Ok(()) => Json(AckResponse { ok: true }).into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "delete failed");
            internal_error("Delete failed.")
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

pub(crate) fn nonempty(s: Option<String>) -> Option<String> {
    s.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}
