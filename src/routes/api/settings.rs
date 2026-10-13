use poem::{
    handler,
    web::{Data, Json, Path, Query},
    IntoResponse, Response,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::routes::api::{auth::ApiAuth, bad_request, internal_error, not_found};
use crate::services::{brands, settings};
use crate::utils::opaque_id::{Brand, OpaqueId};
use crate::AppState;

#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
pub struct PageQuery {
    #[serde(default)]
    pub page: Option<u64>,
    #[serde(default)]
    pub limit: Option<u64>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SuppressionResponse {
    pub id: i32,
    pub email: String,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
pub struct AddEmailBody {
    pub email: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct BlockedDomainResponse {
    pub id: i32,
    pub domain: String,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
pub struct AddDomainBody {
    pub domain: String,
}

/// `GET /api/brands/:brand_id/suppression-list` — list suppressed emails for brand.
#[handler]
pub async fn list_suppression(
    _auth: ApiAuth,
    Path(brand_id): Path<OpaqueId<Brand>>,
    Query(q): Query<PageQuery>,
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

    let page = q.page.unwrap_or(0);
    let limit = q.limit.unwrap_or(50).min(500);

    match settings::list_suppression(&state.db, brand_raw, page, limit).await {
        Ok((rows, _total)) => {
            let body: Vec<SuppressionResponse> = rows
                .into_iter()
                .map(|s| SuppressionResponse {
                    id: s.id,
                    email: s.email,
                    timestamp: Some(s.timestamp),
                })
                .collect();
            Json(body).into_response()
        }
        Err(e) => {
            tracing::error!(error = ?e, "list_suppression failed");
            internal_error("Database error.")
        }
    }
}

/// `POST /api/brands/:brand_id/suppression-list` — add email to suppression list.
#[handler]
pub async fn add_suppression(
    _auth: ApiAuth,
    Path(brand_id): Path<OpaqueId<Brand>>,
    Json(body): Json<AddEmailBody>,
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

    if body.email.trim().is_empty() {
        return bad_request("Email cannot be empty.");
    }

    match settings::add_suppression(&state.db, brand_raw, &body.email).await {
        Ok(_) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "add_suppression failed");
            internal_error("Failed to add email to suppression list.")
        }
    }
}

/// `DELETE /api/suppression-list/:id` — remove email from suppression list.
#[handler]
pub async fn delete_suppression(
    _auth: ApiAuth,
    Path(id): Path<i32>,
    Data(state): Data<&AppState>,
) -> Response {
    match settings::delete_suppression(&state.db, id).await {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "delete_suppression failed");
            internal_error("Failed to remove suppression entry.")
        }
    }
}

/// `GET /api/brands/:brand_id/blocked-domains` — list blocked domains for brand.
#[handler]
pub async fn list_blocked_domains(
    _auth: ApiAuth,
    Path(brand_id): Path<OpaqueId<Brand>>,
    Query(q): Query<PageQuery>,
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

    let page = q.page.unwrap_or(0);
    let limit = q.limit.unwrap_or(50).min(500);

    match settings::list_blocked_domains(&state.db, brand_raw, page, limit).await {
        Ok((rows, _total)) => {
            let body: Vec<BlockedDomainResponse> = rows
                .into_iter()
                .map(|d| BlockedDomainResponse {
                    id: d.id,
                    domain: d.domain,
                    timestamp: Some(d.timestamp),
                })
                .collect();
            Json(body).into_response()
        }
        Err(e) => {
            tracing::error!(error = ?e, "list_blocked_domains failed");
            internal_error("Database error.")
        }
    }
}

/// `POST /api/brands/:brand_id/blocked-domains` — add domain to blocked domains.
#[handler]
pub async fn add_blocked_domain(
    _auth: ApiAuth,
    Path(brand_id): Path<OpaqueId<Brand>>,
    Json(body): Json<AddDomainBody>,
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

    if body.domain.trim().is_empty() {
        return bad_request("Domain cannot be empty.");
    }

    match settings::add_blocked_domain(&state.db, brand_raw, &body.domain).await {
        Ok(_) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "add_blocked_domain failed");
            internal_error("Failed to add domain to blocked domains.")
        }
    }
}

/// `DELETE /api/blocked-domains/:id` — remove blocked domain.
#[handler]
pub async fn delete_blocked_domain(
    _auth: ApiAuth,
    Path(id): Path<i32>,
    Data(state): Data<&AppState>,
) -> Response {
    match settings::delete_blocked_domain(&state.db, id).await {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "delete_blocked_domain failed");
            internal_error("Failed to remove blocked domain.")
        }
    }
}
