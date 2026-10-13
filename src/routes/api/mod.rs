pub mod auth;
pub mod brands;
pub mod campaigns;
pub mod lists;
pub mod rules;
pub mod segments;
pub mod settings;
pub mod subscribers;
pub mod templates;

use poem::{
    delete, get, http::StatusCode, post, web::Json, IntoResponse, Response, Route,
};
use serde::Serialize;

/// Mount all REST API endpoints under `/api`.
///
/// Conventions:
/// - Authentication is via the `X-Api-Key` header (or `Authorization: Bearer <key>`).
/// - Resource reads use `GET`; state changes use `POST` / `PUT` / `DELETE`.
/// - Identifiers live in the URL path; bodies and queries are JSON-typed.
pub fn routes() -> Route {
    Route::new()
        // ── Brands ───────────────────────────────────────────────────────
        .at("/brands", get(brands::list).post(brands::create))
        .at("/brands/:brand_id", get(brands::get).put(brands::update).delete(brands::delete))
        // ── Lists ────────────────────────────────────────────────────────
        .at("/brands/:brand_id/lists", get(lists::list_for_brand).post(lists::create))
        .at("/lists/:list_id", get(lists::get).put(lists::update).delete(lists::delete))
        // ── Subscribers ──────────────────────────────────────────────────
        .at(
            "/lists/:list_id/subscribers",
            get(subscribers::list).post(subscribers::subscribe),
        )
        .at(
            "/lists/:list_id/subscribers/count",
            get(subscribers::count),
        )
        .at(
            "/lists/:list_id/subscribers/:email/status",
            get(subscribers::status),
        )
        .at(
            "/lists/:list_id/subscribers/:email/unsubscribe",
            post(subscribers::unsubscribe),
        )
        .at(
            "/lists/:list_id/subscribers/:email",
            delete(subscribers::delete),
        )
        // ── Campaigns ────────────────────────────────────────────────────
        .at("/campaigns", get(campaigns::list).post(campaigns::create))
        .at("/campaigns/:campaign_id", get(campaigns::get).delete(campaigns::delete))
        .at("/campaigns/:campaign_id/report", get(campaigns::report))
        // ── Templates ────────────────────────────────────────────────────
        .at("/brands/:brand_id/templates", get(templates::list_for_brand).post(templates::create))
        .at("/templates/:template_id", get(templates::get).put(templates::update).delete(templates::delete))
        // ── Segments ─────────────────────────────────────────────────────
        .at("/brands/:brand_id/segments", get(segments::list_for_brand).post(segments::create))
        .at("/segments/:segment_id", delete(segments::delete))
        // ── Automation Rules ─────────────────────────────────────────────
        .at("/brands/:brand_id/rules", get(rules::list_for_brand).post(rules::create))
        .at("/rules/:rule_id/toggle", post(rules::toggle))
        .at("/rules/:rule_id", delete(rules::delete))
        // ── Suppression Lists & Blocked Domains ──────────────────────────
        .at("/brands/:brand_id/suppression-list", get(settings::list_suppression).post(settings::add_suppression))
        .at("/suppression-list/:id", delete(settings::delete_suppression))
        .at("/brands/:brand_id/blocked-domains", get(settings::list_blocked_domains).post(settings::add_blocked_domain))
        .at("/blocked-domains/:id", delete(settings::delete_blocked_domain))
}

/// Uniform JSON error envelope.
#[derive(Debug, Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ApiError {
    pub error: String,
}

impl ApiError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { error: msg.into() }
    }
}

/// Build a JSON error response with the given status code.
pub fn json_error(status: StatusCode, msg: impl Into<String>) -> Response {
    let mut resp = Json(ApiError::new(msg)).into_response();
    resp.set_status(status);
    resp
}

pub fn bad_request(msg: impl Into<String>) -> Response {
    json_error(StatusCode::BAD_REQUEST, msg)
}

pub fn not_found(msg: impl Into<String>) -> Response {
    json_error(StatusCode::NOT_FOUND, msg)
}

pub fn internal_error(msg: impl Into<String>) -> Response {
    json_error(StatusCode::INTERNAL_SERVER_ERROR, msg)
}
