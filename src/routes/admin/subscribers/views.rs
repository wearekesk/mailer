//! Subscriber list view, import, edit, delete, unsubscribe.

use leptos::prelude::*;
use serde::Deserialize;

use crate::services::subscribers::{SubscriberRow, SubscriptionStatus};
use crate::utils::opaque_id::{OpaqueId, Subscriber};

pub(crate) const PAGE_SIZE: u64 = 50;

// ── Pages ────────────────────────────────────────────────────────────────────

#[component]
pub fn SubscribersIndexPage(
    list_token: String,
    list_name: String,
    brand_token: String,
    brand_name: String,
    rows: Vec<SubscriberRow>,
    page: u64,
    total: u64,
    /// Active status-tab id ("all" / "active" / "unconfirmed" / etc).
    #[prop(into, default = String::from("all"))]
    active_status: String,
    /// Current `q` search string.
    #[prop(into, default = String::new())]
    search: String,
) -> impl IntoView {
    let import_href = format!("/app/lists/{}/subscribers/import", list_token);
    let last_page = if total == 0 { 0 } else { (total.saturating_sub(1)) / PAGE_SIZE };

    // Preserve `status` and `q` in pagination links.
    let qs_extra = {
        let mut parts = Vec::new();
        if active_status != "all" {
            parts.push(format!("status={}", active_status));
        }
        if !search.is_empty() {
            parts.push(format!("q={}", urlencode(&search)));
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("&{}", parts.join("&"))
        }
    };
    let prev_href = if page > 0 {
        Some(format!("/app/subscribers/{}?page={}{}", list_token, page - 1, qs_extra))
    } else {
        None
    };
    let next_href = if page < last_page {
        Some(format!("/app/subscribers/{}?page={}{}", list_token, page + 1, qs_extra))
    } else {
        None
    };
    let total_label = format!("{} subscribers", total);

    let bulk_action = format!("/app/lists/{}/subscribers/bulk", list_token);
    let tabs: &[(&str, &str)] = &[
        ("all", "All"),
        ("active", "Active"),
        ("unconfirmed", "Unconfirmed"),
        ("unsubscribed", "Unsubscribed"),
        ("bounced", "Bounced"),
        ("complained", "Complained"),
    ];

    view! {
        <section class="space-y-4">
            <nav class="text-sm text-gray-500 dark:text-gray-400">
                <a href="/app/brands" class="hover:underline">"Brands"</a>" / "
                <a href=format!("/app/brands/{}/lists", brand_token) class="hover:underline">{brand_name}</a>" / "
                <span class="text-gray-700 dark:text-gray-300">{list_name.clone()}</span>
            </nav>
            <header class="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-3">
                <div>
                    <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">{list_name}</h1>
                    <p class="text-sm text-gray-500 dark:text-gray-400">{total_label}</p>
                </div>
                <div class="flex gap-2 flex-wrap">
                    <a href=format!("/app/lists/{}/embed", list_token)
                        class="inline-flex items-center text-gray-700 dark:text-gray-200 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-600 font-medium rounded-lg text-sm px-4 py-2"
                    ><i class="fa-regular fa-rectangle-list me-1"></i>"Subscribe form"</a>
                    <a href=format!("/app/lists/{}/subscribers/export.csv", list_token)
                        class="text-blue-700 bg-blue-50 hover:bg-blue-100 font-medium rounded-lg text-sm px-4 py-2"
                    >"Export CSV"</a>
                    <a href=import_href
                        class="inline-flex items-center text-white bg-primary-600 hover:bg-primary-700 font-medium rounded-lg text-sm px-4 py-2"
                    >"Import CSV"</a>
                </div>
            </header>
            <crate::components::layout::ListSubNav list_token=list_token.clone()/>

            // ── Search & status filter dropdown ─────────────────────
            <form method="GET" action=format!("/app/subscribers/{}", list_token) class="flex flex-wrap gap-3 items-center bg-white dark:bg-gray-800 p-3 rounded-lg border border-gray-200 dark:border-gray-700 shadow-xs">
                <div class="grow max-w-sm">
                    <label for="q" class="sr-only">"Search subscribers"</label>
                    <div class="relative">
                        <div class="absolute inset-y-0 start-0 flex items-center ps-3 pointer-events-none text-gray-400">
                            <i class="fa-solid fa-magnifying-glass text-xs"></i>
                        </div>
                        <input type="search" id="q" name="q" value=search.clone()
                            placeholder="Search email or name…"
                            class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full ps-9 p-2"/>
                    </div>
                </div>
                <div class="w-48">
                    <label for="status" class="sr-only">"Status filter"</label>
                    <select id="status" name="status" onchange="this.form.submit()"
                        class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2">
                        {tabs.iter().map(|(id, label)| {
                            let is_selected = *id == active_status;
                            view! {
                                <option value=*id selected=is_selected>{*label}</option>
                            }
                        }).collect_view()}
                    </select>
                </div>
                <button type="submit"
                    class="text-white bg-primary-600 hover:bg-primary-700 font-medium rounded-lg text-sm px-4 py-2">
                    "Filter"
                </button>
                {if !search.is_empty() || active_status != "all" {
                    let clear = format!("/app/subscribers/{}", list_token);
                    Some(view! {
                        <a href=clear class="text-xs text-gray-600 dark:text-gray-400 hover:underline self-center">"Reset filters"</a>
                    })
                } else { None }}
            </form>

            <form method="POST" action=bulk_action>
                <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm overflow-x-auto">
                    <table class="w-full text-sm text-left text-gray-500 dark:text-gray-400">
                        <thead class="text-xs text-gray-700 dark:text-gray-300 uppercase bg-gray-50 dark:bg-gray-700">
                            <tr>
                                <th class="px-4 py-3 w-1"><input type="checkbox" id="bulk-select-all" class="h-4 w-4"/></th>
                                <th class="px-4 py-3">"Email"</th>
                                <th class="px-4 py-3">"Name"</th>
                                <th class="px-4 py-3">"Status"</th>
                                <th class="px-4 py-3">"Added"</th>
                                <th class="px-4 py-3 w-1"><span class="sr-only">"Actions"</span></th>
                            </tr>
                        </thead>
                        <tbody>
                            {if rows.is_empty() {
                                view! {
                                    <tr><td colspan="6" class="text-center text-gray-400 dark:text-gray-500 py-10">"No subscribers."</td></tr>
                                }.into_any()
                            } else {
                                let token_for_rows = list_token.clone();
                                rows.into_iter().map(|r| {
                                    let stoken = OpaqueId::<Subscriber>::new(r.id).to_string();
                                    let unsubscribe_action = format!("/app/subscribers/{}/unsubscribe", stoken);
                                    let delete_action = format!("/app/subscribers/{}/delete?list={}", stoken, token_for_rows);
                                    let added_label = chrono::DateTime::<chrono::Utc>::from_timestamp(r.timestamp, 0)
                                        .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                                        .unwrap_or_default();
                                    let row_id = r.id;
                                    view! {
                                        <tr class="bg-white dark:bg-gray-800 border-b hover:bg-gray-50 dark:hover:bg-gray-600">
                                            <td class="px-4 py-3">
                                                <input type="checkbox" name="ids" value=row_id.to_string() class="bulk-row h-4 w-4"/>
                                            </td>
                                            <td class="px-4 py-3 font-medium text-gray-900 dark:text-white">{r.email}</td>
                                            <td class="px-4 py-3">{r.name.unwrap_or_default()}</td>
                                            <td class="px-4 py-3">{StatusBadge(StatusBadgeProps{status:r.status})}</td>
                                            <td class="px-4 py-3 text-xs text-gray-500 dark:text-gray-400">{added_label}</td>
                                            <td class="px-4 py-3 text-right">
                                                <div class="flex gap-3 justify-end">
                                                    <a href=format!("/app/subscribers/{}/edit", stoken) class="text-blue-600 hover:underline text-sm">"Edit"</a>
                                                    <form method="POST" action=unsubscribe_action>
                                                        <button type="submit" class="text-amber-600 hover:underline text-sm">"Unsubscribe"</button>
                                                    </form>
                                                    <form method="POST" action=delete_action onsubmit="return confirm('Permanently delete this subscriber?');">
                                                        <button type="submit" class="text-red-600 hover:underline text-sm">"Delete"</button>
                                                    </form>
                                                </div>
                                            </td>
                                        </tr>
                                    }
                                }).collect_view().into_any()
                            }}
                        </tbody>
                    </table>
                </div>
                <div class="flex flex-wrap gap-2 justify-end pt-3">
                    <button type="submit" name="op" value="unsubscribe"
                        class="text-sm font-medium text-amber-700 dark:text-amber-300 bg-amber-50 dark:bg-amber-900/30 hover:bg-amber-100 dark:hover:bg-amber-900/50 rounded-lg px-3 py-2">
                        "Unsubscribe selected"
                    </button>
                    <button type="submit" name="op" value="delete"
                        onclick="return confirm('Permanently delete every selected subscriber?');"
                        class="text-sm font-medium text-white bg-red-600 hover:bg-red-700 rounded-lg px-3 py-2">
                        "Delete selected"
                    </button>
                </div>
            </form>
            <script>{r#"document.getElementById('bulk-select-all')?.addEventListener('change',function(e){document.querySelectorAll('input.bulk-row').forEach(function(cb){cb.checked=e.target.checked;});});"#}</script>

            <nav class="flex justify-between items-center text-sm">
                <span class="text-gray-500 dark:text-gray-400">{format!("Page {} of {}", page + 1, last_page + 1)}</span>
                <div class="flex gap-2">
                    {prev_href.map(|h| view! { <a href=h class="px-3 py-1 border rounded hover:bg-gray-50 dark:hover:bg-gray-600">"Previous"</a> })}
                    {next_href.map(|h| view! { <a href=h class="px-3 py-1 border rounded hover:bg-gray-50 dark:hover:bg-gray-600">"Next"</a> })}
                </div>
            </nav>
        </section>
    }
}

/// Minimal URL-encode for the search box round-trip — `+` and `%` and
/// the few characters that would break a query-string. We don't need
/// full RFC 3986 here.
fn urlencode(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            ' ' => "+".chars().collect::<Vec<_>>(),
            '&' => "%26".chars().collect::<Vec<_>>(),
            '#' => "%23".chars().collect::<Vec<_>>(),
            '+' => "%2B".chars().collect::<Vec<_>>(),
            '%' => "%25".chars().collect::<Vec<_>>(),
            '=' => "%3D".chars().collect::<Vec<_>>(),
            '?' => "%3F".chars().collect::<Vec<_>>(),
            c => vec![c],
        })
        .collect()
}

#[component]
fn StatusBadge(status: SubscriptionStatus) -> impl IntoView {
    let (cls, label) = match status {
        SubscriptionStatus::Subscribed => ("bg-green-100 text-green-800", "Subscribed"),
        SubscriptionStatus::Unconfirmed => ("bg-amber-100 text-amber-800", "Unconfirmed"),
        SubscriptionStatus::Unsubscribed => ("bg-gray-200 text-gray-800", "Unsubscribed"),
        SubscriptionStatus::Bounced => ("bg-red-100 text-red-800", "Bounced"),
        SubscriptionStatus::Complained => ("bg-rose-100 text-rose-800", "Complained"),
        SubscriptionStatus::SoftBounced => ("bg-orange-100 text-orange-800", "Soft bounced"),
        SubscriptionStatus::NotInList => ("bg-gray-100 text-gray-700 dark:text-gray-300", "—"),
    };
    let class_str = format!("text-xs font-medium px-2.5 py-0.5 rounded {}", cls);
    view! { <span class=class_str>{label}</span> }
}

#[component]
pub fn ImportPage(
    list_token: String,
    list_name: String,
    #[prop(optional)] result: Option<(usize, usize)>,
) -> impl IntoView {
    let action = format!("/app/lists/{}/subscribers/import", list_token);
    let back = format!("/app/subscribers/{}", list_token);

    view! {
        <section class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6 max-w-2xl">
            <nav class="text-sm text-gray-500 dark:text-gray-400 mb-4">
                <a href=back.clone() class="hover:underline">"← Back to "{list_name}</a>
            </nav>
            <h1 class="text-2xl font-semibold text-gray-900 dark:text-white mb-2">"Import subscribers"</h1>
            <p class="text-sm text-gray-500 dark:text-gray-400 mb-2">
                "Upload a CSV with at least an "<code class="bg-gray-100 dark:bg-gray-700 px-1.5 rounded">"email"</code>" column. "
                "An optional "<code class="bg-gray-100 dark:bg-gray-700 px-1.5 rounded">"name"</code>" column will be used for personalization."
            </p>
            <p class="text-sm text-gray-500 dark:text-gray-400 mb-5">
                "Need an example? "
                <a href="/app/subscribers/sample.csv" download="mailer-sample.csv"
                    class="text-blue-600 hover:underline">
                    "Download sample CSV"
                </a>
            </p>
            {result.map(|(ins, skip)| view! {
                <div class="p-3 mb-4 text-sm text-green-800 rounded-lg bg-green-50 border border-green-200">
                    {format!("Imported {} subscribers; skipped {}.", ins, skip)}
                </div>
            })}
            <form method="POST" action=action enctype="multipart/form-data" class="space-y-4">
                <div>
                    <label for="csv" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"CSV file"</label>
                    <input type="file" id="csv" name="csv" accept=".csv,text/csv" required
                        class="block w-full text-sm text-gray-900 dark:text-white border border-gray-300 dark:border-gray-600 rounded-lg cursor-pointer bg-gray-50 dark:bg-gray-700 focus:outline-none"/>
                </div>
                <div>
                    <label for="mode" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"What should we do with each row?"</label>
                    <select id="mode" name="mode"
                        class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5">
                        <option value="add" selected>"Add (insert new subscribers, skip duplicates)"</option>
                        <option value="update">"Update (refresh name/fields on existing subscribers)"</option>
                        <option value="unsubscribe">"Unsubscribe matching subscribers"</option>
                        <option value="delete">"Delete matching subscribers (GDPR erasure)"</option>
                    </select>
                </div>
                <button type="submit"
                    class="text-white bg-blue-600 hover:bg-blue-700 font-medium rounded-lg text-sm px-5 py-2.5">
                    "Run import"
                </button>
            </form>
        </section>
    }
}

// ── Handlers ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PageQuery {
    #[serde(default)]
    pub page: Option<u64>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub q: Option<String>,
}

