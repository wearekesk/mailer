use super::views::*;
use leptos::prelude::*;
use poem::{
    handler,
    http::StatusCode,
    web::{Data, Form, Path, Query},
    Response,
};
use serde::Deserialize;
use crate::routes::admin::session::CurrentUser;
use crate::services::settings::{
    self, ProfileInput,
};
use crate::utils::opaque_id::{Brand, OpaqueId};
use crate::AppState;

#[handler]
pub async fn index(
    user: CurrentUser,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Settings", move || {
        view! { <SettingsPage user=user.0/> }
    })
    .await
}

#[handler]
pub async fn save(
    user: CurrentUser,
    Form(form): Form<ProfileForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let input: ProfileInput = form.into();
    match settings::save_profile(&state.db, user.0.id, input).await {
        Ok(()) => redirect("/app/settings?i=saved"),
        Err(e) => {
            tracing::error!(error = ?e, "save_profile failed");
            redirect("/app/settings?e=db")
        }
    }
}

#[handler]
pub async fn regenerate_api_key(user: CurrentUser, Data(state): Data<&AppState>) -> Response {
    let _ = settings::regenerate_api_key(&state.db, user.0.id).await;
    redirect("/app/settings?i=regen")
}

// ── 2FA enrolment ───────────────────────────────────────────────────────────

#[handler]
pub async fn two_factor_setup(
    user: CurrentUser,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let secret = match crate::services::auth::begin_totp_enrollment(&state.db, user.0.id).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = ?e, "2fa enrollment start failed");
            return redirect("/app/settings?e=2fa");
        }
    };
    let account = user.0.username.clone().unwrap_or_else(|| "admin".into());
    let url = crate::services::auth::totp_provisioning_url(&secret, &account, "Mailer")
        .unwrap_or_default();
    let svg = crate::services::auth::qr_svg(&url).unwrap_or_default();

    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Two-factor setup", move || {
        view! { <TwoFactorSetupPage qr_svg=svg secret=secret/> }
    })
    .await
}

#[derive(Debug, Deserialize)]
pub struct TotpForm {
    pub otp_code: String,
}

#[handler]
pub async fn two_factor_confirm(
    user: CurrentUser,
    Form(form): Form<TotpForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let code = form.otp_code.trim();
    if !code.chars().all(|c| c.is_ascii_digit()) || code.len() != 6 {
        return redirect("/app/settings/two-factor/setup?e=bad_code");
    }
    match crate::services::auth::confirm_totp_enrollment(&state.db, user.0.id, code).await {
        Ok(true) => redirect("/app/settings?i=2fa_enabled"),
        Ok(false) => redirect("/app/settings/two-factor/setup?e=mismatch"),
        Err(e) => {
            tracing::error!(error = ?e, "2fa confirm failed");
            redirect("/app/settings?e=2fa")
        }
    }
}

#[handler]
pub async fn two_factor_disable(user: CurrentUser, Data(state): Data<&AppState>) -> Response {
    let _ = crate::services::auth::disable_totp(&state.db, user.0.id).await;
    redirect("/app/settings?i=2fa_disabled")
}

/// `POST /app/settings/ses/verify-login-email` — verify the admin's login
/// email with SES so password-reset / system emails can go out via SES.
#[handler]
pub async fn ses_verify_login_email(user: CurrentUser, _state: Data<&AppState>) -> Response {
    let key = user.0.s3_key.clone().unwrap_or_default();
    let secret = user.0.s3_secret.clone().unwrap_or_default();
    let email = user.0.username.clone().unwrap_or_default();
    if key.is_empty() || secret.is_empty() {
        return redirect("/app/settings?e=no_aws_creds");
    }
    if email.is_empty() {
        return redirect("/app/settings?e=no_email");
    }
    let region =
        crate::services::ses::region_from_endpoint(user.0.ses_endpoint.as_deref().unwrap_or(""));
    if let Err(e) = crate::services::ses::verify_identity(&key, &secret, &region, &email).await {
        tracing::warn!(error = ?e, "ses verify-login-email failed");
        return redirect("/app/settings?e=ses_verify");
    }
    redirect("/app/settings?i=ses_verify_sent")
}

// ── Suppression-list pages + handlers ───────────────────────────────────────

#[handler]
pub async fn suppression_page(
    _user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    Query(q): Query<PageQuery>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let brand = match crate::services::brands::find_brand(&state.db, brand_token.raw()).await {
        Ok(Some(b)) => b,
        _ => return redirect("/app/brands"),
    };
    let page = q.page.unwrap_or(0);
    let (rows, total) = settings::list_suppression(&state.db, brand_token.raw(), page, PAGE_SIZE)
        .await
        .unwrap_or((Vec::new(), 0));
    let token = brand_token.to_string();
    let brand_name = brand.app_name.unwrap_or_default();
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Suppression list", move || {
        view! {
            <SuppressionListPage brand_token=token brand_name=brand_name rows=rows page=page total=total/>
        }
    })
    .await
}

#[handler]
pub async fn add_suppression(
    _user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    Form(form): Form<EmailForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = settings::add_suppression(&state.db, brand_token.raw(), &form.email).await;
    redirect(&format!("/app/brands/{brand_token}/suppression-list"))
}

#[handler]
pub async fn delete_suppression(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Query(q): Query<BrandQuery>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = settings::delete_suppression(&state.db, id).await;
    redirect(&format!("/app/brands/{}/suppression-list", q.brand))
}

// ── Blocked domains pages + handlers ────────────────────────────────────────

#[handler]
pub async fn blocked_domains_page(
    _user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    Query(q): Query<PageQuery>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let brand = match crate::services::brands::find_brand(&state.db, brand_token.raw()).await {
        Ok(Some(b)) => b,
        _ => return redirect("/app/brands"),
    };
    let page = q.page.unwrap_or(0);
    let (rows, total) = settings::list_blocked_domains(&state.db, brand_token.raw(), page, PAGE_SIZE)
        .await
        .unwrap_or((Vec::new(), 0));
    let token = brand_token.to_string();
    let brand_name = brand.app_name.unwrap_or_default();
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Blocked domains", move || {
        view! {
            <BlockedDomainsPage brand_token=token brand_name=brand_name rows=rows page=page total=total/>
        }
    })
    .await
}

#[handler]
pub async fn add_blocked_domain(
    _user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    Form(form): Form<DomainForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = settings::add_blocked_domain(&state.db, brand_token.raw(), &form.domain).await;
    redirect(&format!("/app/brands/{brand_token}/blocked-domains"))
}

#[handler]
pub async fn delete_blocked_domain(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Query(q): Query<BrandQuery>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = settings::delete_blocked_domain(&state.db, id).await;
    redirect(&format!("/app/brands/{}/blocked-domains", q.brand))
}

/// `POST /app/brands/:id/suppression-list/clear` — wipe every entry.
#[handler]
pub async fn clear_suppression_list(
    _user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = settings::clear_suppression_list(&state.db, brand_token.raw()).await;
    redirect(&format!("/app/brands/{brand_token}/suppression-list?i=cleared"))
}

/// `POST /app/brands/:id/blocked-domains/clear` — wipe every entry.
#[handler]
pub async fn clear_blocked_domains(
    _user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = settings::clear_blocked_domains(&state.db, brand_token.raw()).await;
    redirect(&format!("/app/brands/{brand_token}/blocked-domains?i=cleared"))
}

/// `POST /app/settings/scheduler/reset-cron-lock` — clear the
/// `login.cron` lock if a previous tick crashed without releasing it.
#[handler]
pub async fn reset_cron_lock(_user: CurrentUser, Data(state): Data<&AppState>) -> Response {
    let _ = crate::services::scheduler::force_release_lock(&state.db).await;
    redirect("/app/settings?i=cron_reset")
}

/// `POST /app/brands/:id/suppression-list/import` — bulk-add suppressions
/// from a CSV.
#[handler]
pub async fn import_suppression_csv(
    _user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    mut multipart: poem::web::Multipart,
    Data(state): Data<&AppState>,
) -> Response {
    let mut bytes: Vec<u8> = Vec::new();
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("csv") {
            if let Ok(b) = field.bytes().await {
                bytes = b;
                break;
            }
        }
    }
    if bytes.is_empty() {
        return redirect(&format!(
            "/app/brands/{brand_token}/suppression-list?e=empty"
        ));
    }
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(std::io::Cursor::new(bytes));
    match settings::import_suppression_csv(&state.db, brand_token.raw(), &mut rdr).await {
        Ok((ins, _)) => redirect(&format!(
            "/app/brands/{brand_token}/suppression-list?imported={ins}"
        )),
        Err(e) => {
            tracing::error!(error = ?e, "suppression csv import failed");
            redirect(&format!(
                "/app/brands/{brand_token}/suppression-list?e=import"
            ))
        }
    }
}

/// `POST /app/brands/:id/blocked-domains/import` — bulk-add blocked
/// domains from a CSV.
#[handler]
pub async fn import_blocked_domains_csv(
    _user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    mut multipart: poem::web::Multipart,
    Data(state): Data<&AppState>,
) -> Response {
    let mut bytes: Vec<u8> = Vec::new();
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("csv") {
            if let Ok(b) = field.bytes().await {
                bytes = b;
                break;
            }
        }
    }
    if bytes.is_empty() {
        return redirect(&format!(
            "/app/brands/{brand_token}/blocked-domains?e=empty"
        ));
    }
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(std::io::Cursor::new(bytes));
    match settings::import_blocked_domains_csv(&state.db, brand_token.raw(), &mut rdr).await {
        Ok((ins, _)) => redirect(&format!(
            "/app/brands/{brand_token}/blocked-domains?imported={ins}"
        )),
        Err(e) => {
            tracing::error!(error = ?e, "blocked-domains csv import failed");
            redirect(&format!(
                "/app/brands/{brand_token}/blocked-domains?e=import"
            ))
        }
    }
}

#[handler]
pub async fn settings_suppression_redirect(
    _user: CurrentUser,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let chrome = crate::routes::admin::build_chrome(state, jar, req.uri().path()).await;
    if let Some(brand) = chrome.current {
        redirect(&format!("/app/brands/{}/suppression-list", brand.token))
    } else {
        redirect("/app/brands")
    }
}

#[handler]
pub async fn settings_blocked_domains_redirect(
    _user: CurrentUser,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let chrome = crate::routes::admin::build_chrome(state, jar, req.uri().path()).await;
    if let Some(brand) = chrome.current {
        redirect(&format!("/app/brands/{}/blocked-domains", brand.token))
    } else {
        redirect("/app/brands")
    }
}

fn redirect(location: &str) -> Response {
    Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", location)
        .body(())
}

