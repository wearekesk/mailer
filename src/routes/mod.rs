pub mod admin;
pub mod api;
pub mod subscription;
pub mod tracking;
pub mod webhooks;

use poem::{
    endpoint::StaticFilesEndpoint,
    get,
    middleware::{CatchPanic, CookieJarManager},
    post, EndpointExt, Route,
};

use crate::{services, AppState};

/// Assemble the full application router and attach shared state.
pub fn build(state: AppState) -> impl poem::Endpoint {
    Route::new()
        // ── Tracking ────────────────────────────────────────────────────────
        .at(
            "/t/:link_token/:subscriber_token/:campaign_token",
            get(tracking::link_click),
        )
        .at(
            "/o/:campaign_token/:subscriber_token",
            get(tracking::open_pixel),
        )
        .at(
            "/w/:campaign_token/:subscriber_token",
            get(tracking::web_version),
        )
        // ── Subscription (public) ────────────────────────────────────────────
        .at(
            "/subscribe/:list_token",
            get(subscription::subscribe_form).post(subscription::subscribe),
        )
        .at("/unsubscribe/success", get(subscription::unsubscribe_success))
        .at(
            "/unsubscribe/:list_token/:subscriber_token",
            get(subscription::unsubscribe_form).post(subscription::unsubscribe),
        )
        .at("/confirm", get(subscription::confirm))
        .at("/reconsent/success", get(subscription::reconsent_success))
        .at("/delete/:list_token", post(subscription::delete_from_list))
        .at(
            "/subscription/:list_token/:subscriber_token",
            get(subscription::subscription_page).post(subscription::subscription_save),
        )
        // ── Webhooks ─────────────────────────────────────────────────────────
        .at("/webhooks/sns/bounce", post(webhooks::sns_bounce))
        .at("/webhooks/sns/complaint", post(webhooks::sns_complaint))
        .at("/webhooks/sns/confirm", post(webhooks::sns_confirm))
        .at("/webhooks/elasticemail", post(webhooks::elasticemail))
        .at("/webhooks/mailjet", post(webhooks::mailjet))
        .at("/webhooks/sendgrid", post(webhooks::sendgrid))
        // ── REST API ─────────────────────────────────────────────────────────
        .nest("/api", api::routes())
        // ── Model Context Protocol (MCP for AI Agents) ────────────────────────
        .nest("/v1/mcp", crate::mcp::mcp_endpoint(state.clone()))
        // ── Auth (unauthenticated pages) ─────────────────────────────────────
        .at(
            "/login",
            get(admin::auth::login_form).post(admin::auth::login),
        )
        .at("/logout", get(admin::auth::logout))
        .at(
            "/2fa",
            get(admin::auth::two_factor_form).post(admin::auth::two_factor_verify),
        )
        .at(
            "/forgot-password",
            get(admin::auth::forgot_form).post(admin::auth::forgot),
        )
        .at(
            "/reset-password",
            get(admin::auth::reset_form).post(admin::auth::reset_submit),
        )
        // ── Static assets (Tailwind output, images, etc.) ────────────────────
        .nest("/static", StaticFilesEndpoint::new("./static").show_files_listing())
        .nest("/uploads", StaticFilesEndpoint::new("./uploads"))
        // ── Root redirect ────────────────────────────────────────────────────
        .at("/", get(admin::dashboard::root))
        // ── Admin app ────────────────────────────────────────────────────────
        // poem's `nest("/app", …)` only matches `/app/…` — bare `/app`
        // 404s. Sidebar/nav links use `/app/` (trailing slash) to land on
        // the dashboard handler at `at("/", …)` inside admin::routes().
        .nest("/app", admin::routes())
        // Attach shared state so every handler can extract Data<&AppState>
        .data(state)
        // CookieJar extractor (used by the session/auth modules) panics at
        // runtime without this middleware — see poem's request.rs:432.
        .with(CookieJarManager::new())
        // Convert any handler panic into a 500 response instead of dropping
        // the connection, so a single bad page doesn't take down the server.
        .with(CatchPanic::new())
        // Fluent-based i18n. Handlers extract a `Locale` and call
        // `locale.text("key")`; resources live under `./i18n/<lang>/`.
        // Attached as `Data` because that's how poem's Locale extractor
        // looks them up (poem 3.1's i18n module ships no middleware).
        .data(services::i18n::resources())
}
