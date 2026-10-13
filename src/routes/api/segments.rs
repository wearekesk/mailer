use poem::{
    handler,
    web::{Data, Json, Path},
    IntoResponse, Response,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::routes::api::{auth::ApiAuth, bad_request, internal_error, not_found};
use crate::services::{brands, segments::{self, SegmentInput}};
use crate::utils::opaque_id::{Brand, List, OpaqueId, Segment};
use crate::AppState;

#[derive(Debug, Serialize, JsonSchema)]
pub struct SegmentResponse {
    pub id: OpaqueId<Segment>,
    pub brand_id: Option<OpaqueId<Brand>>,
    pub list_id: Option<OpaqueId<List>>,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct CreateSegmentBody {
    pub name: String,
    pub list_id: OpaqueId<List>,
}

/// `GET /api/brands/:brand_id/segments` — list segments for brand.
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

    match segments::list_for_brand(&state.db, brand_raw).await {
        Ok(rows) => {
            let body: Vec<SegmentResponse> = rows
                .into_iter()
                .map(|s| SegmentResponse {
                    id: OpaqueId::new(s.id),
                    brand_id: s.app.map(OpaqueId::new),
                    list_id: s.list.map(OpaqueId::new),
                    name: s.name.unwrap_or_default(),
                })
                .collect();
            Json(body).into_response()
        }
        Err(e) => {
            tracing::error!(error = ?e, "list segments failed");
            internal_error("Database error.")
        }
    }
}

/// `POST /api/brands/:brand_id/segments` — create a new segment.
#[handler]
pub async fn create(
    _auth: ApiAuth,
    Path(brand_id): Path<OpaqueId<Brand>>,
    Json(body): Json<CreateSegmentBody>,
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

    if body.name.trim().is_empty() {
        return bad_request("Segment name cannot be empty.");
    }

    let input = SegmentInput {
        name: body.name.trim().to_string(),
        list_id: body.list_id.raw(),
    };

    match segments::create(&state.db, brand_raw, input).await {
        Ok(new_id) => {
            let resp = SegmentResponse {
                id: OpaqueId::new(new_id),
                brand_id: Some(brand_id),
                list_id: Some(body.list_id),
                name: body.name,
            };
            Json(resp).into_response()
        }
        Err(e) => {
            tracing::error!(error = ?e, "create segment failed");
            internal_error("Failed to create segment.")
        }
    }
}

/// `DELETE /api/segments/:segment_id` — delete segment.
#[handler]
pub async fn delete(
    _auth: ApiAuth,
    Path(segment_id): Path<OpaqueId<Segment>>,
    Data(state): Data<&AppState>,
) -> Response {
    let raw = segment_id.raw();
    match segments::find(&state.db, raw).await {
        Ok(Some(_)) => {}
        Ok(None) => return not_found("Segment does not exist."),
        Err(e) => {
            tracing::error!(error = ?e, "find segment failed");
            return internal_error("Database error.");
        }
    }

    match segments::delete(&state.db, raw).await {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "delete segment failed");
            internal_error("Failed to delete segment.")
        }
    }
}
