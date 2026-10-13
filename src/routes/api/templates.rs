use poem::{
    handler,
    web::{Data, Json, Path},
    IntoResponse, Response,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::routes::api::{auth::ApiAuth, bad_request, internal_error, not_found};
use crate::services::{brands, templates::{self, TemplateInput}};
use crate::utils::opaque_id::{Brand, OpaqueId, Template};
use crate::AppState;

#[derive(Debug, Serialize, JsonSchema)]
pub struct TemplateResponse {
    pub id: OpaqueId<Template>,
    pub brand_id: Option<OpaqueId<Brand>>,
    pub name: String,
    pub from_name: Option<String>,
    pub from_email: Option<String>,
    pub reply_to: Option<String>,
    pub html_text: Option<String>,
    pub plain_text: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
pub struct CreateTemplateBody {
    pub name: String,
    #[serde(default)]
    pub html_text: Option<String>,
    #[serde(default)]
    pub plain_text: Option<String>,
    #[serde(default)]
    pub from_name: Option<String>,
    #[serde(default)]
    pub from_email: Option<String>,
    #[serde(default)]
    pub reply_to: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
pub struct UpdateTemplateBody {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub html_text: Option<String>,
    #[serde(default)]
    pub plain_text: Option<String>,
    #[serde(default)]
    pub from_name: Option<String>,
    #[serde(default)]
    pub from_email: Option<String>,
    #[serde(default)]
    pub reply_to: Option<String>,
}

/// `GET /api/brands/:brand_id/templates` — list templates for brand.
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

    match templates::list_for_brand(&state.db, brand_raw).await {
        Ok(rows) => {
            let body: Vec<TemplateResponse> = rows
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
            Json(body).into_response()
        }
        Err(e) => {
            tracing::error!(error = ?e, "list templates failed");
            internal_error("Database error.")
        }
    }
}

/// `GET /api/templates/:template_id` — fetch template details.
#[handler]
pub async fn get(
    _auth: ApiAuth,
    Path(template_id): Path<OpaqueId<Template>>,
    Data(state): Data<&AppState>,
) -> Response {
    match templates::find(&state.db, template_id.raw()).await {
        Ok(Some(t)) => {
            let resp = TemplateResponse {
                id: OpaqueId::new(t.id),
                brand_id: t.app.map(OpaqueId::new),
                name: t.template_name.unwrap_or_default(),
                from_name: t.from_name,
                from_email: t.from_email,
                reply_to: t.reply_to,
                html_text: t.html_text,
                plain_text: t.plain_text,
            };
            Json(resp).into_response()
        }
        Ok(None) => not_found("Template does not exist."),
        Err(e) => {
            tracing::error!(error = ?e, "find template failed");
            internal_error("Database error.")
        }
    }
}

/// `POST /api/brands/:brand_id/templates` — create a new template.
#[handler]
pub async fn create(
    auth: ApiAuth,
    Path(brand_id): Path<OpaqueId<Brand>>,
    Json(body): Json<CreateTemplateBody>,
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
        return bad_request("Template name cannot be empty.");
    }

    let input = TemplateInput {
        template_name: body.name.trim().to_string(),
        html_text: body.html_text,
        plain_text: body.plain_text,
        from_name: body.from_name,
        from_email: body.from_email,
        reply_to: body.reply_to,
    };

    match templates::create(&state.db, auth.0.id, brand_raw, input).await {
        Ok(new_id) => {
            let resp = TemplateResponse {
                id: OpaqueId::new(new_id),
                brand_id: Some(brand_id),
                name: body.name,
                from_name: None,
                from_email: None,
                reply_to: None,
                html_text: None,
                plain_text: None,
            };
            Json(resp).into_response()
        }
        Err(e) => {
            tracing::error!(error = ?e, "create template failed");
            internal_error("Failed to create template.")
        }
    }
}

/// `PUT /api/templates/:template_id` — update an existing template.
#[handler]
pub async fn update(
    _auth: ApiAuth,
    Path(template_id): Path<OpaqueId<Template>>,
    Json(body): Json<UpdateTemplateBody>,
    Data(state): Data<&AppState>,
) -> Response {
    let raw = template_id.raw();
    let existing = match templates::find(&state.db, raw).await {
        Ok(Some(t)) => t,
        Ok(None) => return not_found("Template does not exist."),
        Err(e) => {
            tracing::error!(error = ?e, "find template failed");
            return internal_error("Database error.");
        }
    };

    let input = TemplateInput {
        template_name: body.name.unwrap_or_else(|| existing.template_name.clone().unwrap_or_default()),
        html_text: body.html_text.or(existing.html_text),
        plain_text: body.plain_text.or(existing.plain_text),
        from_name: body.from_name.or(existing.from_name),
        from_email: body.from_email.or(existing.from_email),
        reply_to: body.reply_to.or(existing.reply_to),
    };

    match templates::update(&state.db, raw, input).await {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "update template failed");
            internal_error("Failed to update template.")
        }
    }
}

/// `DELETE /api/templates/:template_id` — delete template.
#[handler]
pub async fn delete(
    _auth: ApiAuth,
    Path(template_id): Path<OpaqueId<Template>>,
    Data(state): Data<&AppState>,
) -> Response {
    let raw = template_id.raw();
    match templates::find(&state.db, raw).await {
        Ok(Some(_)) => {}
        Ok(None) => return not_found("Template does not exist."),
        Err(e) => {
            tracing::error!(error = ?e, "find template failed");
            return internal_error("Database error.");
        }
    }

    match templates::delete(&state.db, raw).await {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "delete template failed");
            internal_error("Failed to delete template.")
        }
    }
}
