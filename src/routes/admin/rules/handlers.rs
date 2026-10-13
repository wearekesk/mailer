use super::views::*;
use leptos::prelude::*;
use poem::{
    handler,
    http::StatusCode,
    web::{Data, Form, Path, Query},
    Response,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use crate::routes::admin::session::CurrentUser;
use crate::services::rules as rules_svc;
use crate::services::rules::RuleInput;
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
    let rows = rules_svc::list_for_brand(&state.db, brand_token.raw())
        .await
        .unwrap_or_default();

    // Populate the new dropdowns: every list in the brand, every
    // autoresponder, and the union of custom-field names across the
    // brand's lists (de-duplicated, case-insensitive).
    let lists_in_brand: Vec<ListOption> = crate::services::lists::list_for_brand(
        &state.db,
        brand_token.raw(),
        true,
    )
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|l| ListOption { id: l.id, name: l.name })
    .collect();

    let ares_in_brand: Vec<AresOption> = crate::services::autoresponders::list_for_brand(
        &state.db,
        brand_token.raw(),
    )
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|a| AresOption { id: a.id, name: a.name.unwrap_or_default() })
    .collect();

    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut custom_fields: Vec<String> = Vec::new();
    for list_row in crate::database::entities::lists::Entity::find()
        .filter(crate::database::entities::lists::Column::App.eq(brand_token.raw()))
        .all(&state.db)
        .await
        .unwrap_or_default()
    {
        for cf in crate::services::lists::parse_custom_fields(
            list_row.custom_fields.as_deref().unwrap_or(""),
        ) {
            if seen.insert(cf.name.to_ascii_lowercase()) {
                custom_fields.push(cf.name);
            }
        }
    }

    let token = brand_token.to_string();
    let brand_name = brand.app_name.unwrap_or_default();
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Rules", move || {
        view! {
            <RulesListPage
                brand_token=token brand_name=brand_name rows=rows
                lists=lists_in_brand
                autoresponders=ares_in_brand
                custom_fields=custom_fields
            />
        }
    })
    .await
}

#[handler]
pub async fn save(
    _user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    Form(form): Form<RuleForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = rules_svc::save(
        &state.db,
        RuleInput {
            brand_id: brand_token.raw(),
            app_id: brand_token.raw(),
            trigger: form.trigger,
            action: form.action,
            notification_email: nonempty(form.notification_email),
            webhook_url: nonempty(form.webhook_url),
            list_id: form.list_id.as_deref().and_then(|s| s.trim().parse().ok()),
            ares_id: form.ares_id.as_deref().and_then(|s| s.trim().parse().ok()),
        },
    )
    .await;
    redirect(&format!("/app/brands/{brand_token}/rules"))
}

#[handler]
pub async fn toggle(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Query(q): Query<BrandQuery>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = rules_svc::toggle(&state.db, id).await;
    redirect(&format!("/app/brands/{}/rules", q.brand))
}

#[handler]
pub async fn delete(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Query(q): Query<BrandQuery>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = rules_svc::delete(&state.db, id).await;
    redirect(&format!("/app/brands/{}/rules", q.brand))
}

fn nonempty(s: Option<String>) -> Option<String> {
    s.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

fn redirect(location: &str) -> Response {
    Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", location)
        .body(())
}
