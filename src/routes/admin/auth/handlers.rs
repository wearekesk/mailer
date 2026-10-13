use super::views::*;
use leptos::prelude::*;
use poem::{
    handler,
    http::StatusCode,
    web::{
        cookie::CookieJar,
        Data, Form, Query,
    },
    IntoResponse, Response,
};
use serde::Deserialize;
use std::collections::HashMap;
use crate::components::layout::{AuthCard, AuthDocument};
use crate::routes::admin::session::{
    clear_pre_2fa, clear_session, issue_pre_2fa, issue_session, read_pre_2fa,
};
use crate::services::auth::{self, LoginOutcome};
use crate::AppState;

/// `GET /login`
#[handler]
pub async fn login_form(Query(q): Query<LoginQuery>, jar: &CookieJar) -> impl IntoResponse {
    // Already logged in? Send to /app.
    if crate::routes::admin::session::read_session_user_id(jar).is_some() {
        return Response::builder()
            .status(StatusCode::FOUND)
            .header("Location", "/app")
            .body(());
    }

    let error = match q.e {
        Some(1) => Some("Please fill in both email and password.".to_string()),
        Some(2) => Some("Incorrect password or user does not exist.".to_string()),
        Some(3) => Some("Invalid password reset request.".to_string()),
        _ => None,
    };
    let info = match q.i {
        Some(1) => Some("Your password reset email has been sent.".to_string()),
        _ => None,
    };
    let html = LoginPage(LoginPageProps {
        error,
        info,
    })
    .to_html();
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(html)
}

/// `POST /login`
#[handler]
pub async fn login(
    Form(params): Form<HashMap<String, String>>,
    Data(state): Data<&AppState>,
    jar: &CookieJar,
) -> impl IntoResponse {
    let email = params.get("email").map(|s| s.trim()).unwrap_or("");
    let password = params.get("password").map(String::as_str).unwrap_or("");

    if email.is_empty() || password.is_empty() {
        return redirect("/login?e=1");
    }

    match auth::login(&state.db, email, password).await {
        Ok(LoginOutcome::Authenticated(user)) => {
            issue_session(jar, &user);
            redirect("/app")
        }
        Ok(LoginOutcome::NeedsTwoFactor(user)) => {
            issue_pre_2fa(jar, &user);
            redirect("/2fa")
        }
        Ok(LoginOutcome::Invalid) => redirect("/login?e=2"),
        Err(e) => {
            tracing::error!(error = ?e, "login failed");
            redirect("/login?e=2")
        }
    }
}

/// `GET /logout`
#[handler]
pub async fn logout(jar: &CookieJar) -> impl IntoResponse {
    clear_session(jar);
    redirect("/login")
}

#[derive(Debug, Deserialize)]
pub struct TwoFactorQuery {
    pub e: Option<u8>,
}

/// `GET /2fa`
#[handler]
pub async fn two_factor_form(Query(q): Query<TwoFactorQuery>, jar: &CookieJar) -> impl IntoResponse {
    // Must be in the pre-2FA window.
    if read_pre_2fa(jar).is_none() {
        return Response::builder()
            .status(StatusCode::FOUND)
            .header("Location", "/login")
            .body(());
    }
    let error = match q.e {
        Some(1) => Some("OTP code must be numeric.".to_string()),
        Some(2) => Some("OTP code is incorrect.".to_string()),
        _ => None,
    };
    let html = TwoFactorPage(TwoFactorPageProps { error }).to_html();
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(html)
}

/// `POST /2fa`
#[handler]
pub async fn two_factor_verify(
    Form(params): Form<HashMap<String, String>>,
    Data(state): Data<&AppState>,
    jar: &CookieJar,
) -> impl IntoResponse {
    let Some(user_id) = read_pre_2fa(jar) else {
        return redirect("/login");
    };
    let code = params.get("otp_code").map(|s| s.trim()).unwrap_or("");
    if code.is_empty() || !code.chars().all(|c| c.is_ascii_digit()) {
        return redirect("/2fa?e=1");
    }

    let user = match auth::find_user(&state.db, user_id).await {
        Ok(Some(u)) => u,
        _ => return redirect("/login"),
    };
    let secret = user.auth_key.as_deref().unwrap_or("");
    if !auth::verify_totp(secret, code) {
        return redirect("/2fa?e=2");
    }

    clear_pre_2fa(jar);
    issue_session(jar, &user);
    redirect("/app")
}

/// `GET /forgot-password`
#[handler]
pub async fn forgot_form() -> impl IntoResponse {
    let html = ForgotPasswordPage(ForgotPasswordPageProps { info: None }).to_html();
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(html)
}

/// `POST /forgot-password`
#[handler]
pub async fn forgot(
    Form(params): Form<HashMap<String, String>>,
    Data(state): Data<&AppState>,
) -> impl IntoResponse {
    let email = params.get("email").map(|s| s.trim()).unwrap_or("");
    if email.is_empty() {
        return redirect("/forgot-password");
    }

    match auth::request_password_reset(&state.db, email).await {
        Ok(Some((token, user_id))) => {
            // Real send goes through services::mailer once SMTP/SES is wired.
            // For now: log the reset URL.
            let link = format!(
                "{}/reset-password?token={}&uid={}",
                state.config.app_url, token, user_id
            );
            tracing::info!(email = %email, %link, "Password reset link generated");
        }
        Ok(None) => {
            tracing::info!(email = %email, "Password reset requested for unknown email");
        }
        Err(e) => {
            tracing::error!(error = ?e, "request_password_reset failed");
        }
    }

    // Always respond positively to avoid leaking which addresses exist.
    redirect("/login?i=1")
}

#[derive(Debug, Deserialize)]
pub struct ResetQuery {
    pub token: String,
    pub uid: i32,
}

/// `GET /reset-password?token=…&uid=…`
#[handler]
pub async fn reset_form(Query(q): Query<ResetQuery>) -> impl IntoResponse {
    let html = ResetPasswordPage(ResetPasswordPageProps {
        token: q.token,
        uid: q.uid,
    })
    .to_html();
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(html)
}

#[derive(Debug, Deserialize)]
pub struct ResetForm {
    pub token: String,
    pub uid: i32,
    pub password: String,
}

/// `POST /reset-password`
#[handler]
pub async fn reset_submit(
    Form(form): Form<ResetForm>,
    Data(state): Data<&AppState>,
) -> impl IntoResponse {
    if form.password.len() < 8 {
        return redirect(&format!(
            "/reset-password?token={}&uid={}&e=1",
            form.token, form.uid
        ));
    }
    match auth::reset_password(&state.db, &form.token, form.uid, &form.password).await {
        Ok(true) => redirect("/login?i=2"),
        Ok(false) => redirect("/login?e=3"),
        Err(e) => {
            tracing::error!(error = ?e, "reset_password failed");
            redirect("/login?e=3")
        }
    }
}

#[component]
fn ResetPasswordPage(#[prop(into)] token: String, uid: i32) -> impl IntoView {
    view! {
        <AuthDocument title="Reset password">
            <AuthCard heading="Set a new password">
                <form class="space-y-4 md:space-y-6" method="POST" action="/reset-password">
                    <input type="hidden" name="token" value=token/>
                    <input type="hidden" name="uid" value=uid.to_string()/>
                    <div>
                        <label for="password" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"New password"</label>
                        <input
                            type="password" name="password" id="password" autocomplete="new-password" required minlength="8"
                            class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg focus:ring-blue-600 focus:border-blue-600 block w-full p-2.5"
                        />
                    </div>
                    <button
                        type="submit"
                        class="w-full text-white bg-blue-600 hover:bg-blue-700 focus:ring-4 focus:outline-none focus:ring-blue-300 font-medium rounded-lg text-sm px-5 py-2.5"
                    >
                        "Set password"
                    </button>
                </form>
            </AuthCard>
        </AuthDocument>
    }
}

// ── Helper ───────────────────────────────────────────────────────────────────

fn redirect(location: &str) -> Response {
    Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", location)
        .body(())
}
