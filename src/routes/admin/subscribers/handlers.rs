use super::views::*;
use leptos::prelude::*;
use poem::{
    handler,
    http::StatusCode,
    web::{Data, Form, Multipart, Path, Query},
    Response,
};
use sea_orm::EntityTrait;
use serde::Deserialize;
use crate::routes::admin::session::CurrentUser;
use crate::services::lists;
use crate::services::subscribers::{self};
use crate::utils::opaque_id::{Brand, List, OpaqueId, Subscriber};
use crate::AppState;

/// `GET /app/subscribers/:list_id`
#[handler]
pub async fn index(
    _user: CurrentUser,
    Path(list_token): Path<OpaqueId<List>>,
    Query(qs): Query<PageQuery>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let list = match lists::find_list(&state.db, list_token.raw()).await {
        Ok(Some(l)) => l,
        _ => return redirect("/app/brands"),
    };
    let brand_id = list.app.unwrap_or_default();
    let brand_name = match crate::services::brands::find_brand(&state.db, brand_id).await {
        Ok(Some(b)) => b.app_name.unwrap_or_default(),
        _ => String::new(),
    };
    let page = qs.page.unwrap_or(0);
    let status = qs
        .status
        .as_deref()
        .map(crate::services::subscribers::StatusFilter::parse)
        .unwrap_or_default();
    let search = qs.q.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(|s| s.to_string());
    let (rows, total) = subscribers::paginated(
        &state.db,
        list_token.raw(),
        page,
        PAGE_SIZE,
        status,
        search.as_deref(),
    )
    .await
    .unwrap_or((Vec::new(), 0));

    let token = list_token.to_string();
    let list_name = list.name.unwrap_or_default();
    let brand_token = OpaqueId::<Brand>::new(brand_id).to_string();
    let status_str = status.as_str().to_string();
    let search_str = search.unwrap_or_default();
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Subscribers", move || {
        view! {
            <SubscribersIndexPage
                list_token=token list_name=list_name
                brand_token=brand_token brand_name=brand_name
                rows=rows page=page total=total
                active_status=status_str search=search_str
            />
        }
    })
    .await
}

/// `GET /app/lists/:id/subscribers/import`
#[handler]
pub async fn import_form(
    _user: CurrentUser,
    Path(list_token): Path<OpaqueId<List>>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let list_name = lists::find_list(&state.db, list_token.raw())
        .await
        .ok()
        .flatten()
        .and_then(|l| l.name)
        .unwrap_or_default();
    let token = list_token.to_string();
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Import subscribers", move || {
        view! { <ImportPage list_token=token list_name=list_name/> }
    })
    .await
}

/// `POST /app/lists/:id/subscribers/import`
#[handler]
pub async fn import_csv(
    user: CurrentUser,
    Path(list_token): Path<OpaqueId<List>>,
    mut multipart: Multipart,
    Data(state): Data<&AppState>,
) -> Response {
    use crate::services::subscribers::ImportMode;

    let mut bytes: Vec<u8> = Vec::new();
    let mut mode = ImportMode::Add;
    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("csv") => {
                if let Ok(b) = field.bytes().await {
                    bytes = b;
                }
            }
            Some("mode") => {
                if let Ok(s) = field.text().await {
                    mode = ImportMode::parse(&s);
                }
            }
            _ => {}
        }
    }
    if bytes.is_empty() {
        return redirect(&format!("/app/lists/{}/subscribers/import?e=empty", list_token));
    }

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(std::io::Cursor::new(bytes));
    match subscribers::import_csv_rows(&state.db, list_token.raw(), user.0.id, &mut rdr, mode)
        .await
    {
        Ok((aff, skip)) => {
            tracing::info!(list = list_token.raw(), affected = aff, skipped = skip, ?mode, "csv import done");
            redirect(&format!("/app/subscribers/{}?imported={}", list_token, aff))
        }
        Err(e) => {
            tracing::error!(error = ?e, "csv import failed");
            redirect(&format!("/app/subscribers/{}?e=import", list_token))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DeleteQuery {
    pub list: String,
}

// ── Edit page ───────────────────────────────────────────────────────────────

#[component]
pub fn SubscriberEditPage(
    subscriber: crate::database::entities::subscribers::Model,
    list_token: String,
    list_name: String,
    custom_fields: Vec<(crate::services::lists::CustomField, String)>,
) -> impl IntoView {
    let stoken = OpaqueId::<Subscriber>::new(subscriber.id).to_string();
    let action = format!("/app/subscribers/{stoken}/edit");
    let cancel_href = format!("/app/subscribers/{list_token}");
    view! {
        <section class="space-y-4">
            <nav class="text-sm text-gray-500 dark:text-gray-400">
                <a href=cancel_href.clone() class="hover:underline">"← "{list_name}</a>
            </nav>
            <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">{subscriber.email.clone().unwrap_or_default()}</h1>

            <form method="POST" action=action class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6 space-y-4 max-w-2xl">
                <div>
                    <label for="name" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Name"</label>
                    <input type="text" id="name" name="name" value=subscriber.name.clone().unwrap_or_default()
                        class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
                </div>

                {if custom_fields.is_empty() {
                    ().into_any()
                } else {
                    view! {
                        <fieldset class="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
                            <legend class="text-sm font-semibold px-2 text-gray-700 dark:text-gray-300">"Custom fields"</legend>
                            <div class="space-y-3">
                                {custom_fields.into_iter().map(|(cf, value)| {
                                    let id = format!("cf_{}", cf.name);
                                    let input_type = match cf.kind {
                                        crate::services::lists::CustomFieldType::Date => "date".to_string(),
                                        _ => "text".to_string(),
                                    };
                                    view! {
                                        <div>
                                            <label for=id.clone() class="block mb-1 text-sm font-medium text-gray-900 dark:text-white">{cf.name.clone()}</label>
                                            <input type=input_type id=id name="custom_value" value=value
                                                class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                        </fieldset>
                    }.into_any()
                }}

                <label class="inline-flex items-center cursor-pointer">
                    <input type="checkbox" name="gdpr" value="1" class="sr-only peer"
                        checked=subscriber.gdpr.unwrap_or(0) == 1/>
                    <div class="relative w-11 h-6 bg-gray-200 peer-focus:ring-4 peer-focus:ring-blue-300 rounded-full peer peer-checked:bg-blue-600 after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white dark:bg-gray-800 after:border-gray-300 dark:border-gray-600 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:after:translate-x-full"></div>
                    <span class="ms-3 text-sm font-medium text-gray-900 dark:text-white">"GDPR consent on file"</span>
                </label>

                <div class="flex justify-end gap-3 pt-2">
                    <a href=cancel_href class="text-sm font-medium text-gray-600 dark:text-gray-400 hover:underline self-center">"Cancel"</a>
                    <button type="submit" class="text-white bg-blue-600 hover:bg-blue-700 font-medium rounded-lg text-sm px-5 py-2.5">"Save changes"</button>
                </div>
            </form>
        </section>
    }
}

#[derive(Debug, Deserialize)]
pub struct SubscriberEditForm {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub gdpr: Option<String>,
    /// Each `<input name="custom_value">` arrives as one entry in the same
    /// order they were rendered (which mirrors the list's custom_fields
    /// packed schema).
    #[serde(default, rename = "custom_value")]
    pub custom_values: Vec<String>,
}

/// `GET /app/subscribers/:id/edit`
#[handler]
pub async fn edit_form(
    _user: CurrentUser,
    Path(subscriber_token): Path<OpaqueId<Subscriber>>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    use crate::database::entities::subscribers as subs_entity;
    use crate::services::lists::parse_custom_fields;

    let row = match subs_entity::Entity::find_by_id(subscriber_token.raw())
        .one(&state.db)
        .await
    {
        Ok(Some(s)) => s,
        _ => return redirect("/app/brands"),
    };
    let list_id = row.list.unwrap_or_default();
    let list = lists::find_list(&state.db, list_id).await.ok().flatten();
    let list_name = list
        .as_ref()
        .and_then(|l| l.name.clone())
        .unwrap_or_default();
    let cf_packed = list
        .and_then(|l| l.custom_fields.clone())
        .unwrap_or_default();
    let cf_meta = parse_custom_fields(&cf_packed);
    let cf_values: Vec<&str> = row
        .custom_fields
        .as_deref()
        .unwrap_or("")
        .split("%s%")
        .collect();
    let zipped: Vec<_> = cf_meta
        .into_iter()
        .enumerate()
        .map(|(i, f)| (f, cf_values.get(i).copied().unwrap_or("").to_string()))
        .collect();

    let list_token = OpaqueId::<List>::new(list_id).to_string();
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Edit subscriber", move || {
        view! {
            <SubscriberEditPage
                subscriber=row
                list_token=list_token
                list_name=list_name
                custom_fields=zipped
            />
        }
    })
    .await
}

/// `POST /app/subscribers/:id/edit`
#[handler]
pub async fn edit_save(
    _user: CurrentUser,
    Path(subscriber_token): Path<OpaqueId<Subscriber>>,
    Form(form): Form<SubscriberEditForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let name = form.name.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let gdpr = form.gdpr.is_some();
    let _ = subscribers::update_inline(
        &state.db,
        subscriber_token.raw(),
        name,
        gdpr,
        &form.custom_values,
    )
    .await;

    let list_token = match crate::database::entities::subscribers::Entity::find_by_id(
        subscriber_token.raw(),
    )
    .one(&state.db)
    .await
    {
        Ok(Some(s)) => OpaqueId::<List>::new(s.list.unwrap_or_default()).to_string(),
        _ => return redirect("/app/brands"),
    };
    redirect(&format!("/app/subscribers/{list_token}"))
}

/// `POST /app/subscribers/:id/unsubscribe`
#[handler]
pub async fn unsubscribe_one(
    _user: CurrentUser,
    Path(subscriber_token): Path<OpaqueId<Subscriber>>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = subscribers::unsubscribe_by_id(&state.db, subscriber_token.raw()).await;
    // We don't know the list-token here without an extra query, so redirect
    // to the brands index — the user-facing forms always include it via
    // the per-row delete handler.
    redirect("/app/brands")
}

/// `POST /app/subscribers/:id/delete?list=…`
#[handler]
pub async fn delete_one(
    _user: CurrentUser,
    Path(subscriber_token): Path<OpaqueId<Subscriber>>,
    Query(q): Query<DeleteQuery>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = subscribers::delete_by_id(&state.db, subscriber_token.raw()).await;
    redirect(&format!("/app/subscribers/{}", q.list))
}

#[derive(Debug, Deserialize)]
pub struct BulkForm {
    pub op: String,
    /// Repeated `ids` checkbox values from the table.
    #[serde(default)]
    pub ids: Vec<i32>,
}

/// `POST /app/lists/:id/subscribers/bulk` — apply `op` to every selected
/// row. `op` is "delete" or "unsubscribe".
#[handler]
pub async fn bulk(
    _user: CurrentUser,
    Path(list_token): Path<OpaqueId<List>>,
    Form(form): Form<BulkForm>,
    Data(state): Data<&AppState>,
) -> Response {
    if form.ids.is_empty() {
        return redirect(&format!("/app/subscribers/{}", list_token));
    }
    match form.op.as_str() {
        "delete" => {
            let _ = subscribers::bulk_delete(&state.db, list_token.raw(), &form.ids).await;
        }
        "unsubscribe" => {
            let _ = subscribers::bulk_unsubscribe(&state.db, list_token.raw(), &form.ids).await;
        }
        _ => {}
    }
    redirect(&format!("/app/subscribers/{}", list_token))
}

fn redirect(location: &str) -> Response {
    Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", location)
        .body(())
}

/// `GET /app/subscribers/sample.csv` — download an example CSV showing
/// the expected column headers.
#[handler]
pub async fn sample_csv(_user: CurrentUser) -> Response {
    const SAMPLE: &str = "email,name,country\n\
        alice@example.com,Alice,US\n\
        bob@example.com,Bob,GB\n\
        carol@example.com,Carol,DE\n";
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/csv; charset=utf-8")
        .header(
            "Content-Disposition",
            "attachment; filename=\"mailer-sample.csv\"",
        )
        .body(SAMPLE.to_string())
}
