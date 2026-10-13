use super::views::*;
use leptos::prelude::*;
use poem::{
    handler,
    http::StatusCode,
    web::{Data, Form, Path},
    Response,
};
use serde::Deserialize;
use crate::routes::admin::session::CurrentUser;
use crate::services::brands;
use crate::services::lists::{
    self, CustomFieldType, ListInput,
};
use crate::utils::opaque_id::{Brand, List, OpaqueId};
use crate::AppState;

/// `GET /app/brands/:brand/lists`
#[handler]
pub async fn brand_lists_page(
    _user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let brand = match brands::find_brand(&state.db, brand_token.raw()).await {
        Ok(Some(b)) => b,
        _ => return redirect("/app/brands"),
    };
    let brand_name = brand.app_name.unwrap_or_default();
    let lists = lists::list_for_brand(&state.db, brand_token.raw(), true)
        .await
        .unwrap_or_default();
    let token = brand_token.to_string();
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Lists", move || {
        view! { <ListsIndexPage brand_token=token brand_name=brand_name lists=lists/> }
    })
    .await
}

/// `GET /app/brands/:brand/lists/new`
#[handler]
pub async fn new_form_page(
    _user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let token = brand_token.to_string();
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "New list", move || {
        view! { <ListFormPage brand_token=token/> }
    })
    .await
}

/// `GET /app/lists/:id/edit`
#[handler]
pub async fn edit_form_page(
    _user: CurrentUser,
    Path(list_token): Path<OpaqueId<List>>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let row = match lists::find_list(&state.db, list_token.raw()).await {
        Ok(Some(l)) => l,
        _ => return redirect("/app/brands"),
    };
    let brand_id = row.app.unwrap_or_default();
    let brand_token = OpaqueId::<Brand>::new(brand_id).to_string();
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Edit list", move || {
        view! { <ListFormPage brand_token=brand_token existing=row/> }
    })
    .await
}

/// `GET /app/lists/:id` — view (for now redirect to edit; full subscriber
/// view comes with the subscribers page).
#[handler]
pub async fn view(_user: CurrentUser, Path(list_token): Path<OpaqueId<List>>) -> Response {
    redirect(&format!("/app/subscribers/{}", list_token))
}

// ── POST handlers ───────────────────────────────────────────────────────────

#[handler]
pub async fn create(
    user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    Form(form): Form<ListForm>,
    Data(state): Data<&AppState>,
) -> Response {
    if form.name.trim().is_empty() {
        return redirect(&format!("/app/brands/{}/lists/new", brand_token));
    }
    let input: ListInput = form.into();
    match lists::create_list(&state.db, user.0.id, brand_token.raw(), input).await {
        Ok(_) => redirect(&format!("/app/brands/{}/lists", brand_token)),
        Err(e) => {
            tracing::error!(error = ?e, "create_list failed");
            redirect(&format!("/app/brands/{}/lists/new", brand_token))
        }
    }
}

#[handler]
pub async fn update(
    _user: CurrentUser,
    Path(list_token): Path<OpaqueId<List>>,
    Form(form): Form<ListForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let input: ListInput = form.into();
    match lists::update_list(&state.db, list_token.raw(), input).await {
        Ok(()) => redirect(&format!("/app/lists/{}/edit", list_token)),
        Err(e) => {
            tracing::error!(error = ?e, "update_list failed");
            redirect(&format!("/app/lists/{}/edit", list_token))
        }
    }
}

#[handler]
pub async fn delete(
    _user: CurrentUser,
    Path(list_token): Path<OpaqueId<List>>,
    Data(state): Data<&AppState>,
) -> Response {
    let row = lists::find_list(&state.db, list_token.raw()).await.ok().flatten();
    let _ = lists::delete_list(&state.db, list_token.raw()).await;
    let brand_id = row.and_then(|r| r.app).unwrap_or_default();
    let brand_token = OpaqueId::<Brand>::new(brand_id).to_string();
    redirect(&format!("/app/brands/{}/lists", brand_token))
}

#[handler]
pub async fn toggle_hide(
    _user: CurrentUser,
    Path(list_token): Path<OpaqueId<List>>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = lists::toggle_hidden(&state.db, list_token.raw()).await;
    redirect(&format!("/app/lists/{}/edit", list_token))
}

#[derive(Debug, Deserialize)]
pub struct BulkToggleForm {
    pub op: String,
    /// Repeated `list_ids` checkbox values from the list table.
    #[serde(default)]
    pub list_ids: Vec<i32>,
}

/// `POST /app/brands/:id/lists/bulk-toggle` — hide/show every selected
/// list in one shot.
#[handler]
pub async fn bulk_toggle(
    _user: CurrentUser,
    Path(brand_token): Path<OpaqueId<Brand>>,
    Form(form): Form<BulkToggleForm>,
    Data(state): Data<&AppState>,
) -> Response {
    if !form.list_ids.is_empty() {
        let hide = form.op.eq_ignore_ascii_case("hide");
        let _ = lists::bulk_set_hidden(&state.db, &form.list_ids, hide).await;
    }
    redirect(&format!("/app/brands/{brand_token}/lists"))
}

#[derive(Debug, Deserialize)]
pub struct CustomFieldForm {
    pub c_field: String,
    #[serde(default)]
    pub c_type: Option<String>,
}

#[handler]
pub async fn add_custom_field(
    _user: CurrentUser,
    Path(list_token): Path<OpaqueId<List>>,
    Form(form): Form<CustomFieldForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let kind = form
        .c_type
        .as_deref()
        .map(CustomFieldType::parse)
        .unwrap_or(CustomFieldType::Text);
    let _ = lists::add_custom_field(&state.db, list_token.raw(), &form.c_field, kind).await;
    redirect(&format!("/app/lists/{}/edit", list_token))
}

#[derive(Debug, Deserialize)]
pub struct DeleteCustomFieldForm {
    pub name: String,
}

#[handler]
pub async fn delete_custom_field(
    _user: CurrentUser,
    Path(list_token): Path<OpaqueId<List>>,
    Form(form): Form<DeleteCustomFieldForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let _ = lists::delete_custom_field(&state.db, list_token.raw(), &form.name).await;
    redirect(&format!("/app/lists/{}/edit", list_token))
}

// ── Embed-form generator ────────────────────────────────────────────────────

/// Embed-form page — three Flowbite tabs: Hosted URL, HTML form code, and
/// iframe embed. The HTML and iframe panels include a copy-to-clipboard
/// button so non-technical users don't have to hand-select the snippet.
#[component]
pub fn EmbedFormPage(
    list_token: String,
    settings: crate::services::embed::EmbedSettings,
) -> impl IntoView {
    let html_form = settings.html_form_code();
    let iframe_code = settings.iframe_code();
    let hosted_url = settings.hosted_url.clone();

    let recap_note = if settings.recaptcha_sitekey.is_some() {
        view! { <p class="text-xs text-emerald-700 dark:text-emerald-300"><i class="fa-solid fa-check me-1"></i>"reCAPTCHA is enabled — the form already includes the widget."</p> }.into_any()
    } else {
        view! { <p class="text-xs text-gray-500 dark:text-gray-400">"Tip: enable reCAPTCHA in the brand settings to add a spam-bot challenge to this form."</p> }.into_any()
    };

    let gdpr_note = if settings.gdpr_enabled {
        view! { <p class="text-xs text-gray-500 dark:text-gray-400">"GDPR consent fields are enabled on this list — they are included in the snippet above."</p> }.into_any()
    } else {
        view! { <p class="text-xs text-gray-500 dark:text-gray-400">"GDPR consent fields are disabled on this list. Toggle "<code class="text-xs bg-gray-100 dark:bg-gray-700 px-1 rounded">"Enable GDPR fields"</code>" in list settings if you need them."</p> }.into_any()
    };

    view! {
        <section class="space-y-4">
            <header>
                <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">{settings.list_name.clone()}</h1>
                <p class="text-sm text-gray-500 dark:text-gray-400">"Subscribe form code — paste this into your website to collect sign-ups for this list."</p>
            </header>
            <crate::components::layout::ListSubNav list_token=list_token.clone()/>

            // Flowbite "tabs with underline" pattern.
            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm">
                <div class="border-b border-gray-200 dark:border-gray-700">
                    <ul class="flex flex-wrap -mb-px text-sm font-medium text-center" id="embed-tabs" role="tablist" data-tabs-toggle="#embed-tab-content" data-tabs-active-classes="text-primary-600 dark:text-primary-300 border-primary-600 dark:border-primary-300" data-tabs-inactive-classes="text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300 border-transparent hover:border-gray-300 dark:hover:border-gray-600">
                        <li class="me-2" role="presentation">
                            <button class="inline-block p-4 border-b-2 rounded-t-lg" id="hosted-tab" data-tabs-target="#hosted" type="button" role="tab" aria-controls="hosted" aria-selected="true">
                                <i class="fa-solid fa-link me-2"></i>"Hosted URL"
                            </button>
                        </li>
                        <li class="me-2" role="presentation">
                            <button class="inline-block p-4 border-b-2 rounded-t-lg" id="html-tab" data-tabs-target="#html" type="button" role="tab" aria-controls="html" aria-selected="false">
                                <i class="fa-solid fa-code me-2"></i>"HTML form code"
                            </button>
                        </li>
                        <li class="me-2" role="presentation">
                            <button class="inline-block p-4 border-b-2 rounded-t-lg" id="iframe-tab" data-tabs-target="#iframe" type="button" role="tab" aria-controls="iframe" aria-selected="false">
                                <i class="fa-regular fa-window-maximize me-2"></i>"Iframe"
                            </button>
                        </li>
                    </ul>
                </div>
                <div id="embed-tab-content" class="p-6">
                    // ── Hosted URL pane ───────────────────────────────────
                    <div id="hosted" role="tabpanel" aria-labelledby="hosted-tab" class="space-y-3">
                        <p class="text-sm text-gray-600 dark:text-gray-400">"Send people to this URL or share it on social — Mailer hosts the page."</p>
                        <div class="flex flex-col sm:flex-row gap-2">
                            <input id="hosted-url" type="text" readonly value=hosted_url.clone()
                                onclick="this.select()"
                                class="grow font-mono text-xs bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white rounded-lg p-2.5"/>
                            <button type="button"
                                onclick=format!("navigator.clipboard.writeText({}); this.innerText='Copied!'; setTimeout(()=>this.innerText='Copy URL',1800);", js_string(&hosted_url))
                                class="text-white bg-primary-600 hover:bg-primary-700 font-medium rounded-lg text-sm px-4 py-2.5">
                                <i class="fa-regular fa-copy me-1"></i>"Copy URL"
                            </button>
                            <a href=hosted_url.clone() target="_blank" rel="noopener"
                                class="text-sm font-medium text-gray-700 dark:text-gray-200 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-600 rounded-lg px-4 py-2.5">
                                <i class="fa-regular fa-eye me-1"></i>"Preview"
                            </a>
                        </div>
                        {recap_note}
                    </div>

                    // ── HTML form code pane ───────────────────────────────
                    <div id="html" role="tabpanel" aria-labelledby="html-tab" class="hidden space-y-3">
                        <p class="text-sm text-gray-600 dark:text-gray-400">"Drop this snippet directly into your website's HTML — restyle freely with your own CSS."</p>
                        <textarea id="html-code" rows="14" readonly
                            onclick="this.select()"
                            class="w-full font-mono text-xs bg-gray-50 dark:bg-gray-900 border border-gray-300 dark:border-gray-700 text-gray-900 dark:text-gray-100 rounded-lg p-3">{html_form}</textarea>
                        <div class="flex flex-wrap gap-2">
                            <button type="button"
                                onclick="(function(b){var t=document.getElementById('html-code'); navigator.clipboard.writeText(t.value); b.innerText='Copied!'; setTimeout(()=>b.innerText='Copy HTML',1800);})(this)"
                                class="text-white bg-primary-600 hover:bg-primary-700 font-medium rounded-lg text-sm px-4 py-2.5">
                                <i class="fa-regular fa-copy me-1"></i>"Copy HTML"
                            </button>
                        </div>
                        {gdpr_note}
                    </div>

                    // ── Iframe pane ───────────────────────────────────────
                    <div id="iframe" role="tabpanel" aria-labelledby="iframe-tab" class="hidden space-y-3">
                        <p class="text-sm text-gray-600 dark:text-gray-400">"Embed the hosted form via iframe — keeps Mailer's styling and reCAPTCHA without any code changes on your side."</p>
                        <textarea id="iframe-code" rows="3" readonly
                            onclick="this.select()"
                            class="w-full font-mono text-xs bg-gray-50 dark:bg-gray-900 border border-gray-300 dark:border-gray-700 text-gray-900 dark:text-gray-100 rounded-lg p-3">{iframe_code}</textarea>
                        <button type="button"
                            onclick="(function(b){var t=document.getElementById('iframe-code'); navigator.clipboard.writeText(t.value); b.innerText='Copied!'; setTimeout(()=>b.innerText='Copy iframe',1800);})(this)"
                            class="text-white bg-primary-600 hover:bg-primary-700 font-medium rounded-lg text-sm px-4 py-2.5">
                            <i class="fa-regular fa-copy me-1"></i>"Copy iframe"
                        </button>
                    </div>
                </div>
            </div>
        </section>
    }
}

/// `GET /app/lists/:id/embed`
#[handler]
pub async fn embed_page(
    _user: CurrentUser,
    Path(list_token): Path<OpaqueId<List>>,
    Data(state): Data<&AppState>,
    jar: &poem::web::cookie::CookieJar,
    req: &poem::Request,
) -> Response {
    let list = match crate::services::lists::find_list(&state.db, list_token.raw())
        .await
        .ok()
        .flatten()
    {
        Some(l) => l,
        None => return redirect("/app/brands"),
    };
    let brand_id = list.app.unwrap_or_default();
    let brand = match brands::find_brand(&state.db, brand_id).await.ok().flatten() {
        Some(b) => b,
        None => return redirect("/app/brands"),
    };
    let settings =
        crate::services::embed::EmbedSettings::from_models(&brand, &list, &state.config.app_url);
    let token_str = list_token.to_string();
    crate::routes::admin::render_admin_page_chrome(state, jar, req, "Embed form", move || {
        view! { <EmbedFormPage list_token=token_str settings=settings/> }
    })
    .await
}

/// JS-string-literal escape for inlining a value into an `onclick` handler.
fn js_string(s: &str) -> String {
    let escaped = s
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "");
    format!("'{}'", escaped)
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn redirect(location: &str) -> Response {
    Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", location)
        .body(())
}

