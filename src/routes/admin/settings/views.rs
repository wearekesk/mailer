//! Settings, suppression list and blocked-domain admin pages.

use leptos::prelude::*;
use serde::Deserialize;

use crate::services::settings::{
    BlockedDomainEntry, ProfileInput, SuppressionEntry,
};

pub(crate) const PAGE_SIZE: u64 = 50;

// ── Profile / general settings ──────────────────────────────────────────────

#[component]
pub fn SettingsPage(
    user: crate::database::entities::login::Model,
    #[prop(optional)] info: Option<String>,
) -> impl IntoView {
    view! {
        <section class="space-y-6">
            <header>
                <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">"Settings"</h1>
                <p class="text-sm text-gray-500 dark:text-gray-400">"Personal account settings, integrations, and credentials."</p>
            </header>
            {info.map(|m| view! {
                <div class="p-3 text-sm text-green-800 rounded-lg bg-green-50 border border-green-200">{m}</div>
            })}
            // Flowbite Tabbed Settings Navigation
            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm overflow-hidden">
                <div class="border-b border-gray-200 dark:border-gray-700">
                    <ul class="flex flex-wrap -mb-px text-sm font-medium text-center" id="settings-tabs" role="tablist" data-tabs-toggle="#settings-tab-content" data-tabs-active-classes="text-primary-600 dark:text-primary-300 border-primary-600 dark:border-primary-300" data-tabs-inactive-classes="text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300 border-transparent hover:border-gray-300 dark:hover:border-gray-600">
                        <li class="me-2" role="presentation">
                            <button class="inline-flex items-center gap-2 p-4 border-b-2 rounded-t-lg" id="settings-account-tab" data-tabs-target="#settings-account-panel" type="button" role="tab" aria-controls="settings-account-panel" aria-selected="true">
                                <i class="fa-solid fa-user-gear"></i>"Account & Profile"
                            </button>
                        </li>
                        <li class="me-2" role="presentation">
                            <button class="inline-flex items-center gap-2 p-4 border-b-2 rounded-t-lg" id="settings-aws-tab" data-tabs-target="#settings-aws-panel" type="button" role="tab" aria-controls="settings-aws-panel" aria-selected="false">
                                <i class="fa-brands fa-aws"></i>"AWS & SES"
                            </button>
                        </li>
                        <li class="me-2" role="presentation">
                            <button class="inline-flex items-center gap-2 p-4 border-b-2 rounded-t-lg" id="settings-security-tab" data-tabs-target="#settings-security-panel" type="button" role="tab" aria-controls="settings-security-panel" aria-selected="false">
                                <i class="fa-solid fa-key"></i>"Security & API"
                            </button>
                        </li>
                        <li class="me-2" role="presentation">
                            <button class="inline-flex items-center gap-2 p-4 border-b-2 rounded-t-lg" id="settings-system-tab" data-tabs-target="#settings-system-panel" type="button" role="tab" aria-controls="settings-system-panel" aria-selected="false">
                                <i class="fa-solid fa-clock-rotate-left"></i>"System & Scheduler"
                            </button>
                        </li>
                    </ul>
                </div>

                <div id="settings-tab-content" class="p-6">
                    // Tab 1: Account & Profile
                    <div id="settings-account-panel" role="tabpanel" aria-labelledby="settings-account-tab">
                        <form method="POST" action="/app/settings" class="space-y-6">
                            <input type="hidden" name="aws_key" value=user.s3_key.clone().unwrap_or_default()/>
                            <input type="hidden" name="aws_secret" value=user.s3_secret.clone().unwrap_or_default()/>
                            <input type="hidden" name="ses_endpoint" value=user.ses_endpoint.clone().unwrap_or_default()/>
                            <input type="hidden" name="send_rate" value=user.send_rate.map(|v| v.to_string()).unwrap_or_default()/>
                            <div class="grid grid-cols-1 md:grid-cols-2 gap-5">
                                <Input id="personal_name" label="Your name" value=user.name.clone().unwrap_or_default()/>
                                <Input id="company" label="Company" value=user.company.clone().unwrap_or_default()/>
                                <Input id="email" label="Login email" input_type="email" required=true value=user.username.clone().unwrap_or_default()/>
                                <Input id="password" label="New password (leave blank to keep)" input_type="password" value=String::new()/>
                                <Input id="timezone" label="Timezone" value=user.timezone.clone().unwrap_or_default() placeholder="UTC"/>
                                <Input id="language" label="Language" value=user.language.clone().unwrap_or_default() placeholder="en"/>
                            </div>

                            <fieldset class="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
                                <legend class="text-sm font-semibold px-2 text-gray-700 dark:text-gray-300">"Preferences"</legend>
                                <div class="grid grid-cols-1 md:grid-cols-2 gap-5">
                                    <Input id="brands_rows" label="Brands per page (table pagination)" input_type="number"
                                        value=user.brands_rows.map(|v| v.to_string()).unwrap_or_else(|| "20".into())/>
                                    <Input id="paypal" label="PayPal email (used by sub-user billing — optional)" input_type="email"
                                        value=user.paypal.clone().unwrap_or_default()/>
                                </div>
                            </fieldset>

                            <div class="flex justify-between items-center pt-2">
                                <label class="inline-flex items-center cursor-pointer">
                                    <input type="checkbox" id="strict_delete" name="strict_delete" value="1" class="sr-only peer"
                                        checked=user.strict_delete.unwrap_or(0) == 1/>
                                    <div class="relative w-11 h-6 bg-gray-200 peer-focus:ring-4 peer-focus:ring-blue-300 rounded-full peer peer-checked:bg-blue-600 after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white dark:bg-gray-800 after:border-gray-300 dark:border-gray-600 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:after:translate-x-full"></div>
                                    <span class="ms-3 text-sm font-medium text-gray-900 dark:text-white">"Strict delete (requires confirmation)"</span>
                                </label>

                                <button type="submit"
                                    class="text-white bg-blue-600 hover:bg-blue-700 font-medium rounded-lg text-sm px-5 py-2.5">
                                    "Save changes"
                                </button>
                            </div>
                        </form>
                    </div>

                    // Tab 2: AWS & SES
                    <div class="hidden space-y-6" id="settings-aws-panel" role="tabpanel" aria-labelledby="settings-aws-tab">
                        <form method="POST" action="/app/settings" class="space-y-6">
                            <input type="hidden" name="personal_name" value=user.name.clone().unwrap_or_default()/>
                            <input type="hidden" name="company" value=user.company.clone().unwrap_or_default()/>
                            <input type="hidden" name="email" value=user.username.clone().unwrap_or_default()/>
                            <input type="hidden" name="timezone" value=user.timezone.clone().unwrap_or_default()/>
                            <input type="hidden" name="language" value=user.language.clone().unwrap_or_default()/>
                            <input type="hidden" name="brands_rows" value=user.brands_rows.map(|v| v.to_string()).unwrap_or_else(|| "20".into())/>
                            <input type="hidden" name="paypal" value=user.paypal.clone().unwrap_or_default()/>
                            <input type="hidden" name="strict_delete" value=if user.strict_delete.unwrap_or(0) == 1 { "1" } else { "0" }/>

                            <fieldset class="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
                                <legend class="text-sm font-semibold px-2 text-gray-700 dark:text-gray-300">"AWS credentials & send limits"</legend>
                                <div class="grid grid-cols-1 md:grid-cols-2 gap-5">
                                    <Input id="aws_key" label="Access key" value=user.s3_key.clone().unwrap_or_default()/>
                                    <Input id="aws_secret" label="Secret key" input_type="password" value=user.s3_secret.clone().unwrap_or_default()/>
                                    <Input id="ses_endpoint" label="SES endpoint" value=user.ses_endpoint.clone().unwrap_or_default() placeholder="email.us-east-1.amazonaws.com"/>
                                    <Input id="send_rate" label="Send rate (per second)" input_type="number"
                                        value=user.send_rate.map(|v| v.to_string()).unwrap_or_default()/>
                                </div>
                            </fieldset>

                            <div class="flex justify-end">
                                <button type="submit"
                                    class="text-white bg-blue-600 hover:bg-blue-700 font-medium rounded-lg text-sm px-5 py-2.5">
                                    "Save AWS settings"
                                </button>
                            </div>
                        </form>

                        <SesCard
                            login_email=user.username.clone().unwrap_or_default()
                            aws_keys_set=user.s3_key.clone().filter(|s| !s.is_empty()).is_some()
                        />
                    </div>

                    // Tab 3: Security & API
                    <div class="hidden space-y-6" id="settings-security-panel" role="tabpanel" aria-labelledby="settings-security-tab">
                        <TwoFactorCard enabled=user.auth_enabled.unwrap_or(0) == 1/>
                        <ApiKeyCard api_key=user.api_key.clone().unwrap_or_default()/>
                    </div>

                    // Tab 4: System / Scheduler
                    <div class="hidden space-y-6" id="settings-system-panel" role="tabpanel" aria-labelledby="settings-system-tab">
                        <SchedulerCard cron_locked=user.cron.unwrap_or(0) == 1/>
                    </div>
                </div>
            </div>
        </section>
    }
}

#[component]
fn SchedulerCard(cron_locked: bool) -> impl IntoView {
    view! {
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6">
            <div class="flex items-center justify-between gap-4 flex-wrap">
                <div>
                    <h2 class="text-lg font-semibold text-gray-900 dark:text-white">"Background scheduler"</h2>
                    <p class="text-sm text-gray-500 dark:text-gray-400">
                        {if cron_locked {
                            "A scheduler tick is currently in flight (login.cron = 1). If a previous tick crashed it may be wedged — clear the lock to let the next tick run."
                        } else {
                            "Lock is free. The scheduler runs every 30s and acquires this lock automatically."
                        }}
                    </p>
                </div>
                <crate::components::layout::ConfirmDialog
                    id="reset-cron-modal".to_string()
                    action="/app/settings/scheduler/reset-cron-lock".to_string()
                    trigger_label="Reset cron lock".to_string()
                    message="Force-clear login.cron. Only do this if a tick is genuinely stuck — clearing while a tick is actually running can cause double-sends.".to_string()
                    confirm_label="Yes, reset the lock".to_string()
                    trigger_class="text-sm font-medium text-white bg-amber-600 hover:bg-amber-700 rounded-lg px-3 py-2".to_string()
                />
            </div>
        </div>
    }
}

#[component]
fn SesCard(
    #[prop(into)] login_email: String,
    aws_keys_set: bool,
) -> impl IntoView {
    view! {
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6">
            <div class="flex items-center justify-between gap-4 flex-wrap">
                <div>
                    <h2 class="text-lg font-semibold text-gray-900 dark:text-white">"Amazon SES — login email verification"</h2>
                    <p class="text-sm text-gray-500 dark:text-gray-400">{format!("Verify {login_email} with AWS SES so password-reset and admin notifications can be sent through SES.")}</p>
                </div>
                {if aws_keys_set {
                    view! {
                        <form method="POST" action="/app/settings/ses/verify-login-email">
                            <button type="submit" class="text-white bg-blue-600 hover:bg-blue-700 font-medium rounded-lg text-sm px-4 py-2">
                                "Send SES verification email"
                            </button>
                        </form>
                    }.into_any()
                } else {
                    view! {
                        <p class="text-sm text-amber-700">"Add AWS access + secret keys above first."</p>
                    }.into_any()
                }}
            </div>
        </div>
    }
}

#[component]
fn TwoFactorCard(enabled: bool) -> impl IntoView {
    view! {
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6">
            <div class="flex items-center justify-between gap-4">
                <div>
                    <h2 class="text-lg font-semibold text-gray-900 dark:text-white">"Two-factor authentication"</h2>
                    {if enabled {
                        view! { <p class="text-sm text-green-700">"Enabled. Disable below if you've lost access to your authenticator."</p> }.into_any()
                    } else {
                        view! { <p class="text-sm text-gray-500 dark:text-gray-400">"Add a second factor to require a code at sign-in."</p> }.into_any()
                    }}
                </div>
                {if enabled {
                    view! {
                        <form method="POST" action="/app/settings/two-factor/disable"
                            onsubmit="return confirm('Disable two-factor authentication?');">
                            <button type="submit" class="text-white bg-red-600 hover:bg-red-700 font-medium rounded-lg text-sm px-4 py-2">
                                "Disable"
                            </button>
                        </form>
                    }.into_any()
                } else {
                    view! {
                        <a href="/app/settings/two-factor/setup"
                            class="text-white bg-blue-600 hover:bg-blue-700 font-medium rounded-lg text-sm px-4 py-2">
                            "Set up"
                        </a>
                    }.into_any()
                }}
            </div>
        </div>
    }
}

#[component]
pub fn TwoFactorSetupPage(
    qr_svg: String,
    secret: String,
    #[prop(optional)] error: Option<String>,
) -> impl IntoView {
    view! {
        <section class="space-y-6">
            <header>
                <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">"Two-factor authentication"</h1>
                <p class="text-sm text-gray-500 dark:text-gray-400">"Scan the QR code with Google Authenticator, 1Password, Authy, or similar — then enter the 6-digit code below to confirm."</p>
            </header>
            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6 grid grid-cols-1 md:grid-cols-2 gap-6 items-start">
                <div class="flex flex-col items-center gap-3">
                    <div inner_html=qr_svg></div>
                    <p class="text-xs text-gray-500 dark:text-gray-400 break-all text-center">{format!("Secret: {secret}")}</p>
                </div>
                <form method="POST" action="/app/settings/two-factor/confirm" class="space-y-4">
                    {error.map(|m| view! {
                        <div class="p-3 text-sm text-red-800 rounded-lg bg-red-50 border border-red-200" role="alert">{m}</div>
                    })}
                    <div>
                        <label for="otp_code" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"6-digit code"</label>
                        <input type="text" id="otp_code" name="otp_code" inputmode="numeric"
                            pattern="[0-9]{6}" maxlength="6" required placeholder="000000"
                            class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5 tracking-[0.5em] text-center font-mono"/>
                    </div>
                    <button type="submit" class="text-white bg-blue-600 hover:bg-blue-700 font-medium rounded-lg text-sm px-5 py-2.5">
                        "Confirm and enable"
                    </button>
                    <a href="/app/settings" class="block text-center text-sm text-gray-600 dark:text-gray-400 hover:underline">"Cancel"</a>
                </form>
            </div>
        </section>
    }
}

#[component]
fn ApiKeyCard(#[prop(into)] api_key: String) -> impl IntoView {
    let masked = if api_key.is_empty() {
        "—".to_string()
    } else {
        let len = api_key.len();
        format!("{}…{}", &api_key[..api_key.len().min(4)], &api_key[len.saturating_sub(4)..])
    };
    view! {
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6">
            <div class="flex items-center justify-between gap-4">
                <div>
                    <h2 class="text-lg font-semibold text-gray-900 dark:text-white">"API key"</h2>
                    <p class="text-sm text-gray-500 dark:text-gray-400">{format!("Current: {masked}")}</p>
                </div>
                <form method="POST" action="/app/settings/regenerate-api-key"
                    onsubmit="return confirm('Regenerate the API key? Existing clients will stop working until they pick up the new key.');">
                    <button type="submit"
                        class="text-white bg-amber-600 hover:bg-amber-700 font-medium rounded-lg text-sm px-4 py-2">
                        "Regenerate key"
                    </button>
                </form>
            </div>
        </div>
    }
}

#[component]
fn Input(
    #[prop(into)] id: String,
    #[prop(into)] label: String,
    #[prop(into, optional)] placeholder: String,
    #[prop(into, default = "text".to_string())] input_type: String,
    #[prop(into, optional)] value: String,
    #[prop(default = false)] required: bool,
) -> impl IntoView {
    let id_for = id.clone();
    view! {
        <div>
            <label for=id_for class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">{label}</label>
            <input type=input_type id=id.clone() name=id placeholder=placeholder value=value required=required
                class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
        </div>
    }
}

// ── Suppression list page ───────────────────────────────────────────────────

#[component]
pub fn SuppressionListPage(
    brand_token: String,
    brand_name: String,
    rows: Vec<SuppressionEntry>,
    page: u64,
    total: u64,
) -> impl IntoView {
    let last_page = if total == 0 { 0 } else { (total.saturating_sub(1)) / PAGE_SIZE };
    let csv_url = format!("/app/brands/{brand_token}/suppression-list/export.csv");
    let clear_url = format!("/app/brands/{brand_token}/suppression-list/clear");
    let total_for_clear = total;
    view! {
        <section class="space-y-4">
            <header class="flex justify-between items-end gap-3 flex-wrap">
                <div>
                    <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">"Suppression list"</h1>
                    <p class="text-sm text-gray-500 dark:text-gray-400">{format!("Brand: {brand_name} — {total} suppressed addresses")}</p>
                </div>
                <div class="flex items-center gap-2">
                    <a href=csv_url
                        class="text-sm font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 rounded-lg px-3 py-2">
                        "Export CSV"
                    </a>
                    {if total_for_clear == 0 {
                        ().into_any()
                    } else {
                        view! {
                            <crate::components::layout::ConfirmDialog
                                id="clear-suppression-modal".to_string()
                                action=clear_url
                                trigger_label=format!("Clear all ({total_for_clear})")
                                message="Permanently delete every suppression entry for this brand. This cannot be undone.".to_string()
                                confirm_label="Yes, clear them all".to_string()
                                trigger_class="text-sm font-medium text-white bg-red-600 hover:bg-red-700 rounded-lg px-3 py-2".to_string()
                            />
                        }.into_any()
                    }}
                </div>
            </header>
            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6 space-y-4">
                <form method="POST" action=format!("/app/brands/{brand_token}/suppression-list/add") class="flex gap-3 items-end">
                    <div class="grow">
                        <label for="email" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Add email to suppression list"</label>
                        <input type="email" id="email" name="email" required
                            class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
                    </div>
                    <button type="submit"
                        class="text-white bg-blue-600 hover:bg-blue-700 font-medium rounded-lg text-sm px-4 py-2.5">"Suppress"</button>
                </form>
                <form method="POST" action=format!("/app/brands/{brand_token}/suppression-list/import") enctype="multipart/form-data" class="flex flex-wrap gap-3 items-end pt-4 border-t border-gray-200 dark:border-gray-700">
                    <div class="grow">
                        <label for="csv" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Import CSV"</label>
                        <input type="file" id="csv" name="csv" accept=".csv,text/csv" required
                            class="block w-full text-sm text-gray-900 dark:text-white border border-gray-300 dark:border-gray-600 rounded-lg cursor-pointer bg-gray-50 dark:bg-gray-700"/>
                    </div>
                    <button type="submit"
                        class="text-white bg-blue-600 hover:bg-blue-700 font-medium rounded-lg text-sm px-4 py-2.5">"Import"</button>
                </form>
            </div>
            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm overflow-x-auto">
                <table class="w-full text-sm text-left text-gray-500 dark:text-gray-400">
                    <thead class="text-xs text-gray-700 dark:text-gray-300 uppercase bg-gray-50 dark:bg-gray-700">
                        <tr>
                            <th class="px-4 py-3">"Email"</th>
                            <th class="px-4 py-3 text-right">"Block attempts"</th>
                            <th class="px-4 py-3">"Added"</th>
                            <th class="px-4 py-3 w-1"></th>
                        </tr>
                    </thead>
                    <tbody>
                        {if rows.is_empty() {
                            view! { <tr><td colspan="4" class="text-center text-gray-400 dark:text-gray-500 py-10">"Nothing suppressed."</td></tr> }.into_any()
                        } else {
                            rows.into_iter().map(|r| {
                                let action = format!("/app/suppressions/{}/delete?brand={}", r.id, brand_token);
                                let added_label = chrono::DateTime::<chrono::Utc>::from_timestamp(r.timestamp, 0)
                                    .map(|d| d.format("%Y-%m-%d").to_string())
                                    .unwrap_or_default();
                                view! {
                                    <tr class="bg-white dark:bg-gray-800 border-b">
                                        <td class="px-4 py-3 font-medium text-gray-900 dark:text-white">{r.email}</td>
                                        <td class="px-4 py-3 text-right">{r.block_attempts}</td>
                                        <td class="px-4 py-3 text-xs text-gray-500 dark:text-gray-400">{added_label}</td>
                                        <td class="px-4 py-3 text-right">
                                            <form method="POST" action=action>
                                                <button type="submit" class="text-red-600 hover:underline text-sm">"Remove"</button>
                                            </form>
                                        </td>
                                    </tr>
                                }
                            }).collect_view().into_any()
                        }}
                    </tbody>
                </table>
            </div>
            <Pagination brand_token=brand_token suffix="/suppression-list" page=page last=last_page/>
        </section>
    }
}

#[component]
pub fn BlockedDomainsPage(
    brand_token: String,
    brand_name: String,
    rows: Vec<BlockedDomainEntry>,
    page: u64,
    total: u64,
) -> impl IntoView {
    let last_page = if total == 0 { 0 } else { (total.saturating_sub(1)) / PAGE_SIZE };
    let csv_url = format!("/app/brands/{brand_token}/blocked-domains/export.csv");
    let clear_url = format!("/app/brands/{brand_token}/blocked-domains/clear");
    let total_for_clear = total;
    view! {
        <section class="space-y-4">
            <header class="flex justify-between items-end gap-3 flex-wrap">
                <div>
                    <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">"Blocked domains"</h1>
                    <p class="text-sm text-gray-500 dark:text-gray-400">{format!("Brand: {brand_name} — {total} blocked domains")}</p>
                </div>
                <div class="flex items-center gap-2">
                    <a href=csv_url
                        class="text-sm font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 rounded-lg px-3 py-2">
                        "Export CSV"
                    </a>
                    {if total_for_clear == 0 {
                        ().into_any()
                    } else {
                        view! {
                            <crate::components::layout::ConfirmDialog
                                id="clear-blocked-domains-modal".to_string()
                                action=clear_url
                                trigger_label=format!("Clear all ({total_for_clear})")
                                message="Permanently delete every blocked-domain entry for this brand. This cannot be undone.".to_string()
                                confirm_label="Yes, clear them all".to_string()
                                trigger_class="text-sm font-medium text-white bg-red-600 hover:bg-red-700 rounded-lg px-3 py-2".to_string()
                            />
                        }.into_any()
                    }}
                </div>
            </header>
            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6 space-y-4">
                <form method="POST" action=format!("/app/brands/{brand_token}/blocked-domains/add") class="flex gap-3 items-end">
                    <div class="grow">
                        <label for="domain" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Block domain"</label>
                        <input type="text" id="domain" name="domain" placeholder="example.com" required
                            class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
                    </div>
                    <button type="submit"
                        class="text-white bg-blue-600 hover:bg-blue-700 font-medium rounded-lg text-sm px-4 py-2.5">"Block"</button>
                </form>
                <form method="POST" action=format!("/app/brands/{brand_token}/blocked-domains/import") enctype="multipart/form-data" class="flex flex-wrap gap-3 items-end pt-4 border-t border-gray-200 dark:border-gray-700">
                    <div class="grow">
                        <label for="csv" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Import CSV"</label>
                        <input type="file" id="csv" name="csv" accept=".csv,text/csv" required
                            class="block w-full text-sm text-gray-900 dark:text-white border border-gray-300 dark:border-gray-600 rounded-lg cursor-pointer bg-gray-50 dark:bg-gray-700"/>
                    </div>
                    <button type="submit"
                        class="text-white bg-blue-600 hover:bg-blue-700 font-medium rounded-lg text-sm px-4 py-2.5">"Import"</button>
                </form>
            </div>
            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm overflow-x-auto">
                <table class="w-full text-sm text-left text-gray-500 dark:text-gray-400">
                    <thead class="text-xs text-gray-700 dark:text-gray-300 uppercase bg-gray-50 dark:bg-gray-700">
                        <tr>
                            <th class="px-4 py-3">"Domain"</th>
                            <th class="px-4 py-3 text-right">"Block attempts"</th>
                            <th class="px-4 py-3">"Added"</th>
                            <th class="px-4 py-3 w-1"></th>
                        </tr>
                    </thead>
                    <tbody>
                        {if rows.is_empty() {
                            view! { <tr><td colspan="4" class="text-center text-gray-400 dark:text-gray-500 py-10">"No blocked domains."</td></tr> }.into_any()
                        } else {
                            rows.into_iter().map(|r| {
                                let action = format!("/app/blocked-domains/{}/delete?brand={}", r.id, brand_token);
                                let added_label = chrono::DateTime::<chrono::Utc>::from_timestamp(r.timestamp, 0)
                                    .map(|d| d.format("%Y-%m-%d").to_string())
                                    .unwrap_or_default();
                                view! {
                                    <tr class="bg-white dark:bg-gray-800 border-b">
                                        <td class="px-4 py-3 font-medium text-gray-900 dark:text-white">{r.domain}</td>
                                        <td class="px-4 py-3 text-right">{r.block_attempts}</td>
                                        <td class="px-4 py-3 text-xs text-gray-500 dark:text-gray-400">{added_label}</td>
                                        <td class="px-4 py-3 text-right">
                                            <form method="POST" action=action>
                                                <button type="submit" class="text-red-600 hover:underline text-sm">"Remove"</button>
                                            </form>
                                        </td>
                                    </tr>
                                }
                            }).collect_view().into_any()
                        }}
                    </tbody>
                </table>
            </div>
            <Pagination brand_token=brand_token suffix="/blocked-domains" page=page last=last_page/>
        </section>
    }
}

#[component]
fn Pagination(
    #[prop(into)] brand_token: String,
    #[prop(into)] suffix: String,
    page: u64,
    last: u64,
) -> impl IntoView {
    let prev = if page > 0 {
        Some(format!("/app/brands/{brand_token}{suffix}?page={}", page - 1))
    } else { None };
    let next = if page < last {
        Some(format!("/app/brands/{brand_token}{suffix}?page={}", page + 1))
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

// ── Form payloads ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ProfileForm {
    #[serde(default)]
    pub personal_name: Option<String>,
    #[serde(default)]
    pub company: Option<String>,
    pub email: String,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub timezone: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub send_rate: Option<String>,
    #[serde(default)]
    pub ses_endpoint: Option<String>,
    #[serde(default)]
    pub aws_key: Option<String>,
    #[serde(default)]
    pub aws_secret: Option<String>,
    #[serde(default)]
    pub paypal: Option<String>,
    #[serde(default)]
    pub brands_rows: Option<String>,
    #[serde(default)]
    pub strict_delete: Option<String>,
    #[serde(default)]
    pub dark_mode: Option<String>,
}

impl From<ProfileForm> for ProfileInput {
    fn from(f: ProfileForm) -> Self {
        ProfileInput {
            name: nonempty(f.personal_name),
            company: nonempty(f.company),
            username: f.email.trim().to_string(),
            timezone: nonempty(f.timezone),
            language: nonempty(f.language),
            send_rate: f.send_rate.as_deref().and_then(|s| s.parse().ok()),
            ses_endpoint: nonempty(f.ses_endpoint),
            s3_key: nonempty(f.aws_key),
            s3_secret: nonempty(f.aws_secret),
            paypal: nonempty(f.paypal),
            brands_rows: f.brands_rows.as_deref().and_then(|s| s.parse().ok()),
            strict_delete: if f.strict_delete.is_some() { 1 } else { 0 },
            dark_mode: if f.dark_mode.is_some() { 1 } else { 0 },
            new_password: nonempty(f.password),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct EmailForm {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct DomainForm {
    pub domain: String,
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

// ── Handlers ────────────────────────────────────────────────────────────────

