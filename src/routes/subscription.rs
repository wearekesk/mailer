//! Public subscription endpoints for list subscription, unsubscription, confirmation,
//! and subscriber preferences.
//!
//! All identifiers in URLs use [`OpaqueId`](crate::utils::opaque_id::OpaqueId)
//! tokens (see `utils/opaque_id.rs`). Path scheme:
//!
//! ```text
//! GET  /subscribe/:list_token
//! POST /subscribe/:list_token
//! GET  /unsubscribe/:list_token/:subscriber_token
//! POST /unsubscribe/:list_token/:subscriber_token
//! GET  /confirm?e=<sid>&l=<list>
//! GET  /unsubscribe/success
//! GET  /reconsent/success
//! POST /delete/:list_token
//! GET  /subscription/:list_token/:subscriber_token
//! POST /subscription/:list_token/:subscriber_token
//! ```

use std::collections::HashMap;

use poem::{
    handler,
    http::StatusCode,
    web::{Data, Form, Path, Query},
    Response,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;

use crate::database::entities::{lists as lists_entity, subscribers as subscribers_entity};
use crate::services::subscribers::{self, SubscribeOutcome, SubscribeParams};
use crate::utils::opaque_id::{List, OpaqueId, Subscriber};
use crate::AppState;

// ── Form / query types ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SubscribeForm {
    pub email: Option<String>,
    pub name: Option<String>,
    /// Honeypot — must be empty.
    pub hp: Option<String>,
    pub gdpr: Option<String>,
    pub silent: Option<String>,
    pub ipaddress: Option<String>,
    pub country: Option<String>,
    pub referrer: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UnsubscribeForm {
    /// RFC 8058 one-click List-Unsubscribe value.
    #[serde(rename = "List-Unsubscribe")]
    pub list_unsubscribe: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmQuery {
    pub e: String,
    pub l: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteForm {
    pub email: Option<String>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// `GET /subscribe/:list_token` — minimal subscribe form.
#[handler]
pub async fn subscribe_form(
    Path(list_token): Path<OpaqueId<List>>,
    Data(state): Data<&AppState>,
) -> Response {
    let list = match crate::services::lists::find_list(&state.db, list_token.raw()).await {
        Ok(Some(l)) => l,
        _ => return text_404("List not found"),
    };
    let title = list.name.unwrap_or_else(|| "Subscribe".into());

    let body = format!(
        r#"<h1 class="text-2xl font-semibold mb-4">{}</h1>
        <form method="POST" action="/subscribe/{list}" class="space-y-3 text-left">
            <input class="w-full border border-slate-300 dark:border-gray-600 rounded px-3 py-2 text-sm"
                   type="email" name="email" placeholder="you@example.com" required/>
            <input class="w-full border border-slate-300 dark:border-gray-600 rounded px-3 py-2 text-sm"
                   type="text" name="name" placeholder="Your name (optional)"/>
            <input type="text" name="hp" tabindex="-1" autocomplete="off"
                   class="hidden" aria-hidden="true"/>
            <button class="w-full bg-sky-600 hover:bg-sky-700 text-white rounded py-2 text-sm font-medium"
                    type="submit">Subscribe</button>
        </form>"#,
        html_escape(&title),
        list = list_token,
    );
    html(body)
}

/// `POST /subscribe/:list_token`
#[handler]
pub async fn subscribe(
    Path(list_token): Path<OpaqueId<List>>,
    Form(form): Form<SubscribeForm>,
    Data(state): Data<&AppState>,
) -> Response {
    if !form.hp.as_deref().unwrap_or("").is_empty() {
        // Honeypot — pretend success.
        return html("<h2>You're subscribed!</h2>".into());
    }

    let email = form.email.unwrap_or_default().trim().to_string();
    if email.is_empty() {
        return bad_request("Some fields are missing.");
    }
    if !subscribers::is_valid_email(&email) {
        return bad_request("Email address is invalid.");
    }

    let params = SubscribeParams {
        email,
        name: nonempty(form.name),
        list_id: list_token.raw(),
        ip: nonempty(form.ipaddress),
        country: nonempty(form.country),
        referrer: nonempty(form.referrer),
        notes: nonempty(form.notes),
        gdpr: yesish(form.gdpr.as_deref()),
        silent: yesish(form.silent.as_deref()),
    };

    match subscribers::subscribe(&state.db, params).await {
        Ok(SubscribeOutcome::Subscribed) => html("<h2>You're subscribed!</h2>".into()),
        Ok(SubscribeOutcome::ConfirmationSent) => {
            html("<h2>Thank you, a confirmation email has been sent to you.</h2>".into())
        }
        Ok(SubscribeOutcome::AlreadySubscribed) => html("<h2>You're already subscribed!</h2>".into()),
        Ok(SubscribeOutcome::Suppressed) => bad_request("Email is suppressed."),
        Ok(SubscribeOutcome::Bounced) => bad_request("Bounced email address."),
        Ok(SubscribeOutcome::InvalidList) => text_404("Invalid list ID."),
        Err(e) => {
            tracing::error!(error = ?e, "subscribe failed");
            internal_error("Subscription failed.")
        }
    }
}

/// `GET /unsubscribe/:list_token/:subscriber_token`
#[handler]
pub async fn unsubscribe_form(
    Path((list_token, subscriber_token)): Path<(OpaqueId<List>, OpaqueId<Subscriber>)>,
    Data(state): Data<&AppState>,
) -> Response {
    let list_id = list_token.raw();
    let needs_confirm = match crate::services::lists::find_list(&state.db, list_id).await {
        Ok(Some(l)) => l.unsubscribe_confirm.unwrap_or(0) == 1,
        _ => return text_404("List not found"),
    };

    if needs_confirm {
        html(format!(
            r#"<h1 class="text-xl font-semibold mb-4">Confirm unsubscribe?</h1>
            <form method="POST" action="/unsubscribe/{list_token}/{subscriber_token}">
                <button class="rounded bg-rose-600 hover:bg-rose-700 text-white px-5 py-2 text-sm font-medium"
                        type="submit">Yes, unsubscribe me</button>
            </form>"#
        ))
    } else {
        // No confirmation step — perform the unsubscribe immediately.
        do_unsubscribe(state, list_token, subscriber_token).await
    }
}

/// `POST /unsubscribe/:list_token/:subscriber_token`
#[handler]
pub async fn unsubscribe(
    Path((list_token, subscriber_token)): Path<(OpaqueId<List>, OpaqueId<Subscriber>)>,
    Form(_form): Form<UnsubscribeForm>,
    Data(state): Data<&AppState>,
) -> Response {
    do_unsubscribe(state, list_token, subscriber_token).await
}

async fn do_unsubscribe(
    state: &AppState,
    list_token: OpaqueId<List>,
    subscriber_token: OpaqueId<Subscriber>,
) -> Response {
    let list_id = list_token.raw();
    let subscriber_id = subscriber_token.raw();

    let list = match crate::services::lists::find_list(&state.db, list_id).await {
        Ok(Some(l)) => l,
        _ => return text_404("List not found"),
    };

    let row = match subscribers_entity::Entity::find_by_id(subscriber_id)
        .one(&state.db)
        .await
    {
        Ok(Some(s)) => s,
        _ => return bad_request("Subscriber does not exist."),
    };
    let Some(email) = row.email else {
        return bad_request("Subscriber has no email.");
    };

    let result = if list.unsubscribe_all_list.unwrap_or(0) == 1 {
        subscribers::unsubscribe_all_in_brand(&state.db, &email, list_id).await
    } else {
        subscribers::unsubscribe(&state.db, &email, list_id)
            .await
            .map(|_| 1u64)
    };

    match result {
        Ok(_) => html("<h2>You're unsubscribed.</h2>".into()),
        Err(crate::utils::errors::AppError::NotFound(_)) => bad_request("Email does not exist."),
        Err(e) => {
            tracing::error!(error = ?e, "unsubscribe failed");
            internal_error("Unsubscribe failed.")
        }
    }
}

/// `GET /confirm?e=<sid>&l=<list>` — double-opt-in confirmation link.
#[handler]
pub async fn confirm(
    Query(q): Query<ConfirmQuery>,
    Data(state): Data<&AppState>,
) -> Response {
    let subscriber_id: i32 = match q.e.parse::<OpaqueId<Subscriber>>() {
        Ok(id) => id.raw(),
        Err(_) => return bad_request("Invalid confirm token"),
    };
    let list_id: i32 = match q.l.parse::<OpaqueId<List>>() {
        Ok(id) => id.raw(),
        Err(_) => return bad_request("Invalid list token"),
    };

    match subscribers::confirm_subscription(&state.db, subscriber_id, list_id).await {
        Ok(_) => html("<h2>You're subscribed!</h2>".into()),
        Err(e) => {
            tracing::error!(error = ?e, "confirm failed");
            internal_error("Confirmation failed.")
        }
    }
}

/// `GET /unsubscribe/success`
#[handler]
pub async fn unsubscribe_success() -> Response {
    html("<h2>You have been unsubscribed.</h2>".into())
}

/// `GET /reconsent/success`
#[handler]
pub async fn reconsent_success() -> Response {
    html("<h2>Your consent has been recorded.</h2>".into())
}

/// `POST /delete/:list_token` — GDPR right-to-erasure (public endpoint).
#[handler]
pub async fn delete_from_list(
    Path(list_token): Path<OpaqueId<List>>,
    Form(form): Form<DeleteForm>,
    Data(state): Data<&AppState>,
) -> Response {
    let email = form.email.unwrap_or_default().trim().to_string();
    if email.is_empty() {
        return bad_request("Email is required");
    }

    match subscribers::delete(&state.db, &email, list_token.raw()).await {
        Ok(()) => html("<h2>Subscriber removed.</h2>".into()),
        Err(e) => {
            tracing::error!(error = ?e, "delete-from-list failed");
            internal_error("Delete failed.")
        }
    }
}

/// `GET /subscription/:list_token/:subscriber_token` — preferences page.
///
/// Shows the subscriber every list under the same brand, with checkboxes
/// reflecting their current subscription state. Posting to the same URL
/// applies the diff.
#[handler]
pub async fn subscription_page(
    Path((list_token, subscriber_token)): Path<(OpaqueId<List>, OpaqueId<Subscriber>)>,
    Query(q): Query<PreferencesQuery>,
    Data(state): Data<&AppState>,
) -> Response {
    let (subscriber, brand_lists, subscribed_ids) =
        match load_preferences(&state.db, list_token.raw(), subscriber_token.raw()).await {
            Ok(t) => t,
            Err(r) => return r,
        };

    let email = subscriber.email.clone().unwrap_or_default();
    let saved_banner = if q.saved.is_some() {
        r#"<div class="mb-4 rounded-md bg-emerald-50 text-emerald-800 px-3 py-2 text-sm">Preferences saved.</div>"#
    } else {
        ""
    };

    let mut rows = String::new();
    for l in &brand_lists {
        let name = l.name.clone().unwrap_or_else(|| format!("List {}", l.id));
        let checked = if subscribed_ids.contains(&l.id) { "checked" } else { "" };
        rows.push_str(&format!(
            r#"<label class="flex items-center gap-3 py-2 border-b border-slate-100 dark:border-gray-700 last:border-b-0">
                <input type="checkbox" name="list_{lid}" value="1" {checked}
                       class="h-4 w-4 text-blue-600 border-slate-300 dark:border-gray-600 rounded"/>
                <span class="text-slate-800 dark:text-gray-100">{name}</span>
            </label>"#,
            lid = l.id,
            checked = checked,
            name = html_escape(&name),
        ));
    }

    let body = format!(
        r#"<h1 class="text-2xl font-semibold mb-1 text-left">Manage your subscription</h1>
        <p class="text-slate-500 dark:text-gray-400 mb-5 text-left text-sm">{email}</p>
        {saved_banner}
        <form method="POST" action="/subscription/{list_token}/{subscriber_token}" class="text-left space-y-1">
            {rows}
            <div class="flex flex-wrap gap-3 pt-4">
                <button type="submit"
                        class="bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium rounded-md px-4 py-2">
                    Save preferences
                </button>
                <button type="submit" name="unsubscribe_all" value="1"
                        class="text-rose-700 hover:underline text-sm font-medium px-2">
                    Unsubscribe from all
                </button>
            </div>
        </form>"#,
        email = html_escape(&email),
        saved_banner = saved_banner,
    );
    html(body)
}

#[derive(Debug, Deserialize)]
pub struct PreferencesQuery {
    pub saved: Option<String>,
}

/// `POST /subscription/:list_token/:subscriber_token` — save preferences.
///
/// Expects a form with `list_<id>=1` for every checked list. Diffs against
/// the subscriber's current state and applies subscribe/unsubscribe per list.
/// `unsubscribe_all=1` short-circuits to a brand-wide unsubscribe.
#[handler]
pub async fn subscription_save(
    Path((list_token, subscriber_token)): Path<(OpaqueId<List>, OpaqueId<Subscriber>)>,
    Form(form): Form<HashMap<String, String>>,
    Data(state): Data<&AppState>,
) -> Response {
    let (subscriber, brand_lists, subscribed_ids) =
        match load_preferences(&state.db, list_token.raw(), subscriber_token.raw()).await {
            Ok(t) => t,
            Err(r) => return r,
        };
    let email = subscriber.email.clone().unwrap_or_default();
    if email.is_empty() {
        return bad_request("Subscriber has no email on file.");
    }

    let redirect_to = format!("/subscription/{list_token}/{subscriber_token}?saved=1");

    if form.get("unsubscribe_all").map(|v| v.as_str()) == Some("1") {
        let _ = subscribers::unsubscribe_all_in_brand(&state.db, &email, list_token.raw()).await;
        return redirect(&redirect_to);
    }

    let wanted: std::collections::HashSet<i32> = brand_lists
        .iter()
        .filter(|l| form.get(&format!("list_{}", l.id)).map(|v| v.as_str()) == Some("1"))
        .map(|l| l.id)
        .collect();

    for l in &brand_lists {
        let was_in = subscribed_ids.contains(&l.id);
        let want_in = wanted.contains(&l.id);
        if was_in == want_in {
            continue;
        }
        if want_in {
            let _ = subscribers::subscribe(
                &state.db,
                SubscribeParams {
                    email: email.clone(),
                    name: subscriber.name.clone(),
                    list_id: l.id,
                    ip: None,
                    country: subscriber.country.clone(),
                    referrer: None,
                    notes: None,
                    gdpr: subscriber.gdpr.unwrap_or(0) == 1,
                    silent: true,
                },
            )
            .await;
        } else {
            let _ = subscribers::unsubscribe(&state.db, &email, l.id).await;
        }
    }

    redirect(&redirect_to)
}

/// Load the subscriber row, every list in the same brand, and the set of
/// list-ids the subscriber is currently active on. Returns a redirect/error
/// `Response` on the left side if the subscriber/list cannot be resolved.
async fn load_preferences(
    db: &sea_orm::DatabaseConnection,
    list_id: i32,
    subscriber_id: i32,
) -> Result<
    (
        subscribers_entity::Model,
        Vec<lists_entity::Model>,
        std::collections::HashSet<i32>,
    ),
    Response,
> {
    let subscriber = subscribers_entity::Entity::find_by_id(subscriber_id)
        .one(db)
        .await
        .ok()
        .flatten()
        .ok_or_else(|| text_404("Subscriber not found"))?;
    let list = lists_entity::Entity::find_by_id(list_id)
        .one(db)
        .await
        .ok()
        .flatten()
        .ok_or_else(|| text_404("List not found"))?;
    let app_id = list.app.unwrap_or_default();

    let brand_lists = lists_entity::Entity::find()
        .filter(lists_entity::Column::App.eq(app_id))
        .filter(lists_entity::Column::Hide.ne(1))
        .all(db)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "load brand lists failed");
            internal_error("Failed to load preferences.")
        })?;

    let email = subscriber.email.clone().unwrap_or_default();
    let list_ids: Vec<i32> = brand_lists.iter().map(|l| l.id).collect();
    let subscribed_rows = subscribers_entity::Entity::find()
        .filter(subscribers_entity::Column::Email.eq(&email))
        .filter(subscribers_entity::Column::List.is_in(list_ids))
        .filter(subscribers_entity::Column::Unsubscribed.ne(1))
        .filter(subscribers_entity::Column::Bounced.ne(1))
        .filter(subscribers_entity::Column::Complaint.ne(1))
        .all(db)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "load subscribed lists failed");
            internal_error("Failed to load preferences.")
        })?;
    let subscribed_ids: std::collections::HashSet<i32> = subscribed_rows
        .into_iter()
        .filter_map(|s| s.list)
        .collect();

    Ok((subscriber, brand_lists, subscribed_ids))
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn nonempty(s: Option<String>) -> Option<String> {
    s.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

fn yesish(s: Option<&str>) -> bool {
    matches!(
        s.map(|s| s.trim().to_ascii_lowercase()).as_deref(),
        Some("true") | Some("1") | Some("yes")
    )
}

fn html(body: String) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(format!(
            "<!DOCTYPE html><html><head><meta charset=\"utf-8\">\
             <title>Mailer</title>\
             <link rel=\"stylesheet\" href=\"/static/dist/mailer.css\"></head>\
             <body class=\"min-h-screen flex items-center justify-center bg-slate-50 dark:bg-gray-900 font-sans\">\
             <main class=\"max-w-md w-full bg-white dark:bg-gray-800 rounded-xl shadow-lg p-8 text-center\">{body}</main>\
             </body></html>"
        ))
}

fn bad_request(msg: &str) -> Response {
    let mut r = html(format!("<h2 class=\"text-rose-600\">{msg}</h2>"));
    r.set_status(StatusCode::BAD_REQUEST);
    r
}

fn text_404(msg: &str) -> Response {
    let mut r = html(format!("<h2>{msg}</h2>"));
    r.set_status(StatusCode::NOT_FOUND);
    r
}

fn internal_error(msg: &str) -> Response {
    let mut r = html(format!("<h2 class=\"text-rose-600\">{msg}</h2>"));
    r.set_status(StatusCode::INTERNAL_SERVER_ERROR);
    r
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn redirect(location: &str) -> Response {
    Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", location)
        .body(())
}
