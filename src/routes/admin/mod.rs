pub mod app;
pub mod auth;
pub mod autoresponders;
pub mod brands;
pub mod campaigns;
pub mod dashboard;
pub mod exports;
pub mod housekeeping;
pub mod lists;
pub mod rules;
pub mod segments;
pub mod session;
pub mod settings;
pub mod subscribers;
pub mod templates;

use leptos::prelude::*;
use sea_orm::EntityTrait;
use poem::{
    get, handler,
    http::StatusCode,
    post,
    web::{
        cookie::{Cookie, CookieJar, SameSite},
        Data,
    },
    IntoResponse, Request, Response, Route,
};

use crate::components::layout::{AdminChrome, AdminDocument, AdminLayout, ChromeBrand};
use crate::utils::opaque_id::{Brand, OpaqueId};
use crate::AppState;

/// Cookie that records the brand id the topbar selector is currently
/// pointed at. Plain integer body — read on every admin page render to
/// build the per-brand sidebar block.
const CURRENT_BRAND_COOKIE: &str = "mailer_brand";

/// Build the topbar/sidebar chrome (brands list + currently-selected
/// brand + current request path) from the database and the request
/// cookie. Cheap enough to run on every admin page load — one or two
/// queries.
pub async fn build_chrome(state: &AppState, jar: &CookieJar, path: &str) -> AdminChrome {
    let brands = match crate::services::brands::list_brands(&state.db).await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(error = ?e, "build_chrome: list_brands failed");
            return AdminChrome {
                current_path: path.to_string(),
                ..Default::default()
            };
        }
    };
    let chrome_brands: Vec<ChromeBrand> = brands
        .iter()
        .map(|b| ChromeBrand {
            id: b.id,
            token: OpaqueId::<Brand>::new(b.id).to_string(),
            name: if b.name.is_empty() {
                format!("Brand {}", b.id)
            } else {
                b.name.clone()
            },
            restriction: None,
        })
        .collect();

    let selected = jar
        .get(CURRENT_BRAND_COOKIE)
        .and_then(|c| c.value_str().parse::<i32>().ok());
    let mut current = selected
        .and_then(|id| chrome_brands.iter().find(|b| b.id == id).cloned())
        .or_else(|| chrome_brands.first().cloned());

    // Pull restriction flags only for the currently-selected brand.
    if let Some(ref mut cur) = current {
        if let Ok(Some(app_row)) =
            crate::database::entities::apps::Entity::find_by_id(cur.id)
                .one(&state.db)
                .await
        {
            cur.restriction = if app_row.campaigns_only.unwrap_or(0) == 1 {
                Some(crate::components::layout::BrandRestriction::CampaignsOnly)
            } else if app_row.templates_only.unwrap_or(0) == 1 {
                Some(crate::components::layout::BrandRestriction::TemplatesOnly)
            } else if app_row.lists_only.unwrap_or(0) == 1 {
                Some(crate::components::layout::BrandRestriction::ListsOnly)
            } else {
                None
            };
        }
    }

    AdminChrome {
        brands: chrome_brands,
        current,
        current_path: path.to_string(),
    }
}

/// Set the current-brand cookie from a handler (path-scoped to `/app`,
/// HttpOnly, SameSite=Lax — same posture as the session cookie).
pub fn set_current_brand_cookie(jar: &CookieJar, brand_id: i32) {
    let mut c = Cookie::new_with_str(CURRENT_BRAND_COOKIE, brand_id.to_string());
    c.set_path("/");
    c.set_http_only(true);
    c.set_same_site(SameSite::Lax);
    jar.add(c);
}

/// Render a Leptos component inside the admin shell (HTML document +
/// sidebar). Sync convenience for handlers that don't need the brand
/// switcher populated. Use [`render_admin_page_chrome`] for the
/// chrome-aware async variant.
pub fn render_admin_page<F, V>(title: &str, page: F) -> Response
where
    F: FnOnce() -> V + Send + 'static,
    V: IntoView + Send + 'static,
{
    render_admin_page_with_chrome(title, AdminChrome::default(), page)
}

/// Async variant that fetches the topbar/sidebar chrome (brands list +
/// currently-selected brand + current path for active-link highlighting)
/// before rendering. Prefer this whenever the handler already has
/// `&AppState`, `&CookieJar`, and `&Request` — it makes the brand
/// switcher, per-brand sidebar block, and active link all populate
/// correctly.
pub async fn render_admin_page_chrome<F, V>(
    state: &AppState,
    jar: &CookieJar,
    req: &Request,
    title: &str,
    page: F,
) -> Response
where
    F: FnOnce() -> V + Send + 'static,
    V: IntoView + Send + 'static,
{
    // poem's `nest("/app", …)` rewrites `req.uri()` to strip the prefix.
    // Use `original_uri()` so the chrome sees the URL the browser sent.
    let path = req.original_uri().path().to_string();
    let chrome = build_chrome(state, jar, &path).await;
    render_admin_page_with_chrome(title, chrome, page)
}

/// Sync variant for handlers that have already built their own chrome.
pub fn render_admin_page_with_chrome<F, V>(title: &str, chrome: AdminChrome, page: F) -> Response
where
    F: FnOnce() -> V + Send + 'static,
    V: IntoView + Send + 'static,
{
    let title = title.to_string();
    let owner = Owner::new();
    let html = owner.with(|| {
        provide_context(chrome);
        let title = title.clone();
        view! {
            <AdminDocument title=title>
                <AdminLayout>
                    {page()}
                </AdminLayout>
            </AdminDocument>
        }
        .to_html()
    });
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(html)
}

/// Mount admin endpoints under `/app`.
///
/// Layout:
/// - `GET /app/*path` is a single catch-all that renders the unified
///   [`app::AdminApp`] Leptos component. The component's `<Routes>` picks the
///   matching page for the URL.
/// - `POST` form actions live alongside their pages in the per-resource files.
pub fn routes() -> Route {
    Route::new()
        // ── Root redirect — `/app/` and `/app/dashboard` both bounce to
        // the canonical brands list, which is the new landing page.
        .at("/", get(dashboard::root_redirect))
        .at("/dashboard", get(dashboard::root_redirect))
        // ── Brand CRUD ───────────────────────────────────────────────────────
        .at("/brands", get(brands::list_page))
        .at("/brands/new", get(brands::new_form_page).post(brands::create))
        .at("/brands/:id/edit", get(brands::edit_form_page).post(brands::update))
        .at("/brands/:id/select", get(brands::select_brand))
        .at("/brands/:id/delete", post(brands::delete))
        .at("/brands/:id/duplicate", post(brands::duplicate))
        .at("/brands/:id/logo", post(brands::upload_logo))
        .at("/brands/:id/logo/delete", post(brands::delete_logo))
        .at("/brands/:id/users/new", post(brands::create_sub_user))
        .at("/users/:uid/delete", post(brands::delete_sub_user))
        .at("/brands/:id/ses/verify", post(brands::ses_verify))
        .at("/brands/:id/lists", get(lists::brand_lists_page))
        .at(
            "/brands/:id/lists/new",
            get(lists::new_form_page).post(lists::create),
        )
        // ── List CRUD ────────────────────────────────────────────────────────
        .at("/lists/:id", get(lists::view))
        .at("/lists/:id/edit", get(lists::edit_form_page).post(lists::update))
        .at("/lists/:id/delete", post(lists::delete))
        .at("/lists/:id/toggle-hide", post(lists::toggle_hide))
        .at("/lists/:id/embed", get(lists::embed_page))
        .at("/brands/:id/lists/bulk-toggle", post(lists::bulk_toggle))
        .at(
            "/lists/:id/custom-fields/add",
            post(lists::add_custom_field),
        )
        .at(
            "/lists/:id/custom-fields/delete",
            post(lists::delete_custom_field),
        )
        // ── Housekeeping ────────────────────────────────────────────────────
        .at("/lists/:id/housekeeping", get(housekeeping::page))
        .at(
            "/lists/:id/housekeeping/inactive",
            post(housekeeping::purge_inactive),
        )
        .at(
            "/lists/:id/housekeeping/unconfirmed",
            post(housekeeping::purge_unconfirmed),
        )
        .at(
            "/lists/:id/housekeeping/no-opens",
            post(housekeeping::purge_no_opens),
        )
        .at(
            "/lists/:id/housekeeping/no-clicks",
            post(housekeeping::purge_no_clicks),
        )
        .at(
            "/lists/:id/housekeeping/dedupe",
            post(housekeeping::dedupe),
        )
        .at(
            "/brands/:id/housekeeping/conflicts",
            get(housekeeping::conflicts),
        )
        // ── Subscribers ──────────────────────────────────────────────────────
        .at("/subscribers/sample.csv", get(subscribers::sample_csv))
        .at("/lists/:id/subscribers/bulk", post(subscribers::bulk))
        .at("/subscribers/:list_id", get(subscribers::index))
        .at(
            "/lists/:id/subscribers/import",
            get(subscribers::import_form).post(subscribers::import_csv),
        )
        .at(
            "/subscribers/:id/edit",
            get(subscribers::edit_form).post(subscribers::edit_save),
        )
        .at(
            "/subscribers/:id/unsubscribe",
            post(subscribers::unsubscribe_one),
        )
        .at(
            "/subscribers/:id/delete",
            post(subscribers::delete_one),
        )
        // ── Campaigns ────────────────────────────────────────────────────────
        .at("/brands/:id/campaigns", get(campaigns::list_for_brand_page))
        .at("/brands/:id/campaigns/new", get(campaigns::new_form).post(campaigns::create))
        .at("/campaigns/:id/edit", get(campaigns::edit_form).post(campaigns::update))
        .at("/campaigns/:id/import-url", post(campaigns::import_url))
        .at("/campaigns/:id/import-html", post(campaigns::import_html))
        .at(
            "/campaigns/:id/dismiss-broken-link-checks",
            post(campaigns::dismiss_link_check),
        )
        .at("/campaigns/:id/attachments/upload", post(campaigns::upload_attachment))
        .at("/campaigns/:id/attachments/delete", post(campaigns::delete_attachment))
        .at("/campaigns/:id/send-now", post(campaigns::send_now))
        .at("/campaigns/:id/send-to", post(campaigns::send_to))
        .at("/campaigns/:id/test-send", post(campaigns::test_send))
        .at("/campaigns/:id/schedule", post(campaigns::schedule))
        .at("/campaigns/:id/stop", post(campaigns::stop))
        .at("/campaigns/:id/delete", post(campaigns::delete))
        .at("/campaigns/:id/report", get(campaigns::report))
        .at("/campaigns/:id/report.csv", get(campaigns::report_csv))
        .at("/campaigns/:id/preview", get(campaigns::preview))
        // ── CSV exports ──────────────────────────────────────────────────────
        .at(
            "/lists/:id/subscribers/export.csv",
            get(exports::subscribers_csv),
        )
        .at(
            "/brands/:id/suppression-list/export.csv",
            get(exports::suppression_csv),
        )
        .at(
            "/brands/:id/blocked-domains/export.csv",
            get(exports::blocked_domains_csv),
        )
        .at(
            "/brands/:id/campaigns/export.csv",
            get(exports::campaigns_csv),
        )
        .at(
            "/segments/:id/export.csv",
            get(exports::segment_members_csv),
        )
        .at("/subscribers/:list_id/import", post(subscribers::import_csv))
        .at("/brands/:id/templates", get(templates::list_for_brand_page))
        .at("/templates/new", get(templates::new_form).post(templates::create))
        .at("/templates/:id/edit", get(templates::edit_form).post(templates::update))
        .at("/templates/:id/use", post(templates::use_template))
        .at("/templates/:id/delete", post(templates::delete))
        .at("/templates/:id/preview", get(templates::preview))
        // ── Segments ─────────────────────────────────────────────────────────
        .at("/brands/:id/segments", get(segments::list_for_brand_page))
        .at("/lists/:id/segments", get(segments::list_for_list_page))
        .at("/lists/:id/segments/new", post(segments::list_create))
        .at("/brands/:id/segments/new", post(segments::create))
        .at("/segments/:id/edit", get(segments::edit_form))
        .at("/segments/:id/conditions/add", post(segments::add_condition))
        .at(
            "/segments/:seg_id/conditions/:cond_id/delete",
            post(segments::delete_condition),
        )
        .at("/segments/:id/delete", post(segments::delete))
        // ── Rules ────────────────────────────────────────────────────────────
        .at("/brands/:id/rules", get(rules::list_for_brand_page))
        .at("/brands/:id/rules/save", post(rules::save))
        .at("/rules/:id/toggle", post(rules::toggle))
        .at("/rules/:id/delete", post(rules::delete))
        // ── Autoresponders ───────────────────────────────────────────────────
        .at("/brands/:id/autoresponders", get(autoresponders::list_for_brand_page))
        .at(
            "/lists/:id/autoresponders",
            get(autoresponders::list_for_list_page),
        )
        .at(
            "/lists/:id/autoresponders/new",
            post(autoresponders::list_create_ar),
        )
        .at("/brands/:id/autoresponders/new", post(autoresponders::create))
        .at("/autoresponders/:id/delete", post(autoresponders::delete))
        .at("/autoresponders/:id/emails", get(autoresponders::emails_page))
        .at(
            "/autoresponders/:id/emails/new",
            get(autoresponders::email_new_form).post(autoresponders::email_create),
        )
        .at(
            "/autoresponder-emails/:id/edit",
            get(autoresponders::email_edit_form).post(autoresponders::email_update),
        )
        .at(
            "/autoresponder-emails/:id/toggle",
            post(autoresponders::email_toggle),
        )
        .at(
            "/autoresponder-emails/:id/delete",
            post(autoresponders::email_delete),
        )
        .at(
            "/autoresponder-emails/:id/duplicate",
            post(autoresponders::email_duplicate),
        )
        .at(
            "/autoresponder-emails/:id/preview",
            get(autoresponders::email_preview),
        )
        .at(
            "/autoresponder-emails/:id/test-send",
            post(autoresponders::email_test_send),
        )
        .at(
            "/autoresponder-emails/:id/attachments/upload",
            post(autoresponders::email_upload_attachment),
        )
        .at(
            "/autoresponder-emails/:id/attachments/delete",
            post(autoresponders::email_delete_attachment),
        )
        .at(
            "/autoresponder-emails/:id/report",
            get(autoresponders::email_report),
        )
        .at(
            "/autoresponder-emails/:id/report.csv",
            get(autoresponders::email_report_csv),
        )
        .at("/settings", get(settings::index).post(settings::save))
        .at(
            "/settings/regenerate-api-key",
            post(settings::regenerate_api_key),
        )
        .at(
            "/settings/two-factor/setup",
            get(settings::two_factor_setup),
        )
        .at(
            "/settings/two-factor/confirm",
            post(settings::two_factor_confirm),
        )
        .at(
            "/settings/two-factor/disable",
            post(settings::two_factor_disable),
        )
        .at(
            "/settings/ses/verify-login-email",
            post(settings::ses_verify_login_email),
        )
        .at(
            "/settings/suppression",
            get(settings::settings_suppression_redirect),
        )
        .at(
            "/settings/suppression-list",
            get(settings::settings_suppression_redirect),
        )
        .at(
            "/settings/blocked-domains",
            get(settings::settings_blocked_domains_redirect),
        )
        .at(
            "/brands/:id/suppression-list",
            get(settings::suppression_page),
        )
        .at(
            "/brands/:id/suppression-list/add",
            post(settings::add_suppression),
        )
        .at(
            "/suppressions/:sid/delete",
            post(settings::delete_suppression),
        )
        .at(
            "/brands/:id/suppression-list/clear",
            post(settings::clear_suppression_list),
        )
        .at(
            "/brands/:id/suppression-list/import",
            post(settings::import_suppression_csv),
        )
        .at(
            "/brands/:id/blocked-domains",
            get(settings::blocked_domains_page),
        )
        .at(
            "/brands/:id/blocked-domains/add",
            post(settings::add_blocked_domain),
        )
        .at(
            "/blocked-domains/:bid/delete",
            post(settings::delete_blocked_domain),
        )
        .at(
            "/brands/:id/blocked-domains/clear",
            post(settings::clear_blocked_domains),
        )
        .at(
            "/brands/:id/blocked-domains/import",
            post(settings::import_blocked_domains_csv),
        )
        .at(
            "/settings/scheduler/reset-cron-lock",
            post(settings::reset_cron_lock),
        )
        // ── Catch-all SSR shell (placeholder pages still in AdminApp) ────────
        .at("/*path", get(render_app))
}

/// Catch-all 404 page for any GET under `/app/*` that doesn't match a real
/// resource handler. Requires a valid session cookie via the
/// [`session::CurrentUser`] extractor — anonymous requests are 302'd to
/// `/login`.
#[handler]
async fn render_app(
    _user: session::CurrentUser,
    Data(_state): Data<&AppState>,
) -> impl IntoResponse {
    let owner = Owner::new();
    let html = owner.with(|| app::AdminApp().to_html());

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(html)
}
