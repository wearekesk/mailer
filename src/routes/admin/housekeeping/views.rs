//! Housekeeping admin pages: per-list purges, dedupe, and brand-wide
//! conflict detection.

use leptos::prelude::*;
use serde::Deserialize;


// ── Page ────────────────────────────────────────────────────────────────────

#[component]
pub fn HousekeepingPage(
    list_token: String,
    list_name: String,
    brand_token: String,
    brand_name: String,
    #[prop(default = None)] result: Option<HousekeepingResult>,
) -> impl IntoView {
    view! {
        <section class="space-y-6">
            <nav class="text-sm text-gray-500 dark:text-gray-400">
                <a href="/app/brands" class="hover:underline">"Brands"</a>" / "
                <a href=format!("/app/brands/{}/lists", brand_token) class="hover:underline">{brand_name}</a>" / "
                <span class="text-gray-700 dark:text-gray-300">{list_name.clone()}</span>" / "
                <span class="text-gray-700 dark:text-gray-300">"Housekeeping"</span>
            </nav>
            <header>
                <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">"Housekeeping — "{list_name}</h1>
                <p class="text-sm text-gray-500 dark:text-gray-400">"Bulk-clean inactive, unconfirmed, or unengaged subscribers. These actions are immediate and cannot be undone — export a CSV first if you want a backup."</p>
            </header>

            {result.map(|r| view! {
                <div class="p-3 text-sm text-green-800 rounded-lg bg-green-50 border border-green-200">
                    {format!("{}: {} row(s) affected.", r.label, r.affected)}
                </div>
            })}

            <Card title="Inactive subscribers" hint="Delete confirmed, active subscribers whose timestamp is older than the threshold (timestamp bumps on send/click)."
                 action=format!("/app/lists/{list_token}/housekeeping/inactive")
                 placeholder="180" submit="Delete inactive"/>

            <Card title="Unconfirmed subscribers" hint="Delete subscribers who never confirmed their email and signed up before the threshold."
                 action=format!("/app/lists/{list_token}/housekeeping/unconfirmed")
                 placeholder="30" submit="Delete unconfirmed"/>

            <Card title="No-opens in last N days" hint="Delete subscribers who didn't open any campaign sent in the last N days."
                 action=format!("/app/lists/{list_token}/housekeeping/no-opens")
                 placeholder="90" submit="Delete no-openers"/>

            <Card title="No-clicks in last N days" hint="Delete subscribers who didn't click any link in any campaign sent in the last N days."
                 action=format!("/app/lists/{list_token}/housekeeping/no-clicks")
                 placeholder="90" submit="Delete no-clickers"/>

            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6 flex items-center justify-between gap-4">
                <div>
                    <h2 class="text-lg font-semibold text-gray-900 dark:text-white">"Remove duplicates"</h2>
                    <p class="text-sm text-gray-500 dark:text-gray-400">"Collapse duplicate (email × list) rows. The lowest id wins."</p>
                </div>
                <form method="POST" action=format!("/app/lists/{list_token}/housekeeping/dedupe")
                    onsubmit="return confirm('Remove duplicate subscribers from this list?');">
                    <button type="submit" class="text-white bg-rose-600 hover:bg-rose-700 font-medium rounded-lg text-sm px-4 py-2">
                        "Run dedupe"
                    </button>
                </form>
            </div>

            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6">
                <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">"Detect cross-list conflicts"</h2>
                <p class="text-sm text-gray-500 dark:text-gray-400 mb-3">"Find emails that appear in more than one list inside this brand."</p>
                <a href=format!("/app/brands/{brand_token}/housekeeping/conflicts")
                    class="inline-flex items-center text-blue-700 bg-blue-50 hover:bg-blue-100 font-medium rounded-lg text-sm px-4 py-2">
                    "Open conflict report"
                </a>
            </div>
        </section>
    }
}

#[derive(Debug, Clone)]
pub struct HousekeepingResult {
    pub label: String,
    pub affected: u64,
}

#[component]
fn Card(
    #[prop(into)] title: String,
    #[prop(into)] hint: String,
    #[prop(into)] action: String,
    #[prop(into)] placeholder: String,
    #[prop(into)] submit: String,
) -> impl IntoView {
    view! {
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6">
            <h2 class="text-lg font-semibold text-gray-900 dark:text-white">{title}</h2>
            <p class="text-sm text-gray-500 dark:text-gray-400 mt-1">{hint}</p>
            <form method="POST" action=action class="mt-4 flex flex-wrap gap-3 items-end"
                onsubmit="return confirm('This will permanently delete subscribers. Continue?');">
                <div class="grow">
                    <label for="days" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Days"</label>
                    <input type="number" id="days" name="days" min="1" required value=placeholder
                        class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
                </div>
                <button type="submit" class="text-white bg-rose-600 hover:bg-rose-700 font-medium rounded-lg text-sm px-4 py-2.5">{submit}</button>
            </form>
        </div>
    }
}

#[component]
pub fn ConflictReportPage(
    brand_token: String,
    brand_name: String,
    rows: Vec<crate::services::housekeeping::ConflictRow>,
) -> impl IntoView {
    let total = rows.len();
    view! {
        <section class="space-y-4">
            <nav class="text-sm text-gray-500 dark:text-gray-400">
                <a href="/app/brands" class="hover:underline">"Brands"</a>" / "
                <a href=format!("/app/brands/{}/lists", brand_token) class="hover:underline">{brand_name}</a>" / "
                <span class="text-gray-700 dark:text-gray-300">"Cross-list conflicts"</span>
            </nav>
            <header>
                <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">"Cross-list conflicts"</h1>
                <p class="text-sm text-gray-500 dark:text-gray-400">{format!("{total} email(s) appear in more than one list.")}</p>
            </header>
            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm overflow-x-auto">
                <table class="w-full text-sm text-left text-gray-500 dark:text-gray-400">
                    <thead class="text-xs text-gray-700 dark:text-gray-300 uppercase bg-gray-50 dark:bg-gray-700">
                        <tr>
                            <th class="px-4 py-3">"Email"</th>
                            <th class="px-4 py-3 text-right">"Lists"</th>
                            <th class="px-4 py-3">"List names"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {if rows.is_empty() {
                            view! { <tr><td colspan="3" class="text-center text-gray-400 dark:text-gray-500 py-10">"No conflicts."</td></tr> }.into_any()
                        } else {
                            rows.into_iter().map(|r| view! {
                                <tr class="bg-white dark:bg-gray-800 border-b">
                                    <td class="px-4 py-3 font-medium text-gray-900 dark:text-white">{r.email}</td>
                                    <td class="px-4 py-3 text-right">{r.list_count}</td>
                                    <td class="px-4 py-3 text-gray-600 dark:text-gray-400">{r.list_names.join(", ")}</td>
                                </tr>
                            }).collect_view().into_any()
                        }}
                    </tbody>
                </table>
            </div>
        </section>
    }
}

// ── Form payloads ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DaysForm {
    pub days: i64,
}

#[derive(Debug, Deserialize)]
pub struct ResultQuery {
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub n: Option<u64>,
}

// ── Handlers ────────────────────────────────────────────────────────────────

