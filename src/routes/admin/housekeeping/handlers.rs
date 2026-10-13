use super::views::*;
use leptos::prelude::*;
use poem::{
    handler,
    http::StatusCode,
    web::{Data, Form, Path, Query},
    Response,
};
use crate::routes::admin::session::CurrentUser;
use crate::services::{brands, housekeeping, lists};
use crate::utils::opaque_id::{Brand, List, OpaqueId};
use crate::AppState;

/// `GET /app/lists/:list_id/housekeeping`
#[handler]
pub async fn page(
    _user: CurrentUser,
    Path(list_token): Path<OpaqueId<List>>,
    Query(q): Query<ResultQuery>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let list = match lists::find_list(&state.db, list_token.raw()).await.ok().flatten() {
        Some(l) => l,
        None => return redirect("/app/brands"),
    };
    let brand_id = list.app.unwrap_or_default();
    let brand = brands::find_brand(&state.db, brand_id).await.ok().flatten();

    let result = match (q.kind.as_deref(), q.n) {
        (Some(k), Some(n)) => Some(HousekeepingResult {
            label: pretty_kind(k).to_string(),
            affected: n,
        }),
        _ => None,
    };

    let list_name = list.name.unwrap_or_default();
    let brand_name = brand.and_then(|b| b.app_name).unwrap_or_default();
    let list_token_str = list_token.to_string();
    let brand_token_str = OpaqueId::<Brand>::new(brand_id).to_string();

    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Housekeeping", move || {
        view! {
            <HousekeepingPage
                list_token=list_token_str list_name=list_name
                brand_token=brand_token_str brand_name=brand_name
                result=result
            />
        }
    })
    .await
}

fn pretty_kind(k: &str) -> &str {
    match k {
        "inactive" => "Inactive purge",
        "unconfirmed" => "Unconfirmed purge",
        "no-opens" => "No-opens purge",
        "no-clicks" => "No-clicks purge",
        "dedupe" => "Dedupe",
        _ => "Operation",
    }
}

/// `POST /app/lists/:list_id/housekeeping/inactive`
#[handler]
pub async fn purge_inactive(
    _user: CurrentUser,
    Path(list_token): Path<OpaqueId<List>>,
    Form(form): Form<DaysForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let n = housekeeping::purge_inactive(&state.db, list_token.raw(), form.days)
        .await
        .unwrap_or(0);
    redirect(&format!(
        "/app/lists/{list_token}/housekeeping?kind=inactive&n={n}"
    ))
}

/// `POST /app/lists/:list_id/housekeeping/unconfirmed`
#[handler]
pub async fn purge_unconfirmed(
    _user: CurrentUser,
    Path(list_token): Path<OpaqueId<List>>,
    Form(form): Form<DaysForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let n = housekeeping::purge_unconfirmed(&state.db, list_token.raw(), form.days)
        .await
        .unwrap_or(0);
    redirect(&format!(
        "/app/lists/{list_token}/housekeeping?kind=unconfirmed&n={n}"
    ))
}

/// `POST /app/lists/:list_id/housekeeping/no-opens`
#[handler]
pub async fn purge_no_opens(
    _user: CurrentUser,
    Path(list_token): Path<OpaqueId<List>>,
    Form(form): Form<DaysForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let n = housekeeping::purge_no_opens(&state.db, list_token.raw(), form.days)
        .await
        .unwrap_or(0);
    redirect(&format!(
        "/app/lists/{list_token}/housekeeping?kind=no-opens&n={n}"
    ))
}

/// `POST /app/lists/:list_id/housekeeping/no-clicks`
#[handler]
pub async fn purge_no_clicks(
    _user: CurrentUser,
    Path(list_token): Path<OpaqueId<List>>,
    Form(form): Form<DaysForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let n = housekeeping::purge_no_clicks(&state.db, list_token.raw(), form.days)
        .await
        .unwrap_or(0);
    redirect(&format!(
        "/app/lists/{list_token}/housekeeping?kind=no-clicks&n={n}"
    ))
}

/// `POST /app/lists/:list_id/housekeeping/dedupe`
#[handler]
pub async fn dedupe(
    _user: CurrentUser,
    Path(list_token): Path<OpaqueId<List>>,
    Data(state): Data<&AppState>,
) -> Response {
    let n = housekeeping::dedupe_list(&state.db, list_token.raw())
        .await
        .unwrap_or(0);
    redirect(&format!(
        "/app/lists/{list_token}/housekeeping?kind=dedupe&n={n}"
    ))
}

/// `GET /app/brands/:brand_id/housekeeping/conflicts`
#[handler]
pub async fn conflicts(
    _user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let brand = match brands::find_brand(&state.db, brand_token.raw()).await.ok().flatten() {
        Some(b) => b,
        None => return redirect("/app/brands"),
    };
    let rows = housekeeping::detect_conflicts(&state.db, brand_token.raw())
        .await
        .unwrap_or_default();
    let token = brand_token.to_string();
    let brand_name = brand.app_name.unwrap_or_default();
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Conflicts", move || {
        view! { <ConflictReportPage brand_token=token brand_name=brand_name rows=rows/> }
    })
    .await
}

fn redirect(location: &str) -> Response {
    Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", location)
        .body(())
}
