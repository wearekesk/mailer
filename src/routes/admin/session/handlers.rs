//! Admin session cookie + `CurrentUser` extractor.
//!
//! Two cookies are issued:
//!
//! - `mailer_session` — opaque token wrapping the user-id; presence ⇒ logged in
//! - `mailer_pre_2fa` — short-lived token issued between password verification
//!   and 2FA confirmation. Replaced by `mailer_session` on TOTP success.
//!
//! Both tokens are AES-128 encrypted with per-tag keys
//! (see `utils/opaque_id.rs`), so they're tamper-proof and require the
//! application secret to mint.

use std::time::Duration;

use poem::{
    http::StatusCode,
    web::cookie::{Cookie, CookieJar, SameSite},
    Error, FromRequest, IntoResponse, Request, RequestBody, Response, Result,
};

use crate::database::entities::login;
use crate::services::auth;
use crate::utils::opaque_id::OpaqueId;
use crate::AppState;

// Phantom tags for domain-separated session cookies.
pub struct Session;
pub struct Pre2fa;

impl crate::utils::opaque_id::OpaqueTag for Session {
    const TAG: &'static str = "session";
}
impl crate::utils::opaque_id::OpaqueTag for Pre2fa {
    const TAG: &'static str = "pre2fa";
}

pub const SESSION_COOKIE: &str = "mailer_session";
pub const PRE_2FA_COOKIE: &str = "mailer_pre_2fa";

const SESSION_LIFETIME: Duration = Duration::from_secs(60 * 60 * 24 * 30); // 30 days
const PRE_2FA_LIFETIME: Duration = Duration::from_secs(60 * 5); // 5 minutes

/// Issue (or refresh) the long-lived session cookie for `user`.
pub fn issue_session(jar: &CookieJar, user: &login::Model) {
    let token = OpaqueId::<Session>::new(user.id).to_string();
    let mut cookie = Cookie::new_with_str(SESSION_COOKIE, token);
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_max_age(SESSION_LIFETIME);
    jar.add(cookie);

    // Drop any leftover pre-2FA cookie from this login flow.
    clear_pre_2fa(jar);
}

/// Issue a short-lived cookie carrying the user-id between the password and
/// TOTP steps of the login flow.
pub fn issue_pre_2fa(jar: &CookieJar, user: &login::Model) {
    let token = OpaqueId::<Pre2fa>::new(user.id).to_string();
    let mut cookie = Cookie::new_with_str(PRE_2FA_COOKIE, token);
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_max_age(PRE_2FA_LIFETIME);
    jar.add(cookie);
}

/// Read & decrypt the pre-2FA cookie. Returns `None` if missing or invalid.
pub fn read_pre_2fa(jar: &CookieJar) -> Option<i32> {
    let raw = jar.get(PRE_2FA_COOKIE)?;
    let value = raw.value_str().to_string();
    value.parse::<OpaqueId<Pre2fa>>().ok().map(|o| o.raw())
}

pub fn clear_pre_2fa(jar: &CookieJar) {
    jar.remove(PRE_2FA_COOKIE);
}

pub fn clear_session(jar: &CookieJar) {
    jar.remove(SESSION_COOKIE);
    clear_pre_2fa(jar);
}

/// Attempt to decrypt the session cookie and return the user-id without
/// touching the database.
pub fn read_session_user_id(jar: &CookieJar) -> Option<i32> {
    let raw = jar.get(SESSION_COOKIE)?;
    let value = raw.value_str().to_string();
    value.parse::<OpaqueId<Session>>().ok().map(|o| o.raw())
}

/// Authenticated user resolved from the session cookie. Use as a poem
/// extractor in any admin handler that requires login. On failure, the
/// extractor returns a 302 redirect to `/login`.
pub struct CurrentUser(pub login::Model);

impl<'a> FromRequest<'a> for CurrentUser {
    async fn from_request(req: &'a Request, body: &mut RequestBody) -> Result<Self> {
        let jar = <&CookieJar>::from_request(req, body).await?;
        let user_id = read_session_user_id(jar).ok_or_else(redirect_to_login)?;
        let state = <poem::web::Data<&AppState>>::from_request(req, body).await?;
        let user = auth::find_user(&state.0.db, user_id)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "session user lookup failed");
                redirect_to_login()
            })?
            .ok_or_else(redirect_to_login)?;
        Ok(CurrentUser(user))
    }
}

fn redirect_to_login() -> Error {
    let resp = Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", "/login")
        .body(())
        .into_response();
    Error::from_response(resp)
}
