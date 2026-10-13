//! Email tracking endpoints for click tracking, open tracking, and web preview.
//!
//! - `GET /t/:link/:subscriber/:campaign` — link click + redirect
//! - `GET /o/:campaign/:subscriber`        — open tracking pixel
//! - `GET /w/:campaign/:subscriber`        — "view in browser" page
//!
//! Identifiers are [`OpaqueId`](crate::utils::opaque_id::OpaqueId) tokens so
//! a leaked URL cannot be reverse-engineered to internal row ids.

use poem::{
    handler,
    http::StatusCode,
    web::{Data, Path},
    IntoResponse, Response,
};
use sea_orm::EntityTrait;

use crate::database::entities::campaigns;
use crate::services::tracking;
use crate::utils::opaque_id::{Campaign, Link, OpaqueId, Subscriber};
use crate::AppState;

/// 1×1 transparent GIF returned for open-pixel requests.
static PIXEL_GIF: &[u8] = &[
    0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x01, 0x00, 0x01, 0x00, 0x80, 0x00, 0x00, 0xff, 0xff,
    0xff, 0x00, 0x00, 0x00, 0x21, 0xf9, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2c, 0x00, 0x00,
    0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x02, 0x02, 0x44, 0x01, 0x00, 0x3b,
];

/// `GET /t/:link/:subscriber/:campaign` — record a click and redirect.
#[handler]
pub async fn link_click(
    Path((link_token, subscriber_token, campaign_token)): Path<(
        OpaqueId<Link>,
        OpaqueId<Subscriber>,
        OpaqueId<Campaign>,
    )>,
    Data(state): Data<&AppState>,
) -> Response {
    match tracking::record_click(
        &state.db,
        link_token.raw(),
        subscriber_token.raw(),
        campaign_token.raw(),
    )
    .await
    {
        Ok(result) => {
            let mut url = result.redirect_to;
            if !url.to_lowercase().starts_with("http") {
                url = format!("http://{url}");
            }
            Response::builder()
                .status(StatusCode::MOVED_PERMANENTLY)
                .header("Location", url)
                .header("X-Robots-Tag", "none")
                .body(())
        }
        Err(e) => {
            tracing::warn!(error = ?e, link = ?link_token, "link_click failed");
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "text/html; charset=utf-8")
                .body("<h2>Link no longer exists</h2>")
        }
    }
}

/// `GET /o/:campaign/:subscriber` — return the 1×1 GIF and record an open.
#[handler]
pub async fn open_pixel(
    Path((campaign_token, subscriber_token)): Path<(OpaqueId<Campaign>, OpaqueId<Subscriber>)>,
    Data(state): Data<&AppState>,
) -> impl IntoResponse {
    if let Err(e) =
        tracking::record_open(&state.db, campaign_token.raw(), None, subscriber_token.raw()).await
    {
        tracing::warn!(error = ?e, "record_open failed");
    }

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "image/gif")
        .header("Cache-Control", "no-store, no-cache, must-revalidate")
        .header("Pragma", "no-cache")
        .header("X-Robots-Tag", "none")
        .body(PIXEL_GIF.to_vec())
}

/// `GET /w/:campaign/:subscriber` — "view in browser" web version of a campaign.
#[handler]
pub async fn web_version(
    Path((campaign_token, _subscriber_token)): Path<(OpaqueId<Campaign>, OpaqueId<Subscriber>)>,
    Data(state): Data<&AppState>,
) -> Response {
    let campaign = match campaigns::Entity::find_by_id(campaign_token.raw())
        .one(&state.db)
        .await
    {
        Ok(Some(c)) => c,
        _ => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "text/html; charset=utf-8")
                .body("<h2>Campaign not found</h2>");
        }
    };

    let body = campaign.html_text.unwrap_or_else(|| "<p>No content.</p>".into());
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("X-Robots-Tag", "none")
        .body(body)
}
