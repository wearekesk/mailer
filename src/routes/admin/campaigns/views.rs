//! Campaigns admin pages — list, create/edit, reports.

use leptos::prelude::*;
use serde::Deserialize;

use crate::database::entities::campaigns;
use crate::utils::opaque_id::{Campaign, OpaqueId};

pub(crate) const PAGE_SIZE: u64 = 50;

/// Curated list of common IANA timezone names presented in the schedule
/// dropdown. The full chrono-tz database has ~600 entries — we surface the
/// 25 most common to keep the UI legible. Users with exotic timezones can
/// still select via the underlying chrono_tz parser; we just need a UI
/// element for the common case.
const COMMON_TIMEZONES: &[&str] = &[
    "UTC",
    "America/Los_Angeles",
    "America/Denver",
    "America/Chicago",
    "America/New_York",
    "America/Toronto",
    "America/Mexico_City",
    "America/Sao_Paulo",
    "America/Argentina/Buenos_Aires",
    "Europe/London",
    "Europe/Dublin",
    "Europe/Lisbon",
    "Europe/Paris",
    "Europe/Berlin",
    "Europe/Madrid",
    "Europe/Rome",
    "Europe/Amsterdam",
    "Europe/Stockholm",
    "Europe/Moscow",
    "Africa/Cairo",
    "Africa/Johannesburg",
    "Asia/Dubai",
    "Asia/Karachi",
    "Asia/Kolkata",
    "Asia/Bangkok",
    "Asia/Singapore",
    "Asia/Hong_Kong",
    "Asia/Shanghai",
    "Asia/Tokyo",
    "Asia/Seoul",
    "Australia/Perth",
    "Australia/Sydney",
    "Pacific/Auckland",
];

// ── Pages ───────────────────────────────────────────────────────────────────

#[component]
pub fn CampaignsListPage(
    brand_token: String,
    brand_name: String,
    rows: Vec<campaigns::Model>,
    page: u64,
    total: u64,
) -> impl IntoView {
    let new_href = format!("/app/brands/{}/campaigns/new", brand_token);
    let last_page = if total == 0 { 0 } else { (total.saturating_sub(1)) / PAGE_SIZE };
    view! {
        <section class="space-y-4">
            <header class="flex justify-between items-center">
                <div>
                    <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">"Campaigns"</h1>
                    <p class="text-sm text-gray-500 dark:text-gray-400">{format!("Brand: {brand_name} — {total}")}</p>
                </div>
                <a href=new_href class="text-white bg-blue-600 hover:bg-blue-700 font-medium rounded-lg text-sm px-4 py-2">
                    "+ New campaign"
                </a>
            </header>
            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm overflow-x-auto">
                <table class="w-full text-sm text-left text-gray-500 dark:text-gray-400">
                    <thead class="text-xs text-gray-700 dark:text-gray-300 uppercase bg-gray-50 dark:bg-gray-700">
                        <tr>
                            <th class="px-4 py-3">"Title"</th>
                            <th class="px-4 py-3">"Subject"</th>
                            <th class="px-4 py-3">"Status"</th>
                            <th class="px-4 py-3 text-right">"Recipients"</th>
                            <th class="px-4 py-3 w-1"></th>
                        </tr>
                    </thead>
                    <tbody>
                        {if rows.is_empty() {
                            view! { <tr><td colspan="5" class="text-center text-gray-400 dark:text-gray-500 py-10">"No campaigns yet."</td></tr> }.into_any()
                        } else {
                            rows.into_iter().map(|c| {
                                let token = OpaqueId::<Campaign>::new(c.id).to_string();
                                let edit_href = format!("/app/campaigns/{}/edit", token);
                                let report_href = format!("/app/campaigns/{}/report", token);
                                let (status_label, status_cls) = if c.sent.as_deref().unwrap_or("").is_empty() && c.send_date.is_none() {
                                    ("Draft", "bg-gray-200 text-gray-700 dark:text-gray-300")
                                } else if !c.sent.as_deref().unwrap_or("").is_empty() && c.to_send.unwrap_or(0) <= c.recipients.unwrap_or(0) {
                                    ("Sent", "bg-green-100 text-green-800")
                                } else if c.send_date.is_some() {
                                    ("Scheduled", "bg-amber-100 text-amber-800")
                                } else {
                                    ("Sending", "bg-blue-100 text-blue-800")
                                };
                                let badge_class = format!("text-xs font-medium px-2.5 py-0.5 rounded {}", status_cls);
                                view! {
                                    <tr class="bg-white dark:bg-gray-800 border-b">
                                        <td class="px-4 py-3 font-medium text-gray-900 dark:text-white">
                                            <a href=edit_href.clone() class="hover:underline">{c.title.unwrap_or_default()}</a>
                                        </td>
                                        <td class="px-4 py-3">{c.subject.unwrap_or_default()}</td>
                                        <td class="px-4 py-3"><span class=badge_class>{status_label}</span></td>
                                        <td class="px-4 py-3 text-right">{format!("{}/{}", c.recipients.unwrap_or(0), c.to_send.unwrap_or(0))}</td>
                                        <td class="px-4 py-3 text-right">
                                            <div class="flex gap-3 justify-end">
                                                <a href=edit_href class="text-blue-600 hover:underline text-sm">"Edit"</a>
                                                <a href=report_href class="text-blue-600 hover:underline text-sm">"Report"</a>
                                            </div>
                                        </td>
                                    </tr>
                                }
                            }).collect_view().into_any()
                        }}
                    </tbody>
                </table>
            </div>
            <Pagination brand_token=brand_token page=page last=last_page/>
        </section>
    }
}

#[component]
fn Pagination(
    #[prop(into)] brand_token: String,
    page: u64,
    last: u64,
) -> impl IntoView {
    let prev = if page > 0 {
        Some(format!("/app/brands/{brand_token}/campaigns?page={}", page - 1))
    } else { None };
    let next = if page < last {
        Some(format!("/app/brands/{brand_token}/campaigns?page={}", page + 1))
    } else { None };
    view! {
        <nav class="flex justify-between items-center text-sm">
            <span class="text-gray-500 dark:text-gray-400">{format!("Page {} of {}", page + 1, last + 1)}</span>
            <div class="flex gap-2">
                {prev.map(|h| view! { <a href=h class="px-3 py-1 border rounded hover:bg-gray-50 dark:hover:bg-gray-600">"Previous"</a> })}
                {next.map(|h| view! { <a href=h class="px-3 py-1 border rounded hover:bg-gray-50 dark:hover:bg-gray-600">"Next"</a> })}
            </div>
        </nav>
    }
}

#[component]
pub fn CampaignFormPage(
    #[prop(into)] brand_token: String,
    #[prop(optional)] existing: Option<campaigns::Model>,
    #[prop(default = Vec::new())] attachments: Vec<String>,
    #[prop(default = Vec::new())] broken_links: Vec<String>,
) -> impl IntoView {
    let is_edit = existing.is_some();
    let action = match existing.as_ref() {
        Some(m) => format!("/app/campaigns/{}/edit", OpaqueId::<Campaign>::new(m.id)),
        None => format!("/app/brands/{}/campaigns/new", brand_token),
    };
    let title = if is_edit { "Edit campaign" } else { "New campaign" };
    let m = existing.unwrap_or_else(blank_campaign);
    let cid = m.id;
    let ignore_checks = m.ignore_checks.unwrap_or(0) == 1;
    let show_link_banner = is_edit && !broken_links.is_empty() && !ignore_checks;
    let link_banner_action = format!(
        "/app/campaigns/{}/dismiss-broken-link-checks",
        OpaqueId::<Campaign>::new(cid)
    );
    let link_banner_items: Vec<String> = broken_links.iter().take(8).cloned().collect();
    let link_banner_total = broken_links.len();

    let token = OpaqueId::<Campaign>::new(cid).to_string();
    let report_href = format!("/app/campaigns/{}/report", token);

    view! {
        <section class="space-y-6">
            <header class="flex items-center justify-between max-w-3xl">
                <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">{title}</h1>
                {if is_edit {
                    let r_href = report_href.clone();
                    view! {
                        <a href=r_href
                            class="inline-flex items-center gap-1.5 text-sm font-medium text-blue-700 bg-blue-50 dark:bg-blue-900/30 dark:text-blue-300 hover:bg-blue-100 dark:hover:bg-blue-900/50 rounded-lg px-3.5 py-2 transition-colors">
                            <i class="fa-solid fa-chart-column text-xs"></i>"View Report"
                        </a>
                    }.into_any()
                } else {
                    ().into_any()
                }}
            </header>
            {if show_link_banner {
                view! {
                    <div class="max-w-3xl rounded-lg border border-amber-300 dark:border-amber-700 bg-amber-50 dark:bg-amber-900/30 p-4">
                        <div class="flex items-start justify-between gap-4">
                            <div>
                                <p class="text-sm font-semibold text-amber-800 dark:text-amber-200">
                                    {format!("{link_banner_total} suspicious link(s) found in the HTML body")}
                                </p>
                                <p class="text-xs text-amber-700 dark:text-amber-300 mt-1">
                                    "Links missing a scheme (http://, https://, mailto:) probably won't render correctly. Showing the first few:"
                                </p>
                                <ul class="text-xs font-mono text-amber-800 dark:text-amber-200 mt-2 list-disc list-inside break-all">
                                    {link_banner_items.into_iter().map(|u| view! { <li>{u}</li> }).collect_view()}
                                </ul>
                            </div>
                            <form method="POST" action=link_banner_action>
                                <button type="submit"
                                    class="text-sm font-medium text-amber-800 dark:text-amber-200 bg-amber-100 dark:bg-amber-800/40 hover:bg-amber-200 dark:hover:bg-amber-800/60 rounded-lg px-3 py-2">
                                    "Send anyway"
                                </button>
                            </form>
                        </div>
                    </div>
                }.into_any()
            } else {
                ().into_any()
            }}
            <form method="POST" action=action class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6 space-y-5 max-w-3xl">
                <Field id="title" label="Internal title" required=true value=m.title.clone().unwrap_or_default()/>
                <Field id="subject" label="Subject line" required=true value=m.subject.clone().unwrap_or_default()/>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-5">
                    <Field id="from_name" label="From name" value=m.from_name.clone().unwrap_or_default()/>
                    <Field id="from_email" label="From email" input_type="email" value=m.from_email.clone().unwrap_or_default()/>
                </div>
                <Field id="reply_to" label="Reply-to" input_type="email" value=m.reply_to.clone().unwrap_or_default()/>
                <Field id="query_string" label="Query string appended to every link (e.g. utm_source=newsletter&utm_medium=email)"
                    value=m.query_string.clone().unwrap_or_default()
                    placeholder="utm_source=newsletter"/>
                // Flowbite Tabbed Content: Visual Builder (Templatical), HTML Source & Plain text body
                <div class="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden bg-gray-50/50 dark:bg-gray-900/50 p-4 space-y-4">
                    <div class="border-b border-gray-200 dark:border-gray-700 flex flex-wrap items-center justify-between gap-3 pb-1">
                        <ul class="flex flex-wrap -mb-px text-sm font-medium text-center" id="campaign-editor-tabs" role="tablist" data-tabs-toggle="#campaign-editor-tab-content" data-tabs-active-classes="text-primary-600 dark:text-primary-300 border-primary-600 dark:border-primary-300" data-tabs-inactive-classes="text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300 border-transparent hover:border-gray-300 dark:hover:border-gray-600">
                            <li class="me-2" role="presentation">
                                <button class="inline-flex items-center gap-2 px-4 py-2.5 border-b-2 rounded-t-lg" id="camp-visual-tab" data-tabs-target="#camp-visual-panel" type="button" role="tab" aria-controls="camp-visual-panel" aria-selected="true">
                                    <i class="fa-solid fa-wand-magic-sparkles text-indigo-500"></i>"Visual Editor (Templatical)"
                                </button>
                            </li>
                            <li class="me-2" role="presentation">
                                <button class="inline-flex items-center gap-2 px-4 py-2.5 border-b-2 rounded-t-lg" id="camp-html-tab" data-tabs-target="#camp-html-panel" type="button" role="tab" aria-controls="camp-html-panel" aria-selected="false">
                                    <i class="fa-solid fa-code"></i>"HTML Source"
                                </button>
                            </li>
                            <li class="me-2" role="presentation">
                                <button class="inline-flex items-center gap-2 px-4 py-2.5 border-b-2 rounded-t-lg" id="camp-text-tab" data-tabs-target="#camp-text-panel" type="button" role="tab" aria-controls="camp-text-panel" aria-selected="false">
                                    <i class="fa-solid fa-align-left"></i>"Plain-text Body"
                                </button>
                            </li>
                        </ul>
                    </div>
                    <div id="campaign-editor-tab-content">
                        <div id="camp-visual-panel" role="tabpanel" aria-labelledby="camp-visual-tab" data-templatical-editor data-target-input="html_text">
                            <div class="templatical-inline-container w-full rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 overflow-hidden" style="height: 650px;"></div>
                        </div>
                        <div class="hidden" id="camp-html-panel" role="tabpanel" aria-labelledby="camp-html-tab">
                            <Textarea id="html_text" label="" rows=18 wysiwyg=true value=m.html_text.clone().unwrap_or_default()/>
                        </div>
                        <div class="hidden" id="camp-text-panel" role="tabpanel" aria-labelledby="camp-text-tab">
                            <Textarea id="plain_text" label="" rows=18 value=m.plain_text.clone().unwrap_or_default()/>
                        </div>
                    </div>
                </div>
                <label class="inline-flex items-center gap-2 text-sm text-gray-700 dark:text-gray-300">
                    <input type="checkbox" name="save_as_template" value="1"
                        class="h-4 w-4 text-primary-600 border-gray-300 dark:border-gray-600 rounded"/>
                    "Also save this body as a reusable template under this brand"
                </label>
                <div class="flex justify-end gap-3 pt-2">
                    <a href=format!("/app/brands/{brand_token}/campaigns") class="text-sm font-medium text-gray-600 dark:text-gray-400 hover:underline self-center">"Cancel"</a>
                    {if is_edit {
                        let token = OpaqueId::<Campaign>::new(cid).to_string();
                        let r_href = report_href.clone();
                        view! {
                            <a href=format!("/app/campaigns/{token}/preview") target="_blank" rel="noopener"
                                class="text-sm font-medium text-gray-700 dark:text-gray-200 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-600 rounded-lg px-4 py-2.5">
                                <i class="fa-regular fa-eye me-1"></i>"Preview"
                            </a>
                            <a href=r_href
                                class="text-sm font-medium text-blue-700 dark:text-blue-300 bg-blue-50 dark:bg-blue-900/30 border border-blue-200 dark:border-blue-800 hover:bg-blue-100 dark:hover:bg-blue-900/50 rounded-lg px-4 py-2.5">
                                <i class="fa-solid fa-chart-column me-1"></i>"Report"
                            </a>
                        }.into_any()
                    } else {
                        ().into_any()
                    }}
                    <button type="submit" class="text-white bg-primary-600 hover:bg-primary-700 font-medium rounded-lg text-sm px-5 py-2.5">
                        {if is_edit {"Save draft"} else {"Create draft"}}
                    </button>
                </div>
            </form>
            {if is_edit {
                let token = OpaqueId::<Campaign>::new(cid).to_string();
                view! {
                    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6 space-y-4 max-w-3xl">
                        <h2 class="text-lg font-semibold text-gray-900 dark:text-white">"Send"</h2>
                        <form method="POST" action=format!("/app/campaigns/{token}/send-now") class="flex flex-wrap gap-3 items-end">
                            <div class="grow">
                                <label for="list_ids" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"List IDs (comma-separated, numeric)"</label>
                                <input type="text" id="list_ids" name="list_ids" placeholder="1,2,3"
                                    class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
                            </div>
                            <button type="submit" class="text-white bg-green-600 hover:bg-green-700 font-medium rounded-lg text-sm px-4 py-2.5">
                                "Send now"
                            </button>
                        </form>
                        <form method="POST" action=format!("/app/campaigns/{token}/stop") onsubmit="return confirm('Stop this campaign mid-send?');">
                            <button type="submit" class="text-amber-700 hover:underline text-sm">"Stop campaign"</button>
                        </form>
                        <hr class="my-2"/>
                        <h2 class="text-lg font-semibold text-gray-900 dark:text-white">"Schedule"</h2>
                        <form method="POST" action=format!("/app/campaigns/{token}/schedule") class="grid grid-cols-1 md:grid-cols-3 gap-3 items-end">
                            <div class="md:col-span-2">
                                <label for="send_at" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Send at"</label>
                                <input type="datetime-local" id="send_at" name="send_at" required
                                    class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
                            </div>
                            <div>
                                <label for="timezone" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Timezone"</label>
                                <select id="timezone" name="timezone"
                                    class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5">
                                    {COMMON_TIMEZONES.iter().map(|tz| view! {
                                        <option value=*tz selected=*tz == "UTC">{*tz}</option>
                                    }).collect_view()}
                                </select>
                            </div>
                            <div class="md:col-span-3 flex flex-wrap gap-3 items-end">
                                <input type="text" name="list_ids" placeholder="List IDs (comma-separated, numeric)"
                                    class="grow bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg p-2.5"/>
                                <button type="submit" class="text-white bg-purple-600 hover:bg-purple-700 font-medium rounded-lg text-sm px-4 py-2.5">
                                    "Schedule"
                                </button>
                            </div>
                        </form>

                        <hr class="my-2"/>
                        <h2 class="text-lg font-semibold text-gray-900 dark:text-white">"Test send"</h2>
                        <form method="POST" action=format!("/app/campaigns/{token}/test-send") class="flex flex-wrap gap-3 items-end">
                            <div class="grow">
                                <label for="email" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Send a preview to"</label>
                                <input type="email" id="email" name="email" required placeholder="you@example.com"
                                    class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
                            </div>
                            <button type="submit" class="text-white bg-slate-700 hover:bg-slate-800 font-medium rounded-lg text-sm px-4 py-2.5">
                                "Send test"
                            </button>
                        </form>

                        <hr class="my-2"/>
                        <h2 class="text-lg font-semibold text-gray-900 dark:text-white">"Import HTML body"</h2>
                        <p class="text-sm text-gray-600 dark:text-gray-400">"Replace the campaign HTML by pasting a URL or uploading a file (max 5 MB)."</p>
                        <form method="POST" action=format!("/app/campaigns/{token}/import-url") class="flex flex-wrap gap-3 items-end">
                            <div class="grow">
                                <label for="url" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Fetch from URL"</label>
                                <input type="url" id="url" name="url" required placeholder="https://example.com/newsletter.html"
                                    class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
                            </div>
                            <button type="submit" class="text-white bg-indigo-600 hover:bg-indigo-700 font-medium rounded-lg text-sm px-4 py-2.5">
                                "Fetch URL"
                            </button>
                        </form>
                        <form method="POST" action=format!("/app/campaigns/{token}/import-html") enctype="multipart/form-data"
                            class="flex flex-wrap gap-3 items-end">
                            <div class="grow">
                                <label for="html" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Upload HTML file"</label>
                                <input type="file" id="html" name="html" accept=".html,.htm,text/html" required
                                    class="block w-full text-sm text-gray-900 dark:text-white border border-gray-300 dark:border-gray-600 rounded-lg cursor-pointer bg-gray-50 dark:bg-gray-700"/>
                            </div>
                            <button type="submit" class="text-white bg-indigo-600 hover:bg-indigo-700 font-medium rounded-lg text-sm px-4 py-2.5">
                                "Upload"
                            </button>
                        </form>

                        <hr class="my-2"/>
                        <h2 class="text-lg font-semibold text-gray-900 dark:text-white">"Attachments"</h2>
                        <p class="text-sm text-gray-600 dark:text-gray-400">"Files attached here ride along with every send (max 10 MB each). Allowed extensions are set per brand."</p>
                        {if attachments.is_empty() {
                            view! { <p class="text-sm text-gray-400 dark:text-gray-500">"No attachments yet."</p> }.into_any()
                        } else {
                            let token2 = token.clone();
                            view! {
                                <ul class="divide-y divide-gray-200 dark:divide-gray-700 border border-gray-200 dark:border-gray-700 rounded-md">
                                    {attachments.into_iter().map(|name| {
                                        let token3 = token2.clone();
                                        view! {
                                            <li class="flex items-center justify-between px-3 py-2 text-sm">
                                                <span class="font-mono text-gray-700 dark:text-gray-300">{name.clone()}</span>
                                                <form method="POST" action=format!("/app/campaigns/{token3}/attachments/delete")
                                                    onsubmit="return confirm('Remove this attachment?');">
                                                    <input type="hidden" name="filename" value=name/>
                                                    <button type="submit" class="text-red-600 hover:underline text-xs">"Remove"</button>
                                                </form>
                                            </li>
                                        }
                                    }).collect_view()}
                                </ul>
                            }.into_any()
                        }}
                        <form method="POST" action=format!("/app/campaigns/{token}/attachments/upload") enctype="multipart/form-data"
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
                    <form method="POST" action=format!("/app/campaigns/{token}/delete?brand={brand_token}")
                        onsubmit="return confirm('Permanently delete this campaign?');" class="text-right max-w-3xl">
                        <button type="submit" class="text-red-600 hover:underline text-sm">"Delete campaign"</button>
                    </form>
                }.into_any()
            } else {
                ().into_any()
            }}
        </section>
    }
}

#[component]
pub fn CampaignReportPage(
    campaign: campaigns::Model,
    report_data: crate::services::reports::CampaignReport,
) -> impl IntoView {
    let r = report_data;
    let title = campaign.title.unwrap_or_default();
    let subject = campaign.subject.unwrap_or_default();

    // ── Donut: opens vs not-opened ──────────────────────────────────────
    let unopened = (r.recipients - r.unique_opens).max(0);
    let opens_chart_id = "chart-opens";
    let opens_chart_config = serde_json::json!({
        "chart": { "type": "donut", "height": 280 },
        "labels": ["Opened", "Not opened"],
        "series": [r.unique_opens, unopened],
        "colors": ["#1c64f2", "#e5e7eb"],
        "stroke": { "colors": ["transparent"] },
        "plotOptions": { "pie": { "donut": { "size": "70%" } } },
        "legend": { "position": "bottom" },
        "dataLabels": { "enabled": true }
    });

    // ── Donut: clicks vs no-clicks ──────────────────────────────────────
    let no_clicks = (r.recipients - r.unique_clickers).max(0);
    let clicks_chart_id = "chart-clicks";
    let clicks_chart_config = serde_json::json!({
        "chart": { "type": "donut", "height": 280 },
        "labels": ["Clicked", "No click"],
        "series": [r.unique_clickers, no_clicks],
        "colors": ["#16a34a", "#e5e7eb"],
        "stroke": { "colors": ["transparent"] },
        "plotOptions": { "pie": { "donut": { "size": "70%" } } },
        "legend": { "position": "bottom" },
        "dataLabels": { "enabled": true }
    });

    // ── Bar: top clicked links ──────────────────────────────────────────
    let links_chart_id = "chart-links";
    let categories: Vec<String> = r
        .top_links
        .iter()
        .map(|l| {
            let mut url = l.url.clone();
            if url.len() > 36 {
                url.truncate(36);
                url.push('…');
            }
            url
        })
        .collect();
    let values: Vec<i64> = r.top_links.iter().map(|l| l.clicks).collect();
    let links_chart_config = serde_json::json!({
        "chart": { "type": "bar", "height": 320, "toolbar": { "show": false } },
        "series": [{ "name": "Clicks", "data": values }],
        "xaxis": { "categories": categories },
        "plotOptions": { "bar": { "horizontal": true, "borderRadius": 4 } },
        "colors": ["#1c64f2"],
        "grid": { "show": false },
        "dataLabels": { "enabled": false }
    });

    // ── Funnel: Recipients → Opened → Clicked (drop-off at each stage)
    let funnel_chart_id = "chart-funnel";
    let funnel_chart_config = serde_json::json!({
        "chart": { "type": "bar", "height": 300, "toolbar": { "show": false } },
        "plotOptions": { "bar": {
            "horizontal": true,
            "barHeight": "60%",
            "isFunnel": true,
            "borderRadius": 4,
        }},
        "series": [{
            "name": "Recipients",
            "data": [
                { "x": "Sent",     "y": r.recipients },
                { "x": "Opened",   "y": r.unique_opens },
                { "x": "Clicked",  "y": r.unique_clickers },
            ]
        }],
        "colors": ["#2563eb"],
        "dataLabels": {
            "enabled": true,
            "formatter": "FUNCTION:function(val, opt){return opt.w.config.series[0].data[opt.dataPointIndex].x + ': ' + val;}",
            "dropShadow": { "enabled": false },
        },
        "legend": { "show": false },
        "grid": { "show": false },
        "xaxis": { "labels": { "show": false }, "axisBorder": { "show": false }, "axisTicks": { "show": false } },
        "yaxis": { "labels": { "show": false } },
    });

    // ── Bar: activity overview (Recipients/Opened/Unopened/Clicked/Unsub/Bounced/Spam)
    let activity_chart_id = "chart-activity";
    let activity_chart_config = serde_json::json!({
        "chart": { "type": "bar", "height": 360, "stacked": false, "toolbar": { "show": false } },
        "plotOptions": { "bar": { "horizontal": true, "borderRadius": 4, "barHeight": "70%" } },
        "series": [
            { "name": "Recipients",     "data": [r.recipients] },
            { "name": "Opened",         "data": [r.unique_opens] },
            { "name": "Unopened",       "data": [(r.recipients - r.unique_opens).max(0)] },
            { "name": "Clicked",        "data": [r.unique_clickers] },
            { "name": "Unsubscribed",   "data": [r.unsubscribed] },
            { "name": "Bounced",        "data": [r.bounced] },
            { "name": "Marked as spam", "data": [r.complained] },
        ],
        "xaxis": { "categories": ["Activity"] },
        "colors": ["#2563eb", "#16a34a", "#94a3b8", "#7c3aed", "#f59e0b", "#ef4444", "#a16207"],
        "dataLabels": { "enabled": true },
        "legend": { "position": "bottom" },
        "grid": { "show": false },
    });

    // ── Pie: top countries opened/clicked from
    let countries_chart_id = "chart-countries";
    let country_labels: Vec<String> = r.top_countries.iter().map(|(c, _)| c.clone()).collect();
    let country_values: Vec<i64> = r.top_countries.iter().map(|(_, n)| *n).collect();
    let countries_chart_config = serde_json::json!({
        "chart": { "type": "pie", "height": 320 },
        "labels": country_labels,
        "series": country_values,
        "legend": { "position": "bottom" },
        "dataLabels": { "enabled": true },
        "colors": ["#2563eb", "#16a34a", "#7c3aed", "#f59e0b", "#ef4444", "#0891b2", "#db2777", "#65a30d", "#0ea5e9", "#a16207", "#475569", "#9333ea"],
    });

    let opens_init = format!(
        "new ApexCharts(document.getElementById('{opens_chart_id}'), {}).render();",
        opens_chart_config
    );
    let clicks_init = format!(
        "new ApexCharts(document.getElementById('{clicks_chart_id}'), {}).render();",
        clicks_chart_config
    );
    let links_init = format!(
        "new ApexCharts(document.getElementById('{links_chart_id}'), {}).render();",
        links_chart_config
    );
    let activity_init = format!(
        "new ApexCharts(document.getElementById('{activity_chart_id}'), {}).render();",
        activity_chart_config
    );
    // Funnel needs JS-callable formatter; reverse the JSON-string back to an actual function.
    let funnel_init = format!(
        "(function(){{var cfg = {}; if (cfg.dataLabels && typeof cfg.dataLabels.formatter === 'string' && cfg.dataLabels.formatter.indexOf('FUNCTION:') === 0) {{ cfg.dataLabels.formatter = new Function('val','opt','return (' + cfg.dataLabels.formatter.slice(9) + ')(val,opt);'); }} new ApexCharts(document.getElementById('{funnel_chart_id}'), cfg).render();}})();",
        funnel_chart_config
    );
    let countries_init = format!(
        "new ApexCharts(document.getElementById('{countries_chart_id}'), {}).render();",
        countries_chart_config
    );

    let no_links = r.top_links.is_empty();
    let no_countries = r.top_countries.is_empty();

    view! {
        <section class="space-y-6">
            <header class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2">
                <div>
                    <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">{title}</h1>
                    <p class="text-sm text-gray-500 dark:text-gray-400">{subject}</p>
                </div>
                <a href=format!("/app/campaigns/{}/report.csv", OpaqueId::<Campaign>::new(campaign.id))
                    class="inline-flex items-center text-blue-700 bg-blue-50 hover:bg-blue-100 font-medium rounded-lg text-sm px-4 py-2">
                    "Download CSV"
                </a>
            </header>

            <div class="grid grid-cols-2 md:grid-cols-4 gap-5">
                <Stat label="Recipients" value=format!("{}", r.recipients)/>
                <Stat label="Queued total" value=format!("{}", r.to_send)/>
                <Stat
                    label="Open rate"
                    value=format!("{:.1}%", r.open_rate())
                    sub=format!("{} unique", r.unique_opens)
                />
                <Stat
                    label="Click rate"
                    value=format!("{:.1}%", r.click_rate())
                    sub=format!("{} unique", r.unique_clickers)
                />
                <Stat label="Total opens" value=format!("{}", r.total_opens)/>
                <Stat label="Total clicks" value=format!("{}", r.total_clicks)/>
                <Stat label="Unsubscribed" value=format!("{}", r.unsubscribed) sub=format!("{:.1}%", r.unsubscribe_rate())/>
                <Stat label="Bounced" value=format!("{}", r.bounced) sub=format!("{:.1}%", r.bounce_rate())/>
            </div>

            <div class="grid grid-cols-1 lg:grid-cols-3 gap-5">
                <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6 lg:col-span-2">
                    <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">"Activity overview"</h2>
                    <div id=activity_chart_id></div>
                </div>
                <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6">
                    <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">"Engagement funnel"</h2>
                    <div id=funnel_chart_id></div>
                </div>
            </div>

            <div class="grid grid-cols-1 lg:grid-cols-2 gap-5">
                <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6">
                    <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">"Open rate"</h2>
                    <div id=opens_chart_id></div>
                </div>
                <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6">
                    <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">"Click rate"</h2>
                    <div id=clicks_chart_id></div>
                </div>
            </div>

            <div class="grid grid-cols-1 lg:grid-cols-3 gap-5">
                <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6 lg:col-span-2">
                    <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">"Top clicked links"</h2>
                    {if no_links {
                        view! { <p class="text-sm text-gray-500 dark:text-gray-400 py-8 text-center">"No clicks recorded yet."</p> }.into_any()
                    } else {
                        view! { <div id=links_chart_id></div> }.into_any()
                    }}
                </div>
                <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6">
                    <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">"Top countries"</h2>
                    {if no_countries {
                        view! { <p class="text-sm text-gray-500 dark:text-gray-400 py-8 text-center">"No country data."</p> }.into_any()
                    } else {
                        view! { <div id=countries_chart_id></div> }.into_any()
                    }}
                </div>
            </div>

            <RecipientList title="Unsubscribed from this campaign" rows=r.unsubscribed_subs/>
            <RecipientList title="Bounced" rows=r.bounced_subs/>
            <RecipientList title="Marked as spam" rows=r.complained_subs/>

            <script>{activity_init}</script>
            <script>{funnel_init}</script>
            <script>{opens_init}</script>
            <script>{clicks_init}</script>
            {if !no_links { Some(view! { <script>{links_init}</script> }) } else { None }}
            {if !no_countries { Some(view! { <script>{countries_init}</script> }) } else { None }}
        </section>
    }
}

#[component]
fn RecipientList(
    #[prop(into)] title: String,
    rows: Vec<crate::services::reports::RecipientRow>,
) -> impl IntoView {
    if rows.is_empty() {
        return view! {}.into_any();
    }
    view! {
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm overflow-x-auto">
            <h2 class="text-lg font-semibold text-gray-900 dark:text-white px-6 pt-6">{title}</h2>
            <table class="w-full text-sm text-left text-gray-500 dark:text-gray-400 mt-2">
                <thead class="text-xs text-gray-700 dark:text-gray-300 uppercase bg-gray-50 dark:bg-gray-700">
                    <tr>
                        <th class="px-4 py-3">"Email"</th>
                        <th class="px-4 py-3">"Name"</th>
                    </tr>
                </thead>
                <tbody>
                    {rows.into_iter().map(|r| view! {
                        <tr class="bg-white dark:bg-gray-800 border-b">
                            <td class="px-4 py-3 font-medium text-gray-900 dark:text-white">{r.email}</td>
                            <td class="px-4 py-3">{r.name.unwrap_or_default()}</td>
                        </tr>
                    }).collect_view()}
                </tbody>
            </table>
        </div>
    }.into_any()
}

#[component]
fn Stat(
    #[prop(into)] label: String,
    #[prop(into)] value: String,
    #[prop(into, optional)] sub: String,
) -> impl IntoView {
    view! {
        <div class="bg-white dark:bg-gray-800 rounded-lg p-5 shadow-sm border border-gray-100">
            <div class="text-xs uppercase tracking-wider text-gray-500 dark:text-gray-400">{label}</div>
            <div class="mt-1 text-2xl font-bold text-gray-900 dark:text-white">{value}</div>
            {if sub.is_empty() { None } else { Some(view! { <div class="text-xs text-gray-500 dark:text-gray-400">{sub}</div> }) }}
        </div>
    }
}

#[component]
fn Field(
    #[prop(into)] id: String,
    #[prop(into)] label: String,
    #[prop(into, default = "text".to_string())] input_type: String,
    #[prop(into, optional)] value: String,
    #[prop(into, optional)] placeholder: String,
    #[prop(default = false)] required: bool,
) -> impl IntoView {
    let id_for = id.clone();
    view! {
        <div>
            <label for=id_for class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">{label}</label>
            <input type=input_type id=id.clone() name=id value=value placeholder=placeholder required=required
                class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
        </div>
    }
}

#[component]
fn Textarea(
    #[prop(into)] id: String,
    #[prop(into)] label: String,
    #[prop(into, optional)] value: String,
    rows: u32,
    #[prop(default = false)] wysiwyg: bool,
) -> impl IntoView {
    let id_for = id.clone();
    let class = if wysiwyg {
        "wysiwyg bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5 font-mono"
    } else {
        "bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5 font-mono"
    };
    view! {
        <div>
            <label for=id_for class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">{label}</label>
            <textarea id=id.clone() name=id rows=rows.to_string() class=class>
                {value}
            </textarea>
        </div>
    }
}

// ── Forms / payloads ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CampaignForm {
    pub title: String,
    pub subject: String,
    #[serde(default)]
    pub from_name: Option<String>,
    #[serde(default)]
    pub from_email: Option<String>,
    #[serde(default)]
    pub reply_to: Option<String>,
    #[serde(default)]
    pub html_text: Option<String>,
    #[serde(default)]
    pub plain_text: Option<String>,
    #[serde(default)]
    pub query_string: Option<String>,
    /// Checkbox — if present, also save the body as a reusable template
    /// under this brand.
    #[serde(default)]
    pub save_as_template: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SendForm {
    pub list_ids: String,
}

#[derive(Debug, Deserialize)]
pub struct PageQuery {
    #[serde(default)]
    pub page: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct BrandQuery {
    pub brand: String,
}

pub(crate) fn nonempty(s: Option<String>) -> Option<String> {
    s.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

fn blank_campaign() -> campaigns::Model {
    campaigns::Model {
        id: 0, user_id: None, app: None,
        from_name: None, from_email: None, reply_to: None,
        title: None, label: None, subject: None,
        plain_text: None, html_text: None,
        query_string: None, sent: None, to_send: Some(0), to_send_lists: None,
        recipients: Some(0), opens: None, errors: None, wysiwyg: Some(1),
        timeout_check: None, lists: None, lists_excl: None, segs: None, segs_excl: None,
        timezone: None, send_date: None,
        bounce_setup: Some(0), complaint_setup: Some(0),
        opens_tracking: Some(1), links_tracking: Some(1), quota_deducted: Some(0),
        web_version_lang: None, campaign_stopped: Some(0), ignore_checks: Some(0),
    }
}

// ── Handlers ────────────────────────────────────────────────────────────────

