//! Rules admin pages.

use leptos::prelude::*;
use serde::Deserialize;

use crate::database::entities::rules;

/// One row in the rules form's list-of-lists dropdown.
#[derive(Debug, Clone)]
pub struct ListOption {
    pub id: i32,
    pub name: String,
}

/// One row in the rules form's autoresponder dropdown.
#[derive(Debug, Clone)]
pub struct AresOption {
    pub id: i32,
    pub name: String,
}

#[component]
pub fn RulesListPage(
    brand_token: String,
    brand_name: String,
    rows: Vec<rules::Model>,
    #[prop(default = Vec::new())] lists: Vec<ListOption>,
    #[prop(default = Vec::new())] autoresponders: Vec<AresOption>,
    #[prop(default = Vec::new())] custom_fields: Vec<String>,
) -> impl IntoView {
    view! {
        <section class="space-y-4">
            <header class="flex justify-between items-center">
                <div>
                    <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">"Automation rules"</h1>
                    <p class="text-sm text-gray-500 dark:text-gray-400">{format!("Brand: {brand_name}")}</p>
                </div>
            </header>

            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6">
                <form method="POST" action=format!("/app/brands/{brand_token}/rules/save") class="space-y-4">
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-5">
                        <div>
                            <label for="trigger" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Trigger"</label>
                            <select id="trigger" name="trigger" required
                                class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5">
                                <option value="subscribe">"Subscribe"</option>
                                <option value="unsubscribe">"Unsubscribe"</option>
                                <option value="campaign_sending">"Campaign sending"</option>
                                <option value="campaign_sent">"Campaign sent"</option>
                            </select>
                        </div>
                        <div>
                            <label for="action" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Action"</label>
                            <select id="action" name="action" required
                                class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5">
                                <option value="notify">"Email notification"</option>
                                <option value="webhook">"Webhook"</option>
                            </select>
                        </div>
                        <div>
                            <label for="list_id" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Scope to list (optional)"</label>
                            <select id="list_id" name="list_id"
                                class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5">
                                <option value="">"— any list —"</option>
                                {lists.into_iter().map(|l| view! {
                                    <option value=l.id.to_string()>{l.name}</option>
                                }).collect_view()}
                            </select>
                        </div>
                        <div>
                            <label for="ares_id" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Autoresponder (only for ares triggers)"</label>
                            <select id="ares_id" name="ares_id"
                                class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5">
                                <option value="">"— none —"</option>
                                {autoresponders.into_iter().map(|a| view! {
                                    <option value=a.id.to_string()>{a.name}</option>
                                }).collect_view()}
                            </select>
                        </div>
                        <div>
                            <label for="notification_email" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Notification email"</label>
                            <input type="email" id="notification_email" name="notification_email"
                                class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
                        </div>
                        <div>
                            <label for="webhook_url" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Webhook URL"</label>
                            <input type="url" id="webhook_url" name="webhook_url"
                                class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
                        </div>
                    </div>
                    {if custom_fields.is_empty() {
                        ().into_any()
                    } else {
                        view! {
                            <p class="text-xs text-gray-500 dark:text-gray-400">
                                "Custom fields available on this brand's lists for use in webhook bodies / notification text: "
                                {custom_fields.into_iter().map(|f| view! {
                                    <code class="bg-gray-100 mx-1 px-1.5 py-0.5 rounded">{f}</code>
                                }).collect_view()}
                            </p>
                        }.into_any()
                    }}
                    <button type="submit" class="text-white bg-blue-600 hover:bg-blue-700 font-medium rounded-lg text-sm px-4 py-2.5">
                        "Add rule"
                    </button>
                </form>
            </div>

            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm overflow-x-auto">
                <table class="w-full text-sm text-left text-gray-500 dark:text-gray-400">
                    <thead class="text-xs text-gray-700 dark:text-gray-300 uppercase bg-gray-50 dark:bg-gray-700">
                        <tr>
                            <th class="px-4 py-3">"Trigger"</th>
                            <th class="px-4 py-3">"Action"</th>
                            <th class="px-4 py-3">"Target"</th>
                            <th class="px-4 py-3">"Status"</th>
                            <th class="px-4 py-3 w-1"></th>
                        </tr>
                    </thead>
                    <tbody>
                        {if rows.is_empty() {
                            view! { <tr><td colspan="5" class="text-center text-gray-400 dark:text-gray-500 py-10">"No rules defined yet."</td></tr> }.into_any()
                        } else {
                            rows.into_iter().map(|r| {
                                let toggle_action = format!("/app/rules/{}/toggle?brand={}", r.id, brand_token);
                                let delete_action = format!("/app/rules/{}/delete?brand={}", r.id, brand_token);
                                let is_enabled = r.enabled.unwrap_or(0) == 1;
                                view! {
                                    <tr class="bg-white dark:bg-gray-800 border-b">
                                        <td class="px-4 py-3">{r.trigger_col.unwrap_or_default()}</td>
                                        <td class="px-4 py-3">{r.action.unwrap_or_default()}</td>
                                        <td class="px-4 py-3">{r.notification_email.or(r.endpoint).unwrap_or_default()}</td>
                                        <td class="px-4 py-3">
                                            {if is_enabled {
                                                view! { <span class="bg-green-100 text-green-800 text-xs font-medium px-2.5 py-0.5 rounded">"Enabled"</span> }.into_any()
                                            } else {
                                                view! { <span class="bg-gray-200 text-gray-700 dark:text-gray-300 text-xs font-medium px-2.5 py-0.5 rounded">"Disabled"</span> }.into_any()
                                            }}
                                        </td>
                                        <td class="px-4 py-3 text-right">
                                            <div class="flex gap-3 justify-end">
                                                <form method="POST" action=toggle_action>
                                                    <button type="submit" class="text-blue-600 hover:underline text-sm">"Toggle"</button>
                                                </form>
                                                <form method="POST" action=delete_action onsubmit="return confirm('Delete this rule?');">
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
        </section>
    }
}

// ── Handlers ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RuleForm {
    pub trigger: String,
    pub action: String,
    #[serde(default)]
    pub notification_email: Option<String>,
    #[serde(default)]
    pub webhook_url: Option<String>,
    #[serde(default)]
    pub list_id: Option<String>,
    #[serde(default)]
    pub ares_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BrandQuery {
    pub brand: String,
}

