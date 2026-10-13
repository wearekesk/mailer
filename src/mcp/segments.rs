use rmcp::{
    handler::server::wrapper::{Json as McpJson, Parameters},
    schemars::JsonSchema,
    ErrorData,
};
use serde::{Deserialize, Serialize};

use crate::routes::api::{
    segments::SegmentResponse,
    subscribers::AckResponse,
};
use crate::services::segments as segments_svc;
use crate::utils::opaque_id::{Brand, List, OpaqueId, Segment};
use crate::AppState;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct CreateSegmentParams {
    #[schemars(description = "Opaque identifier of the brand")]
    pub brand_id: OpaqueId<Brand>,
    #[schemars(description = "Name of the segment")]
    pub name: String,
    #[schemars(description = "Target mailing list ID")]
    pub list_id: OpaqueId<List>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SegmentParam {
    #[schemars(description = "Opaque identifier of the segment")]
    pub segment_id: OpaqueId<Segment>,
}

pub async fn list_segments(
    state: &AppState,
    Parameters(params): Parameters<crate::mcp::brands::BrandParam>,
) -> Result<McpJson<Vec<SegmentResponse>>, ErrorData> {
    let rows = segments_svc::list_for_brand(&state.db, params.brand_id.raw())
        .await
        .map_err(|e| ErrorData::internal_error(format!("list segments error: {e}"), None))?;

    let items = rows
        .into_iter()
        .map(|s| SegmentResponse {
            id: OpaqueId::new(s.id),
            brand_id: s.app.map(OpaqueId::new),
            list_id: s.list.map(OpaqueId::new),
            name: s.name.unwrap_or_default(),
        })
        .collect();

    Ok(McpJson(items))
}

pub async fn create_segment(
    state: &AppState,
    Parameters(params): Parameters<CreateSegmentParams>,
) -> Result<McpJson<SegmentResponse>, ErrorData> {
    let brand_raw = params.brand_id.raw();
    if params.name.trim().is_empty() {
        return Err(ErrorData::invalid_params("Segment name cannot be empty", None));
    }
    let input = segments_svc::SegmentInput {
        name: params.name.trim().to_string(),
        list_id: params.list_id.raw(),
    };
    let new_id = segments_svc::create(&state.db, brand_raw, input)
        .await
        .map_err(|e| ErrorData::internal_error(format!("create segment error: {e}"), None))?;

    Ok(McpJson(SegmentResponse {
        id: OpaqueId::new(new_id),
        brand_id: Some(params.brand_id),
        list_id: Some(params.list_id),
        name: params.name,
    }))
}

pub async fn delete_segment(
    state: &AppState,
    Parameters(params): Parameters<SegmentParam>,
) -> Result<McpJson<AckResponse>, ErrorData> {
    segments_svc::delete(&state.db, params.segment_id.raw())
        .await
        .map_err(|e| ErrorData::internal_error(format!("delete segment error: {e}"), None))?;
    Ok(McpJson(AckResponse { ok: true }))
}
