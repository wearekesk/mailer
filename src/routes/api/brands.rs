use poem::{
    handler,
    web::{Data, Json, Path},
    IntoResponse, Response,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::routes::api::{auth::ApiAuth, bad_request, internal_error, not_found};
use crate::services::brands::{self, BrandInput};
use crate::utils::opaque_id::{Brand, OpaqueId};
use crate::AppState;

#[derive(Debug, Serialize, JsonSchema)]
pub struct BrandResponse {
    pub id: OpaqueId<Brand>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscriber_count: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
pub struct CreateBrandBody {
    pub name: String,
    #[serde(default)]
    pub from_name: Option<String>,
    #[serde(default)]
    pub from_email: Option<String>,
    #[serde(default)]
    pub reply_to: Option<String>,
    #[serde(default)]
    pub smtp_host: Option<String>,
    #[serde(default)]
    pub smtp_port: Option<String>,
    #[serde(default)]
    pub smtp_ssl: Option<String>,
    #[serde(default)]
    pub smtp_username: Option<String>,
    #[serde(default)]
    pub smtp_password: Option<String>,
    #[serde(default)]
    pub query_string: Option<String>,
    #[serde(default)]
    pub test_email_prefix: Option<String>,
    #[serde(default)]
    pub custom_domain: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
pub struct UpdateBrandBody {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub from_name: Option<String>,
    #[serde(default)]
    pub from_email: Option<String>,
    #[serde(default)]
    pub reply_to: Option<String>,
    #[serde(default)]
    pub smtp_host: Option<String>,
    #[serde(default)]
    pub smtp_port: Option<String>,
    #[serde(default)]
    pub smtp_ssl: Option<String>,
    #[serde(default)]
    pub smtp_username: Option<String>,
    #[serde(default)]
    pub smtp_password: Option<String>,
    #[serde(default)]
    pub query_string: Option<String>,
    #[serde(default)]
    pub test_email_prefix: Option<String>,
    #[serde(default)]
    pub custom_domain: Option<String>,
}

/// `GET /api/brands` — list all brands on the install.
#[handler]
pub async fn list(_auth: ApiAuth, Data(state): Data<&AppState>) -> Response {
    match brands::list_brands(&state.db).await {
        Ok(rows) => {
            let body: Vec<BrandResponse> = rows
                .into_iter()
                .map(|b| BrandResponse {
                    id: OpaqueId::new(b.id),
                    name: b.name,
                    from_email: b.from_email,
                    list_count: Some(b.list_count),
                    subscriber_count: Some(b.subscriber_count),
                })
                .collect();
            Json(body).into_response()
        }
        Err(e) => {
            tracing::error!(error = ?e, "list_brands failed");
            internal_error("Database error.")
        }
    }
}

/// `GET /api/brands/:brand_id` — fetch brand details.
#[handler]
pub async fn get(
    _auth: ApiAuth,
    Path(brand_id): Path<OpaqueId<Brand>>,
    Data(state): Data<&AppState>,
) -> Response {
    match brands::find_brand(&state.db, brand_id.raw()).await {
        Ok(Some(b)) => {
            let resp = BrandResponse {
                id: OpaqueId::new(b.id),
                name: b.app_name.unwrap_or_default(),
                from_email: b.from_email,
                list_count: None,
                subscriber_count: None,
            };
            Json(resp).into_response()
        }
        Ok(None) => not_found("Brand does not exist."),
        Err(e) => {
            tracing::error!(error = ?e, "find_brand failed");
            internal_error("Database error.")
        }
    }
}

/// `POST /api/brands` — create a new brand.
#[handler]
pub async fn create(
    auth: ApiAuth,
    Json(body): Json<CreateBrandBody>,
    Data(state): Data<&AppState>,
) -> Response {
    if body.name.trim().is_empty() {
        return bad_request("Brand name cannot be empty.");
    }
    let input = BrandInput {
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
    match brands::create_brand(&state.db, auth.0.id, input).await {
        Ok(new_id) => {
            let resp = BrandResponse {
                id: OpaqueId::new(new_id),
                name: body.name,
                from_email: None,
                list_count: Some(0),
                subscriber_count: Some(0),
            };
            Json(resp).into_response()
        }
        Err(e) => {
            tracing::error!(error = ?e, "create_brand failed");
            internal_error("Failed to create brand.")
        }
    }
}

/// `PUT /api/brands/:brand_id` — update an existing brand.
#[handler]
pub async fn update(
    _auth: ApiAuth,
    Path(brand_id): Path<OpaqueId<Brand>>,
    Json(body): Json<UpdateBrandBody>,
    Data(state): Data<&AppState>,
) -> Response {
    let raw = brand_id.raw();
    let existing = match brands::find_brand(&state.db, raw).await {
        Ok(Some(b)) => b,
        Ok(None) => return not_found("Brand does not exist."),
        Err(e) => {
            tracing::error!(error = ?e, "find_brand failed");
            return internal_error("Database error.");
        }
    };

    let input = BrandInput {
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

    match brands::update_brand(&state.db, raw, input).await {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "update_brand failed");
            internal_error("Failed to update brand.")
        }
    }
}

/// `DELETE /api/brands/:brand_id` — delete brand and its resources.
#[handler]
pub async fn delete(
    _auth: ApiAuth,
    Path(brand_id): Path<OpaqueId<Brand>>,
    Data(state): Data<&AppState>,
) -> Response {
    let raw = brand_id.raw();
    match brands::find_brand(&state.db, raw).await {
        Ok(Some(_)) => {}
        Ok(None) => return not_found("Brand does not exist."),
        Err(e) => {
            tracing::error!(error = ?e, "find_brand failed");
            return internal_error("Database error.");
        }
    }

    match brands::delete_brand(&state.db, raw).await {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "delete_brand failed");
            internal_error("Failed to delete brand.")
        }
    }
}
