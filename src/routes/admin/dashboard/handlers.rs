//! Top-level redirects only — there's no dashboard page anymore. The
//! brands list is the canonical landing page; both `/` and `/app/…/`
//! bounce there.

use poem::{handler, http::StatusCode, IntoResponse, Response};

const LANDING: &str = "/app/brands";

/// `GET /` — bounce to the brands list (the new landing page).
#[handler]
pub async fn root() -> impl IntoResponse {
    redirect(LANDING)
}

/// `GET /app/` and `/app/dashboard` — same bounce so old URLs keep
/// working.
#[handler]
pub async fn root_redirect() -> impl IntoResponse {
    redirect(LANDING)
}

fn redirect(location: &str) -> Response {
    Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", location)
        .body(())
}
