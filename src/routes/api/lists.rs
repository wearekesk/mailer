use poem::{
    handler,
    web::{Data, Json, Path, Query},
    IntoResponse, Response,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::routes::api::{auth::ApiAuth, bad_request, internal_error, not_found};
use crate::services::{brands, lists::{self, ListInput}};
use crate::utils::opaque_id::{Brand, List, OpaqueId};
use crate::AppState;

#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
pub struct ListQuery {
    #[serde(default)]
    pub include_hidden: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ListResponse {
    pub id: OpaqueId<List>,
    pub name: String,
    pub subscriber_count: i64,
    pub hide: bool,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
pub struct CreateListBody {
    pub name: String,
    #[serde(default)]
    pub opt_in: Option<i16>,
    #[serde(default)]
    pub subscribed_url: Option<String>,
    #[serde(default)]
    pub confirm_url: Option<String>,
    #[serde(default)]
    pub already_subscribed_url: Option<String>,
    #[serde(default)]
    pub unsubscribed_url: Option<String>,
    #[serde(default)]
    pub thankyou: Option<i16>,
    #[serde(default)]
    pub thankyou_subject: Option<String>,
    #[serde(default)]
    pub thankyou_message: Option<String>,
    #[serde(default)]
    pub goodbye: Option<i16>,
    #[serde(default)]
    pub goodbye_subject: Option<String>,
    #[serde(default)]
    pub goodbye_message: Option<String>,
    #[serde(default)]
    pub confirmation_subject: Option<String>,
    #[serde(default)]
    pub confirmation_email: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
pub struct UpdateListBody {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub opt_in: Option<i16>,
    #[serde(default)]
    pub subscribed_url: Option<String>,
    #[serde(default)]
    pub confirm_url: Option<String>,
    #[serde(default)]
    pub already_subscribed_url: Option<String>,
    #[serde(default)]
    pub unsubscribed_url: Option<String>,
    #[serde(default)]
    pub thankyou: Option<i16>,
    #[serde(default)]
    pub thankyou_subject: Option<String>,
    #[serde(default)]
    pub thankyou_message: Option<String>,
    #[serde(default)]
    pub goodbye: Option<i16>,
    #[serde(default)]
    pub goodbye_subject: Option<String>,
    #[serde(default)]
    pub goodbye_message: Option<String>,
    #[serde(default)]
    pub confirmation_subject: Option<String>,
    #[serde(default)]
    pub confirmation_email: Option<String>,
}

/// `GET /api/brands/:brand_id/lists?include_hidden=true` — every list under brand.
#[handler]
pub async fn list_for_brand(
    _auth: ApiAuth,
    Path(brand_id): Path<OpaqueId<Brand>>,
    Query(q): Query<ListQuery>,
    Data(state): Data<&AppState>,
) -> Response {
    let brand_raw = brand_id.raw();
    match brands::find_brand(&state.db, brand_raw).await {
        Ok(Some(_)) => {}
        Ok(None) => return not_found("Brand does not exist."),
        Err(e) => {
            tracing::error!(error = ?e, "find_brand failed");
            return internal_error("Database error.");
        }
    }

    match lists::list_brand_lists(&state.db, brand_raw, q.include_hidden).await {
        Ok(rows) => {
            let body: Vec<ListResponse> = rows
                .into_iter()
                .map(|l| ListResponse {
                    id: OpaqueId::new(l.id),
                    name: l.name,
                    subscriber_count: l.subscriber_count,
                    hide: l.hide,
                })
                .collect();
            Json(body).into_response()
        }
        Err(e) => {
            tracing::error!(error = ?e, "list_brand_lists failed");
            internal_error("Database error.")
        }
    }
}

/// `GET /api/lists/:list_id` — fetch list details.
#[handler]
pub async fn get(
    _auth: ApiAuth,
    Path(list_id): Path<OpaqueId<List>>,
    Data(state): Data<&AppState>,
) -> Response {
    let raw = list_id.raw();
    match lists::find_list(&state.db, raw).await {
        Ok(Some(l)) => {
            let count = crate::services::subscribers::active_count(&state.db, raw).await.unwrap_or(0);
            let resp = ListResponse {
                id: OpaqueId::new(l.id),
                name: l.name.unwrap_or_default(),
                subscriber_count: count,
                hide: l.hide.unwrap_or(0) == 1,
            };
            Json(resp).into_response()
        }
        Ok(None) => not_found("List does not exist."),
        Err(e) => {
            tracing::error!(error = ?e, "find_list failed");
            internal_error("Database error.")
        }
    }
}

/// `POST /api/brands/:brand_id/lists` — create a new list for brand.
#[handler]
pub async fn create(
    auth: ApiAuth,
    Path(brand_id): Path<OpaqueId<Brand>>,
    Json(body): Json<CreateListBody>,
    Data(state): Data<&AppState>,
) -> Response {
    let brand_raw = brand_id.raw();
    match brands::find_brand(&state.db, brand_raw).await {
        Ok(Some(_)) => {}
        Ok(None) => return not_found("Brand does not exist."),
        Err(e) => {
            tracing::error!(error = ?e, "find_brand failed");
            return internal_error("Database error.");
        }
    }

    if body.name.trim().is_empty() {
        return bad_request("List name cannot be empty.");
    }

    let input = ListInput {
        name: body.name.trim().to_string(),
        opt_in: body.opt_in.unwrap_or(1),
        subscribed_url: body.subscribed_url,
        confirm_url: body.confirm_url,
        already_subscribed_url: body.already_subscribed_url,
        unsubscribed_url: body.unsubscribed_url,
        thankyou: body.thankyou.unwrap_or(0),
        thankyou_subject: body.thankyou_subject,
        thankyou_message: body.thankyou_message,
        goodbye: body.goodbye.unwrap_or(0),
        goodbye_subject: body.goodbye_subject,
        goodbye_message: body.goodbye_message,
        confirmation_subject: body.confirmation_subject,
        confirmation_email: body.confirmation_email,
        ..Default::default()
    };

    match lists::create_list(&state.db, auth.0.id, brand_raw, input).await {
        Ok(new_id) => {
            let resp = ListResponse {
                id: OpaqueId::new(new_id),
                name: body.name,
                subscriber_count: 0,
                hide: false,
            };
            Json(resp).into_response()
        }
        Err(e) => {
            tracing::error!(error = ?e, "create_list failed");
            internal_error("Failed to create list.")
        }
    }
}

/// `PUT /api/lists/:list_id` — update an existing list.
#[handler]
pub async fn update(
    _auth: ApiAuth,
    Path(list_id): Path<OpaqueId<List>>,
    Json(body): Json<UpdateListBody>,
    Data(state): Data<&AppState>,
) -> Response {
    let raw = list_id.raw();
    let existing = match lists::find_list(&state.db, raw).await {
        Ok(Some(l)) => l,
        Ok(None) => return not_found("List does not exist."),
        Err(e) => {
            tracing::error!(error = ?e, "find_list failed");
            return internal_error("Database error.");
        }
    };

    let input = ListInput {
        name: body.name.unwrap_or_else(|| existing.name.clone().unwrap_or_default()),
        opt_in: body.opt_in.unwrap_or_else(|| existing.opt_in.unwrap_or(1)),
        subscribed_url: body.subscribed_url.or(existing.subscribed_url),
        confirm_url: body.confirm_url.or(existing.confirm_url),
        already_subscribed_url: body.already_subscribed_url.or(existing.already_subscribed_url),
        unsubscribed_url: body.unsubscribed_url.or(existing.unsubscribed_url),
        thankyou: body.thankyou.unwrap_or_else(|| existing.thankyou.unwrap_or(0)),
        thankyou_subject: body.thankyou_subject.or(existing.thankyou_subject),
        thankyou_message: body.thankyou_message.or(existing.thankyou_message),
        goodbye: body.goodbye.unwrap_or_else(|| existing.goodbye.unwrap_or(0)),
        goodbye_subject: body.goodbye_subject.or(existing.goodbye_subject),
        goodbye_message: body.goodbye_message.or(existing.goodbye_message),
        confirmation_subject: body.confirmation_subject.or(existing.confirmation_subject),
        confirmation_email: body.confirmation_email.or(existing.confirmation_email),
        ..Default::default()
    };

    match lists::update_list(&state.db, raw, input).await {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "update_list failed");
            internal_error("Failed to update list.")
        }
    }
}

/// `DELETE /api/lists/:list_id` — delete list and its subscribers.
#[handler]
pub async fn delete(
    _auth: ApiAuth,
    Path(list_id): Path<OpaqueId<List>>,
    Data(state): Data<&AppState>,
) -> Response {
    let raw = list_id.raw();
    match lists::find_list(&state.db, raw).await {
        Ok(Some(_)) => {}
        Ok(None) => return not_found("List does not exist."),
        Err(e) => {
            tracing::error!(error = ?e, "find_list failed");
            return internal_error("Database error.");
        }
    }

    match lists::delete_list(&state.db, raw).await {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "delete_list failed");
            internal_error("Failed to delete list.")
        }
    }
}
