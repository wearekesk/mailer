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
use crate::services::templates::{self, TemplateInput};
use crate::utils::opaque_id::{Brand, OpaqueId};
use crate::AppState;

#[handler]
pub async fn list_for_brand_page(
    _user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let brand = match crate::services::brands::find_brand(&state.db, brand_token.raw()).await {
        Ok(Some(b)) => b,
        _ => return redirect("/app/brands"),
    };
    let rows = templates::list_for_brand(&state.db, brand_token.raw())
        .await
        .unwrap_or_default();
    let token = brand_token.to_string();
    let brand_name = brand.app_name.unwrap_or_default();
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Templates", move || {
        view! { <TemplatesListPage brand_token=token brand_name=brand_name rows=rows/> }
    })
    .await
}

#[handler]
pub async fn new_form(
    _user: CurrentUser,
    Query(q): Query<BrandQuery>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let brand_token = q.brand;
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "New template", move || {
        view! { <TemplateFormPage brand_token=brand_token/> }
    })
    .await
}

#[handler]
pub async fn edit_form(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let row = match templates::find(&state.db, id).await {
        Ok(Some(t)) => t,
        _ => return redirect("/app/brands"),
    };
    let brand_id = row.app.unwrap_or_default();
    let brand_token = OpaqueId::<Brand>::new(brand_id).to_string();
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Edit template", move || {
        view! { <TemplateFormPage brand_token=brand_token existing=row/> }
    })
    .await
}

#[handler]
pub async fn create(
    user: CurrentUser,
    Form(form): Form<TemplateForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let brand_token = form.brand.clone().unwrap_or_default();
    let brand_id = match brand_token.parse::<OpaqueId<Brand>>() {
        Ok(b) => b.raw(),
        Err(_) => return redirect("/app/brands"),
    };
    let input: TemplateInput = form.into();
    if input.template_name.is_empty() {
        return redirect(&format!("/app/templates/new?brand={brand_token}"));
    }
    let _ = templates::create(&state.db, user.0.id, brand_id, input).await;
    redirect(&format!("/app/brands/{brand_token}/templates"))
}

#[handler]
pub async fn update(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Form(form): Form<TemplateForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let input: TemplateInput = form.into();
    let _ = templates::update(&state.db, id, input).await;
    redirect(&format!("/app/templates/{id}/edit"))
}

#[handler]
pub async fn delete(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Query(q): Query<BrandQuery>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = templates::delete(&state.db, id).await;
    redirect(&format!("/app/brands/{}/templates", q.brand))
}

/// `POST /app/templates/:id/use` — clone the template into a new draft
/// campaign under the same brand and redirect to the campaign editor.
#[handler]
pub async fn use_template(
    user: CurrentUser,
    Path(id): Path<i32>,
    Data(state): Data<&AppState>,
) -> Response {
    use crate::database::entities::campaigns;
    use sea_orm::ActiveValue::Set;

    let template = match templates::find(&state.db, id).await {
        Ok(Some(t)) => t,
        _ => return redirect("/app/brands"),
    };
    let brand_id = template.app.unwrap_or_default();
    let now = chrono::Utc::now().timestamp().to_string();
    let active = campaigns::ActiveModel {
        user_id: Set(Some(user.0.id)),
        app: Set(Some(brand_id)),
        title: Set(template.template_name.clone()),
        subject: Set(template.template_name.clone()),
        plain_text: Set(template.plain_text.clone()),
        html_text: Set(template.html_text.clone()),
        from_name: Set(template.from_name.clone()),
        from_email: Set(template.from_email.clone()),
        reply_to: Set(template.reply_to.clone()),
        wysiwyg: Set(Some(1)),
        opens_tracking: Set(Some(1)),
        links_tracking: Set(Some(1)),
        timeout_check: Set(Some(now)),
        ..Default::default()
    };
    use sea_orm::ActiveModelTrait;
    match active.insert(&state.db).await {
        Ok(c) => {
            let token = crate::utils::opaque_id::OpaqueId::<crate::utils::opaque_id::Campaign>::new(c.id);
            redirect(&format!("/app/campaigns/{token}/edit"))
        }
        Err(e) => {
            tracing::error!(error = ?e, "use_template insert failed");
            let brand_token = OpaqueId::<Brand>::new(brand_id).to_string();
            redirect(&format!("/app/brands/{brand_token}/templates?e=use_failed"))
        }
    }
}

/// `GET /app/templates/:id/preview` — render the template body with a
/// dummy subscriber context and return a stand-alone HTML preview.
/// `?view=plain` opens the plain-text pane on first paint.
#[handler]
pub async fn preview(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Query(q): Query<PreviewQuery>,
    Data(state): Data<&AppState>,
) -> Response {
    use crate::services::personalization::Context;
    use crate::services::preview::{build_preview, dummy_subscriber, PreviewMeta, PreviewMode};

    let row = match templates::find(&state.db, id).await {
        Ok(Some(t)) => t,
        _ => return redirect("/app/brands"),
    };

    let dummy = dummy_subscriber("preview@mailer.local", "Preview Recipient");
    let app_url = state.config.app_url.clone();
    let ctx = Context::build(&dummy, 0, 0, "", &app_url);

    let from_name = row.from_name.clone().unwrap_or_default();
    let from_email = row.from_email.clone().unwrap_or_default();
    let subject = row.template_name.clone().unwrap_or_default();
    let html_body = row.html_text.clone().unwrap_or_default();
    let plain_body = row.plain_text.clone().unwrap_or_default();
    let back_href = format!("/app/templates/{id}/edit");
    let mode = PreviewMode::from_query(q.view.as_deref());

    let html = build_preview(
        &html_body,
        &plain_body,
        mode,
        PreviewMeta {
            kind: "Template",
            subject: &subject,
            from_name: &from_name,
            from_email: &from_email,
            back_href: &back_href,
        },
        &ctx,
    );
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("X-Robots-Tag", "none")
        .body(html)
}

#[derive(Debug, Deserialize)]
pub struct PreviewQuery {
    pub view: Option<String>,
}

fn redirect(location: &str) -> Response {
    Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", location)
        .body(())
}
