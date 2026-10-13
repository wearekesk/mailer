use leptos::prelude::*;
use poem::{
    handler,
    http::StatusCode,
    web::{Data, Form, Multipart, Path},
    Response,
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
use serde::Deserialize;

use crate::routes::admin::session::CurrentUser;
use crate::services::brands::{self, BrandInput};
use crate::utils::opaque_id::{Brand, OpaqueId};
use crate::AppState;

use super::views::*;


/// `GET /app/brands` — table listing.
#[handler]
pub async fn list_page(
    _user: CurrentUser,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let brands = match brands::list_brands(&state.db).await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(error = ?e, "list_brands failed");
            Vec::new()
        }
    };
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Brands", move || {
        view! { <BrandsListPage brands=brands/> }
    })
    .await
}

/// `GET /app/brands/new`
#[handler]
pub async fn new_form_page(
    _user: CurrentUser,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "New brand", || {
        view! { <BrandFormPage/> }
    })
    .await
}

/// `GET /app/brands/:id/edit`
#[handler]
pub async fn edit_form_page(
    user: CurrentUser,
    Path(brand_id): Path<OpaqueId<Brand>>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let row = match brands::find_brand(&state.db, brand_id.raw()).await {
        Ok(Some(b)) => b,
        _ => return redirect("/app/brands"),
    };
    let sub_users = crate::services::auth::list_sub_users(&state.db, brand_id.raw())
        .await
        .unwrap_or_default();

    // Best-effort lookup of SES verification status for this brand's from_email.
    let ses_status = match (&user.0.s3_key, &user.0.s3_secret, row.from_email.as_deref()) {
        (Some(k), Some(s), Some(em)) if !k.is_empty() && !s.is_empty() && !em.is_empty() => {
            let region = crate::services::ses::region_from_endpoint(
                user.0.ses_endpoint.as_deref().unwrap_or(""),
            );
            crate::services::ses::verification_status(k, s, &region, em)
                .await
                .ok()
        }
        _ => None,
    };

    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Edit brand", move || {
        view! { <BrandFormPage existing=row sub_users=sub_users ses_status=ses_status/> }
    })
    .await
}

/// `POST /app/brands/new`
#[handler]
pub async fn create(
    user: CurrentUser,
    Form(form): Form<BrandForm>,
    Data(state): Data<&AppState>,
) -> Response {
    if form.app_name.trim().is_empty() {
        return redirect("/app/brands/new?e=name");
    }
    let input: BrandInput = form.into();
    match brands::create_brand(&state.db, user.0.id, input).await {
        Ok(_) => redirect("/app/brands"),
        Err(e) => {
            tracing::error!(error = ?e, "create_brand failed");
            redirect("/app/brands/new?e=db")
        }
    }
}

/// `POST /app/brands/:id/edit`
#[handler]
pub async fn update(
    _user: CurrentUser,
    Path(brand_id): Path<OpaqueId<Brand>>,
    Form(form): Form<BrandForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let input: BrandInput = form.into();
    match brands::update_brand(&state.db, brand_id.raw(), input).await {
        Ok(()) => redirect("/app/brands"),
        Err(e) => {
            tracing::error!(error = ?e, brand = brand_id.raw(), "update_brand failed");
            redirect(&format!("/app/brands/{}/edit?e=db", brand_id))
        }
    }
}

/// `POST /app/brands/:id/delete`
#[handler]
pub async fn delete(
    _user: CurrentUser,
    Path(brand_id): Path<OpaqueId<Brand>>,
    Data(state): Data<&AppState>,
) -> Response {
    match brands::delete_brand(&state.db, brand_id.raw()).await {
        Ok(()) => redirect("/app/brands"),
        Err(e) => {
            tracing::error!(error = ?e, brand = brand_id.raw(), "delete_brand failed");
            redirect(&format!("/app/brands/{}/edit?e=delete", brand_id))
        }
    }
}

/// `POST /app/brands/:id/logo` — multipart upload of a brand logo.
#[handler]
pub async fn upload_logo(
    _user: CurrentUser,
    Path(brand_id): Path<OpaqueId<Brand>>,
    mut multipart: Multipart,
    Data(state): Data<&AppState>,
) -> Response {
    let mut bytes: Vec<u8> = Vec::new();
    let mut original_name: Option<String> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("logo") {
            original_name = field.file_name().map(|s| s.to_string());
            if let Ok(b) = field.bytes().await {
                bytes = b;
            }
            break;
        }
    }
    if bytes.is_empty() {
        return redirect(&format!("/app/brands/{brand_id}/edit?e=empty_logo"));
    }
    if bytes.len() > 5 * 1024 * 1024 {
        return redirect(&format!("/app/brands/{brand_id}/edit?e=logo_too_large"));
    }

    // Pick a safe extension from the original filename or default to png.
    let ext = original_name
        .as_deref()
        .and_then(|n| n.rsplit('.').next())
        .map(|e| e.to_ascii_lowercase())
        .filter(|e| matches!(e.as_str(), "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp"))
        .unwrap_or_else(|| "png".to_string());

    let dir = std::path::Path::new("./uploads/logos");
    if let Err(e) = std::fs::create_dir_all(dir) {
        tracing::warn!(error = ?e, "failed to create logo dir");
        return redirect(&format!("/app/brands/{brand_id}/edit?e=logo_io"));
    }
    let filename = format!("{}.{}", brand_id.raw(), ext);
    let path = dir.join(&filename);
    if let Err(e) = std::fs::write(&path, &bytes) {
        tracing::warn!(error = ?e, "logo write failed");
        return redirect(&format!("/app/brands/{brand_id}/edit?e=logo_io"));
    }

    if let Ok(Some(row)) = crate::database::entities::apps::Entity::find_by_id(brand_id.raw())
        .one(&state.db)
        .await
    {
        let mut active: crate::database::entities::apps::ActiveModel = row.into();
        active.brand_logo_filename = Set(Some(filename));
        let _ = active.update(&state.db).await;
    }
    redirect(&format!("/app/brands/{brand_id}/edit?i=logo_uploaded"))
}

/// `GET /app/brands/:id/select` — set the topbar brand-switcher cookie
/// to this brand and bounce to its lists page.
#[handler]
pub async fn select_brand(
    _user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    jar: &poem::web::cookie::CookieJar,
) -> Response {
    crate::routes::admin::set_current_brand_cookie(jar, brand_token.raw());
    redirect(&format!("/app/brands/{brand_token}/lists"))
}

/// `POST /app/brands/:id/logo/delete`
#[handler]
pub async fn delete_logo(
    _user: CurrentUser,
    Path(brand_id): Path<OpaqueId<Brand>>,
    Data(state): Data<&AppState>,
) -> Response {
    if let Ok(Some(row)) = crate::database::entities::apps::Entity::find_by_id(brand_id.raw())
        .one(&state.db)
        .await
    {
        if let Some(name) = row.brand_logo_filename.clone().filter(|s| !s.is_empty()) {
            let path = std::path::PathBuf::from("./uploads/logos").join(&name);
            let _ = std::fs::remove_file(path);
        }
        let mut active: crate::database::entities::apps::ActiveModel = row.into();
        active.brand_logo_filename = Set(Some(String::new()));
        let _ = active.update(&state.db).await;
    }
    redirect(&format!("/app/brands/{brand_id}/edit?i=logo_removed"))
}

// ── SES verification ────────────────────────────────────────────────────────

/// `POST /app/brands/:id/ses/verify`
///
/// Triggers a `CreateEmailIdentity` for the brand's `from_email` against the
/// main user's AWS credentials. SES delivers a confirmation email; the
/// recipient must click the link before SES will accept outbound mail.
#[handler]
pub async fn ses_verify(
    user: CurrentUser,
    Path(brand_id): Path<OpaqueId<Brand>>,
    Data(state): Data<&AppState>,
) -> Response {
    let brand = match brands::find_brand(&state.db, brand_id.raw()).await.ok().flatten() {
        Some(b) => b,
        None => return redirect("/app/brands"),
    };
    let from_email = match brand.from_email.clone().filter(|s| !s.is_empty()) {
        Some(e) => e,
        None => return redirect(&format!("/app/brands/{brand_id}/edit?e=no_from_email")),
    };

    let key = user.0.s3_key.clone().unwrap_or_default();
    let secret = user.0.s3_secret.clone().unwrap_or_default();
    if key.is_empty() || secret.is_empty() {
        return redirect(&format!("/app/brands/{brand_id}/edit?e=no_aws_creds"));
    }
    let region =
        crate::services::ses::region_from_endpoint(user.0.ses_endpoint.as_deref().unwrap_or(""));

    if let Err(e) = crate::services::ses::verify_identity(&key, &secret, &region, &from_email).await {
        tracing::warn!(error = ?e, "SES verify failed");
        return redirect(&format!("/app/brands/{brand_id}/edit?e=ses_verify_failed"));
    }
    redirect(&format!("/app/brands/{brand_id}/edit?i=ses_verify_sent"))
}

// ── Sub-users ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SubUserForm {
    pub email: String,
    #[serde(default)]
    pub name: Option<String>,
    pub password: String,
}

/// `POST /app/brands/:id/users/new`
#[handler]
pub async fn create_sub_user(
    user: CurrentUser,
    Path(brand_id): Path<OpaqueId<Brand>>,
    Form(form): Form<SubUserForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let name = form.name.and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
    });
    match crate::services::auth::create_sub_user(
        &state.db,
        user.0.id,
        brand_id.raw(),
        &form.email,
        &form.password,
        name,
    )
    .await
    {
        Ok(_) => redirect(&format!("/app/brands/{brand_id}/edit?i=user_added")),
        Err(crate::utils::errors::AppError::BadRequest(msg)) => {
            tracing::warn!(error = %msg, "sub-user create failed");
            redirect(&format!("/app/brands/{brand_id}/edit?e=user_dup"))
        }
        Err(e) => {
            tracing::error!(error = ?e, "sub-user create failed");
            redirect(&format!("/app/brands/{brand_id}/edit?e=db"))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UserBrandQuery {
    pub brand: String,
}

/// `POST /app/users/:id/delete?brand=…`
#[handler]
pub async fn delete_sub_user(
    _user: CurrentUser,
    Path(uid): Path<i32>,
    poem::web::Query(q): poem::web::Query<UserBrandQuery>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = crate::services::auth::delete_sub_user(&state.db, uid).await;
    redirect(&format!("/app/brands/{}/edit?i=user_removed", q.brand))
}

/// `POST /app/brands/:id/duplicate`
#[handler]
pub async fn duplicate(
    _user: CurrentUser,
    Path(brand_id): Path<OpaqueId<Brand>>,
    Data(state): Data<&AppState>,
) -> Response {
    match brands::duplicate_brand(&state.db, brand_id.raw()).await {
        Ok(_) => redirect("/app/brands"),
        Err(e) => {
            tracing::error!(error = ?e, brand = brand_id.raw(), "duplicate_brand failed");
            redirect("/app/brands?e=duplicate")
        }
    }
}

fn redirect(location: &str) -> Response {
    Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", location)
        .body(())
}
