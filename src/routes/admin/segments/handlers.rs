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
use crate::services::segments::{self, SegmentInput};
use crate::utils::opaque_id::{Brand, OpaqueId};
use crate::AppState;

/// `GET /app/lists/:id/segments`
#[handler]
pub async fn list_for_list_page(
    _user: CurrentUser,
    Path(list_token): Path<crate::utils::opaque_id::OpaqueId<crate::utils::opaque_id::List>>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let list = match crate::services::lists::find_list(&state.db, list_token.raw()).await {
        Ok(Some(l)) => l,
        _ => return redirect("/app/brands"),
    };
    let rows = segments::list_for_list(&state.db, list_token.raw())
        .await
        .unwrap_or_default();
    let list_name = list.name.unwrap_or_default();
    let token = list_token.to_string();
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Segments", move || {
        view! { <ListSegmentsPage list_token=token list_name=list_name rows=rows/> }
    })
    .await
}

/// `POST /app/lists/:id/segments/new`
#[handler]
pub async fn list_create(
    _user: CurrentUser,
    Path(list_token): Path<crate::utils::opaque_id::OpaqueId<crate::utils::opaque_id::List>>,
    Form(form): Form<SegmentForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let list = match crate::services::lists::find_list(&state.db, list_token.raw()).await {
        Ok(Some(l)) => l,
        _ => return redirect("/app/brands"),
    };
    let brand_id = list.app.unwrap_or_default();
    let _ = segments::create(
        &state.db,
        brand_id,
        SegmentInput {
            name: form.name.trim().to_string(),
            list_id: list_token.raw(),
        },
    )
    .await;
    redirect(&format!("/app/lists/{list_token}/segments"))
}

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
    let rows = segments::list_for_brand(&state.db, brand_token.raw())
        .await
        .unwrap_or_default();
    let token = brand_token.to_string();
    let brand_name = brand.app_name.unwrap_or_default();
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Segments", move || {
        view! { <SegmentsListPage brand_token=token brand_name=brand_name rows=rows/> }
    })
    .await
}

#[handler]
pub async fn create(
    _user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    Form(form): Form<SegmentForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = segments::create(
        &state.db,
        brand_token.raw(),
        SegmentInput {
            name: form.name.trim().to_string(),
            list_id: form.list_id,
        },
    )
    .await;
    redirect(&format!("/app/brands/{brand_token}/segments"))
}

#[handler]
pub async fn delete(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Query(q): Query<BrandQuery>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = segments::delete(&state.db, id).await;
    redirect(&format!("/app/brands/{}/segments", q.brand))
}

#[derive(Debug, Deserialize)]
pub struct ConditionForm {
    pub group_id: i32,
    pub operator: String,
    pub field: String,
    pub comparison: String,
    #[serde(default)]
    pub val: Option<String>,
}

/// `GET /app/segments/:id/edit` — conditions builder.
#[handler]
pub async fn edit_form(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let segment = match segments::find(&state.db, id).await {
        Ok(Some(s)) => s,
        _ => return redirect("/app/brands"),
    };
    let conditions = segments::list_conditions(&state.db, id).await.unwrap_or_default();
    let brand_token = OpaqueId::<Brand>::new(segment.app.unwrap_or_default()).to_string();

    // Pull the segment's parent list custom fields so the field-picker
    // dropdown can offer them alongside the built-ins.
    let list_id = segment.list.unwrap_or_default();
    let custom_fields: Vec<CustomFieldOption> = if list_id > 0 {
        crate::services::lists::find_list(&state.db, list_id)
            .await
            .ok()
            .flatten()
            .map(|l| {
                crate::services::lists::parse_custom_fields(
                    l.custom_fields.as_deref().unwrap_or(""),
                )
                .into_iter()
                .map(|f| CustomFieldOption {
                    name: f.name,
                    kind: f.kind.as_str().to_string(),
                })
                .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Segment editor", move || {
        view! { <SegmentEditorPage segment=segment conditions=conditions brand_token=brand_token custom_fields=custom_fields/> }
    })
    .await
}

/// `POST /app/segments/:id/conditions/add`
#[handler]
pub async fn add_condition(
    _user: CurrentUser,
    Path(id): Path<i32>,
    Form(form): Form<ConditionForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = segments::add_condition(
        &state.db,
        id,
        crate::services::segments::ConditionInput {
            group_id: form.group_id,
            operator: form.operator,
            field: form.field,
            comparison: form.comparison,
            val: form.val.unwrap_or_default(),
        },
    )
    .await;
    redirect(&format!("/app/segments/{id}/edit"))
}

/// `POST /app/segments/:seg_id/conditions/:cond_id/delete`
#[handler]
pub async fn delete_condition(
    _user: CurrentUser,
    Path((seg_id, cond_id)): Path<(i32, i32)>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = segments::delete_condition(&state.db, cond_id).await;
    redirect(&format!("/app/segments/{seg_id}/edit"))
}

fn redirect(location: &str) -> Response {
    Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", location)
        .body(())
}
