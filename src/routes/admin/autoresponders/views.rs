//! Autoresponders admin pages.

use leptos::prelude::*;
use serde::Deserialize;

use crate::database::entities::{ares, ares_emails};

// ── List-scoped page ────────────────────────────────────────────────────────

#[component]
pub fn ListAutorespondersPage(
    list_token: String,
    list_name: String,
    rows: Vec<ares::Model>,
) -> impl IntoView {
    view! {
        <section class="space-y-4">
            <header>
                <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">{list_name}</h1>
                <p class="text-sm text-gray-500 dark:text-gray-400">"Autoresponders attached to this list."</p>
            </header>
            <crate::components::layout::ListSubNav list_token=list_token.clone()/>

            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6">
                <form method="POST" action=format!("/app/lists/{list_token}/autoresponders/new") class="flex gap-3 items-end flex-wrap">
                    <div class="grow">
                        <label for="name" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Name"</label>
                        <input type="text" id="name" name="name" required placeholder="Welcome series"
                            class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
                    </div>
                    <button type="submit" class="text-white bg-primary-600 hover:bg-primary-700 font-medium rounded-lg text-sm px-4 py-2.5">"Create"</button>
                </form>
            </div>

            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm overflow-x-auto">
                <table class="w-full text-sm text-left text-gray-500 dark:text-gray-400">
                    <thead class="text-xs text-gray-700 dark:text-gray-300 uppercase bg-gray-50 dark:bg-gray-700">
                        <tr>
                            <th class="px-4 py-3">"Name"</th>
                            <th class="px-4 py-3">"Type"</th>
                            <th class="px-4 py-3 w-1"></th>
                        </tr>
                    </thead>
                    <tbody>
                        {if rows.is_empty() {
                            view! { <tr><td colspan="3" class="text-center text-gray-400 dark:text-gray-500 py-10">"No autoresponders yet."</td></tr> }.into_any()
                        } else {
                            let token = list_token.clone();
                            rows.into_iter().map(|a| {
                                let emails_href = format!("/app/autoresponders/{}/emails", a.id);
                                let delete_action = format!("/app/autoresponders/{}/delete?list={}", a.id, token);
                                view! {
                                    <tr class="bg-white dark:bg-gray-800 border-b">
                                        <td class="px-4 py-3 font-medium text-gray-900 dark:text-white">
                                            <a href=emails_href class="hover:underline">{a.name.unwrap_or_default()}</a>
                                        </td>
                                        <td class="px-4 py-3">{a.r#type.map(|t| if t == 0 {"Signup"} else {"Date field"}).unwrap_or("—")}</td>
                                        <td class="px-4 py-3 text-right">
                                            <form method="POST" action=delete_action onsubmit="return confirm('Delete this autoresponder?');">
                                                <button type="submit" class="text-red-600 hover:underline text-sm">"Delete"</button>
                                            </form>
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

// ── Pages ───────────────────────────────────────────────────────────────────

#[component]
pub fn AutorespondersListPage(
    brand_token: String,
    brand_name: String,
    rows: Vec<ares::Model>,
) -> impl IntoView {
    view! {
        <section class="space-y-4">
            <header class="flex justify-between items-center">
                <div>
                    <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">"Autoresponders"</h1>
                    <p class="text-sm text-gray-500 dark:text-gray-400">{format!("Brand: {brand_name}")}</p>
                </div>
            </header>
            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6">
                <form method="POST" action=format!("/app/brands/{brand_token}/autoresponders/new") class="flex gap-3 items-end flex-wrap">
                    <div class="grow">
                        <label for="name" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Name"</label>
                        <input type="text" id="name" name="name" required
                            placeholder="Welcome series"
                            class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
                    </div>
                    <div>
                        <label for="list_id" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"List ID"</label>
                        <input type="number" id="list_id" name="list_id" required min="1"
                            class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block p-2.5"/>
                    </div>
                    <button type="submit" class="text-white bg-blue-600 hover:bg-blue-700 font-medium rounded-lg text-sm px-4 py-2.5">"Create"</button>
                </form>
            </div>

            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm overflow-x-auto">
                <table class="w-full text-sm text-left text-gray-500 dark:text-gray-400">
                    <thead class="text-xs text-gray-700 dark:text-gray-300 uppercase bg-gray-50 dark:bg-gray-700">
                        <tr>
                            <th class="px-4 py-3">"Name"</th>
                            <th class="px-4 py-3">"Type"</th>
                            <th class="px-4 py-3">"List"</th>
                            <th class="px-4 py-3 w-1"></th>
                        </tr>
                    </thead>
                    <tbody>
                        {if rows.is_empty() {
                            view! { <tr><td colspan="4" class="text-center text-gray-400 dark:text-gray-500 py-10">"No autoresponders yet."</td></tr> }.into_any()
                        } else {
                            rows.into_iter().map(|a| {
                                let emails_href = format!("/app/autoresponders/{}/emails", a.id);
                                let delete_action = format!("/app/autoresponders/{}/delete?brand={}", a.id, brand_token);
                                view! {
                                    <tr class="bg-white dark:bg-gray-800 border-b">
                                        <td class="px-4 py-3 font-medium text-gray-900 dark:text-white">
                                            <a href=emails_href class="hover:underline">{a.name.unwrap_or_default()}</a>
                                        </td>
                                        <td class="px-4 py-3">{a.r#type.map(|t| if t == 0 {"Signup"} else {"Date field"}).unwrap_or("—")}</td>
                                        <td class="px-4 py-3">{a.list.unwrap_or_default()}</td>
                                        <td class="px-4 py-3 text-right">
                                            <form method="POST" action=delete_action onsubmit="return confirm('Delete this autoresponder?');">
                                                <button type="submit" class="text-red-600 hover:underline text-sm">"Delete"</button>
                                            </form>
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

#[component]
pub fn EmailsPage(
    autoresponder_id: i32,
    autoresponder_name: String,
    emails: Vec<ares_emails::Model>,
) -> impl IntoView {
    let new_email_url = format!("/app/autoresponders/{autoresponder_id}/emails/new");
    view! {
        <section class="space-y-4">
            <header class="flex justify-between items-center">
                <div>
                    <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">{autoresponder_name}</h1>
                    <p class="text-sm text-gray-500 dark:text-gray-400">"Emails in this autoresponder sequence."</p>
                </div>
                <a href=new_email_url
                    class="text-white bg-primary-600 hover:bg-primary-700 font-medium rounded-lg text-sm px-4 py-2.5">
                    "Add email"
                </a>
            </header>
            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm overflow-x-auto">
                <table class="w-full text-sm text-left text-gray-500 dark:text-gray-400">
                    <thead class="text-xs text-gray-700 dark:text-gray-300 uppercase bg-gray-50 dark:bg-gray-700">
                        <tr>
                            <th class="px-4 py-3">"Subject"</th>
                            <th class="px-4 py-3">"Delay"</th>
                            <th class="px-4 py-3 text-right">"Recipients"</th>
                            <th class="px-4 py-3">"Status"</th>
                            <th class="px-4 py-3 w-1"></th>
                        </tr>
                    </thead>
                    <tbody>
                        {if emails.is_empty() {
                            view! { <tr><td colspan="5" class="text-center text-gray-400 dark:text-gray-500 py-10">"No emails configured yet."</td></tr> }.into_any()
                        } else {
                            emails.into_iter().map(|e| {
                                let edit_url = format!("/app/autoresponder-emails/{}/edit", e.id);
                                let toggle_url = format!("/app/autoresponder-emails/{}/toggle", e.id);
                                let dup_url = format!("/app/autoresponder-emails/{}/duplicate", e.id);
                                let delete_url = format!("/app/autoresponder-emails/{}/delete", e.id);
                                let enabled = e.enabled.unwrap_or(0) == 1;
                                view! {
                                    <tr class="bg-white dark:bg-gray-800 border-b">
                                        <td class="px-4 py-3 font-medium text-gray-900 dark:text-white">
                                            <a href=edit_url class="hover:underline">{e.title.unwrap_or_default()}</a>
                                        </td>
                                        <td class="px-4 py-3 text-xs text-gray-500 dark:text-gray-400">{e.time_condition.unwrap_or_default()}</td>
                                        <td class="px-4 py-3 text-right">{e.recipients.unwrap_or(0)}</td>
                                        <td class="px-4 py-3">
                                            {if enabled {
                                                view! { <span class="bg-green-100 text-green-800 text-xs font-medium px-2.5 py-0.5 rounded">"Enabled"</span> }.into_any()
                                            } else {
                                                view! { <span class="bg-gray-200 text-gray-700 dark:text-gray-300 text-xs font-medium px-2.5 py-0.5 rounded">"Paused"</span> }.into_any()
                                            }}
                                        </td>
                                        <td class="px-4 py-3 text-right whitespace-nowrap">
                                            <a href=format!("/app/autoresponder-emails/{}/report", e.id)
                                                class="text-gray-600 dark:text-gray-300 hover:underline text-sm me-3">"Report"</a>
                                            <form method="POST" action=toggle_url class="inline">
                                                <button type="submit" class="text-blue-600 hover:underline text-sm me-3">
                                                    {if enabled {"Pause"} else {"Enable"}}
                                                </button>
                                            </form>
                                            <form method="POST" action=dup_url class="inline">
                                                <button type="submit" class="text-emerald-700 hover:underline text-sm me-3">"Duplicate"</button>
                                            </form>
                                            <form method="POST" action=delete_url class="inline"
                                                onsubmit="return confirm('Delete this email?');">
                                                <button type="submit" class="text-red-600 hover:underline text-sm">"Delete"</button>
                                            </form>
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

/// One row in the segments multi-select.
#[derive(Debug, Clone)]
pub struct SegmentOption {
    pub id: i32,
    pub name: String,
}

#[component]
pub fn EmailFormPage(
    autoresponder_id: i32,
    autoresponder_name: String,
    #[prop(optional)] existing: Option<ares_emails::Model>,
    #[prop(default = Vec::new())] attachments: Vec<String>,
    #[prop(default = Vec::new())] segments: Vec<SegmentOption>,
) -> impl IntoView {
    let is_edit = existing.is_some();
    let action = match existing.as_ref() {
        Some(m) => format!("/app/autoresponder-emails/{}/edit", m.id),
        None => format!("/app/autoresponders/{autoresponder_id}/emails/new"),
    };
    let title = if is_edit { "Edit email" } else { "New email" };
    let m = existing.unwrap_or_else(|| ares_emails::Model {
        id: 0,
        ares_id: Some(autoresponder_id),
        from_name: None,
        from_email: None,
        reply_to: None,
        title: None,
        plain_text: None,
        html_text: None,
        query_string: None,
        time_condition: None,
        timezone: None,
        created: None,
        recipients: None,
        opens: None,
        wysiwyg: Some(1),
        opens_tracking: Some(1),
        links_tracking: Some(1),
        enabled: Some(0),
        segs: None,
        segs_excl: None,
    });
    view! {
        <section class="space-y-6">
            <header>
                <p class="text-xs text-gray-500 dark:text-gray-400">{format!("Autoresponder: {autoresponder_name}")}</p>
                <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">{title}</h1>
            </header>
            <form method="POST" action=action class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6 space-y-5 max-w-3xl">
                <div>
                    <label for="title" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Subject"</label>
                    <input type="text" id="title" name="title" required value=m.title.clone().unwrap_or_default()
                        class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
                </div>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-5">
                    <div>
                        <label for="from_name" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"From name"</label>
                        <input type="text" id="from_name" name="from_name" value=m.from_name.clone().unwrap_or_default()
                            class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
                    </div>
                    <div>
                        <label for="from_email" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"From email"</label>
                        <input type="email" id="from_email" name="from_email" value=m.from_email.clone().unwrap_or_default()
                            class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
                    </div>
                </div>
                <div>
                    <label for="reply_to" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Reply-to"</label>
                    <input type="email" id="reply_to" name="reply_to" value=m.reply_to.clone().unwrap_or_default()
                        class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
                </div>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-5">
                    <div>
                        <label for="time_condition" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">
                            "Send after (e.g. \"3 days\", \"1 hour\", \"30 minutes\", or empty for immediate)"
                        </label>
                        <input type="text" id="time_condition" name="time_condition" value=m.time_condition.clone().unwrap_or_default()
                            placeholder="3 days"
                            class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
                    </div>
                    <div>
                        <label for="timezone" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Timezone"</label>
                        <input type="text" id="timezone" name="timezone" value=m.timezone.clone().unwrap_or_else(|| "UTC".into())
                            class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
                    </div>
                </div>
                <crate::components::layout::PersonalizationTagsCard/>
                <div class="space-y-2">
                    <label for="html_text" class="block text-sm font-medium text-gray-900 dark:text-white">"HTML body"</label>
                    <div data-templatical-editor data-target-input="html_text" class="space-y-2">
                        <div class="templatical-inline-container w-full rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 overflow-hidden" style="height: 500px;"></div>
                        <details class="text-xs text-gray-500 dark:text-gray-400">
                            <summary class="cursor-pointer hover:underline py-1">"View / edit raw HTML source"</summary>
                            <textarea id="html_text" name="html_text" rows="14"
                                class="wysiwyg bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5 font-mono">
                                {m.html_text.clone().unwrap_or_default()}
                            </textarea>
                        </details>
                    </div>
                </div>
                <div>
                    <label for="plain_text" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Plain-text body (optional)"</label>
                    <textarea id="plain_text" name="plain_text" rows="6"
                        class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5 font-mono">
                        {m.plain_text.clone().unwrap_or_default()}
                    </textarea>
                </div>
                {if !segments.is_empty() {
                    let included: std::collections::HashSet<i32> = m
                        .segs
                        .as_deref()
                        .unwrap_or("")
                        .split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect();
                    let excluded: std::collections::HashSet<i32> = m
                        .segs_excl
                        .as_deref()
                        .unwrap_or("")
                        .split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect();
                    let in_segs = segments.clone();
                    let out_segs = segments.clone();
                    view! {
                        <fieldset class="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
                            <legend class="text-sm font-semibold px-2 text-gray-700 dark:text-gray-300">"Segment filtering"</legend>
                            <div class="grid grid-cols-1 md:grid-cols-2 gap-5">
                                <div>
                                    <p class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Send only to (any of):"</p>
                                    <div class="space-y-1">
                                        {in_segs.into_iter().map(|s| {
                                            let checked = included.contains(&s.id);
                                            view! {
                                                <label class="flex items-center gap-2 text-sm text-gray-700 dark:text-gray-300">
                                                    <input type="checkbox" name="segs" value=s.id.to_string()
                                                        checked=checked
                                                        class="h-4 w-4 text-primary-600 border-gray-300 dark:border-gray-600 rounded"/>
                                                    {s.name}
                                                </label>
                                            }
                                        }).collect_view()}
                                    </div>
                                </div>
                                <div>
                                    <p class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Skip anyone in:"</p>
                                    <div class="space-y-1">
                                        {out_segs.into_iter().map(|s| {
                                            let checked = excluded.contains(&s.id);
                                            view! {
                                                <label class="flex items-center gap-2 text-sm text-gray-700 dark:text-gray-300">
                                                    <input type="checkbox" name="segs_excl" value=s.id.to_string()
                                                        checked=checked
                                                        class="h-4 w-4 text-primary-600 border-gray-300 dark:border-gray-600 rounded"/>
                                                    {s.name}
                                                </label>
                                            }
                                        }).collect_view()}
                                    </div>
                                </div>
                            </div>
                            <p class="mt-2 text-xs text-gray-500 dark:text-gray-400">
                                "Leave \"send only to\" empty to send to everyone on the list (minus exclusions)."
                            </p>
                        </fieldset>
                    }.into_any()
                } else {
                    ().into_any()
                }}
                <div class="flex flex-wrap gap-6 pt-2">
                    <label class="inline-flex items-center gap-2 text-sm text-gray-700 dark:text-gray-300">
                        <input type="checkbox" name="opens_tracking" value="1"
                            checked=m.opens_tracking.unwrap_or(1) == 1
                            class="h-4 w-4 text-primary-600 border-gray-300 dark:border-gray-600 rounded"/>
                        "Track opens"
                    </label>
                    <label class="inline-flex items-center gap-2 text-sm text-gray-700 dark:text-gray-300">
                        <input type="checkbox" name="links_tracking" value="1"
                            checked=m.links_tracking.unwrap_or(1) == 1
                            class="h-4 w-4 text-primary-600 border-gray-300 dark:border-gray-600 rounded"/>
                        "Track clicks"
                    </label>
                </div>
                <div class="flex justify-end gap-3 pt-2">
                    <a href=format!("/app/autoresponders/{autoresponder_id}/emails")
                        class="text-sm font-medium text-gray-600 dark:text-gray-400 hover:underline self-center">"Cancel"</a>
                    {if is_edit {
                        let eid = m.id;
                        view! {
                            <a href=format!("/app/autoresponder-emails/{eid}/preview") target="_blank" rel="noopener"
                                class="text-sm font-medium text-gray-700 dark:text-gray-200 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-600 rounded-lg px-4 py-2.5">
                                <i class="fa-regular fa-eye me-1"></i>"Preview"
                            </a>
                        }.into_any()
                    } else {
                        ().into_any()
                    }}
                    <button type="submit"
                        class="text-white bg-primary-600 hover:bg-primary-700 font-medium rounded-lg text-sm px-5 py-2.5">
                        {if is_edit {"Save email"} else {"Create email"}}
                    </button>
                </div>
            </form>

            {if is_edit {
                let email_id = m.id;
                view! {
                    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6 max-w-3xl space-y-4">
                        <h2 class="text-lg font-semibold text-gray-900 dark:text-white">"Test send"</h2>
                        <form method="POST" action=format!("/app/autoresponder-emails/{email_id}/test-send")
                            class="flex flex-wrap gap-3 items-end">
                            <div class="grow">
                                <label for="email" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Send a preview to"</label>
                                <input type="email" id="email" name="email" required placeholder="you@example.com"
                                    class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
                            </div>
                            <button type="submit" class="text-white bg-slate-700 hover:bg-slate-800 font-medium rounded-lg text-sm px-4 py-2.5">
                                "Send test"
                            </button>
                        </form>
                    </div>
                    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6 max-w-3xl space-y-4">
                        <h2 class="text-lg font-semibold text-gray-900 dark:text-white">"Attachments"</h2>
                        <p class="text-sm text-gray-500 dark:text-gray-400">"Files attached here ride along with every send (max 10 MB each)."</p>
                        {if attachments.is_empty() {
                            view! { <p class="text-sm text-gray-400 dark:text-gray-500">"No attachments yet."</p> }.into_any()
                        } else {
                            view! {
                                <ul class="divide-y divide-gray-200 dark:divide-gray-700 border border-gray-200 dark:border-gray-700 rounded-md">
                                    {attachments.into_iter().map(|name| view! {
                                        <li class="flex items-center justify-between px-3 py-2 text-sm">
                                            <span class="font-mono text-gray-700 dark:text-gray-300">{name.clone()}</span>
                                            <form method="POST" action=format!("/app/autoresponder-emails/{email_id}/attachments/delete")
                                                onsubmit="return confirm('Remove this attachment?');">
                                                <input type="hidden" name="filename" value=name/>
                                                <button type="submit" class="text-red-600 hover:underline text-xs">"Remove"</button>
                                            </form>
                                        </li>
                                    }).collect_view()}
                                </ul>
                            }.into_any()
                        }}
                        <form method="POST" action=format!("/app/autoresponder-emails/{email_id}/attachments/upload") enctype="multipart/form-data"
                            class="flex flex-wrap gap-3 items-end">
                            <div class="grow">
                                <label for="file" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Add attachment"</label>
                                <input type="file" id="file" name="file" required
                                    class="block w-full text-sm text-gray-900 dark:text-white border border-gray-300 dark:border-gray-600 rounded-lg cursor-pointer bg-gray-50 dark:bg-gray-700"/>
                            </div>
                            <button type="submit" class="text-white bg-emerald-600 hover:bg-emerald-700 font-medium rounded-lg text-sm px-4 py-2.5">
                                "Attach"
                            </button>
                        </form>
                    </div>
                }.into_any()
            } else {
                ().into_any()
            }}
        </section>
    }
}

#[component]
pub fn EmailReportPage(report: crate::services::reports::AresEmailReport) -> impl IntoView {
    let csv_url = format!("/app/autoresponder-emails/{}/report.csv", report.email_id);
    view! {
        <section class="space-y-6">
            <header class="flex justify-between items-end gap-3 flex-wrap">
                <div>
                    <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">{report.title.clone()}</h1>
                    <p class="text-sm text-gray-500 dark:text-gray-400">"Autoresponder email — engagement summary"</p>
                </div>
                <a href=csv_url
                    class="text-sm font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 rounded-lg px-3 py-2">
                    "Export CSV"
                </a>
            </header>
            <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
                <Stat label="Recipients" value=report.recipients/>
                <Stat label="Unique opens" value=report.unique_opens/>
                <Stat label="Total opens" value=report.total_opens/>
                <Stat label="Unique clickers" value=report.unique_clickers/>
            </div>
            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm overflow-x-auto">
                <table class="w-full text-sm text-left text-gray-500 dark:text-gray-400">
                    <thead class="text-xs text-gray-700 dark:text-gray-300 uppercase bg-gray-50 dark:bg-gray-700">
                        <tr>
                            <th class="px-4 py-3">"Top link"</th>
                            <th class="px-4 py-3 text-right">"Clicks"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {if report.top_links.is_empty() {
                            view! { <tr><td colspan="2" class="text-center text-gray-400 dark:text-gray-500 py-10">"No clicks recorded."</td></tr> }.into_any()
                        } else {
                            report.top_links.into_iter().map(|l| view! {
                                <tr class="bg-white dark:bg-gray-800 border-b">
                                    <td class="px-4 py-3 font-mono text-xs text-gray-700 dark:text-gray-300 break-all">{l.url}</td>
                                    <td class="px-4 py-3 text-right">{l.clicks}</td>
                                </tr>
                            }).collect_view().into_any()
                        }}
                    </tbody>
                </table>
            </div>
        </section>
    }
}

#[component]
fn Stat(#[prop(into)] label: String, value: i64) -> impl IntoView {
    view! {
        <div class="bg-white dark:bg-gray-800 rounded-lg p-4 shadow-sm">
            <p class="text-xs uppercase tracking-wider text-gray-500 dark:text-gray-400">{label}</p>
            <p class="mt-1 text-2xl font-bold text-gray-900 dark:text-white">{value}</p>
        </div>
    }
}

// ── Handlers ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AresForm {
    pub name: String,
    pub list_id: i32,
}

#[derive(Debug, Deserialize)]
pub struct ListAresForm {
    pub name: String,
}

