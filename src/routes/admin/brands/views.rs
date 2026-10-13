//! Brand admin pages and form-action handlers.
//!
//! UI uses Flowbite blocks: a CRUD table block on the index, a form block
//! on new/edit, and a confirmation modal on delete.

use leptos::prelude::*;
use serde::Deserialize;

use crate::services::brands::{BrandInput, BrandSummary};
use crate::utils::opaque_id::{Brand, OpaqueId};

// ── Page components ─────────────────────────────────────────────────────────

/// `GET /app/brands` — listing table.
#[component]
pub fn BrandsListPage(brands: Vec<BrandSummary>) -> impl IntoView {
    view! {
        <section class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6">
            <header class="flex flex-col sm:flex-row justify-between items-start sm:items-center mb-6 gap-3">
                <div>
                    <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">"Brands"</h1>
                    <p class="text-sm text-gray-500 dark:text-gray-400 mt-1">"Each brand has its own SMTP, sender identity, and quota."</p>
                </div>
                <a
                    href="/app/brands/new"
                    class="inline-flex items-center text-white bg-blue-600 hover:bg-blue-700 focus:ring-4 focus:outline-none focus:ring-blue-300 font-medium rounded-lg text-sm px-4 py-2"
                >
                    <svg class="w-4 h-4 me-2" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                        <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 12h14m-7 7V5"/>
                    </svg>
                    "New brand"
                </a>
            </header>

            <div class="relative overflow-x-auto sm:rounded-lg">
                <table class="w-full text-sm text-left text-gray-500 dark:text-gray-400">
                    <thead class="text-xs text-gray-700 dark:text-gray-300 uppercase bg-gray-50 dark:bg-gray-700">
                        <tr>
                            <th scope="col" class="px-4 py-3">"Name"</th>
                            <th scope="col" class="px-4 py-3">"From email"</th>
                            <th scope="col" class="px-4 py-3 text-right">"Lists"</th>
                            <th scope="col" class="px-4 py-3 text-right">"Subscribers"</th>
                            <th scope="col" class="px-4 py-3 w-1"><span class="sr-only">"Actions"</span></th>
                        </tr>
                    </thead>
                    <tbody>
                        {if brands.is_empty() {
                            view! {
                                <tr>
                                    <td colspan="5" class="text-center text-gray-400 dark:text-gray-500 py-10">"No brands yet."</td>
                                </tr>
                            }.into_any()
                        } else {
                            brands.into_iter().map(|b| {
                                let token = OpaqueId::<Brand>::new(b.id).to_string();
                                let edit_href = format!("/app/brands/{}/edit", token);
                                let lists_href = format!("/app/brands/{}/lists", token);
                                let logo_url = b.logo_filename.map(|f| format!("/uploads/logos/{f}"));
                                view! {
                                    <tr class="bg-white dark:bg-gray-800 border-b hover:bg-gray-50 dark:hover:bg-gray-600">
                                        <th scope="row" class="px-4 py-3 font-medium text-gray-900 dark:text-white whitespace-nowrap">
                                            <div class="flex items-center gap-3">
                                                {logo_url.map(|u| view! {
                                                    <img src=u alt="" class="h-8 w-8 rounded object-cover border border-gray-200 dark:border-gray-700"/>
                                                })}
                                                <a href=lists_href.clone() class="hover:underline">{b.name}</a>
                                            </div>
                                        </th>
                                        <td class="px-4 py-3">{b.from_email.unwrap_or_default()}</td>
                                        <td class="px-4 py-3 text-right">{b.list_count}</td>
                                        <td class="px-4 py-3 text-right">{b.subscriber_count}</td>
                                        <td class="px-4 py-3 text-right">
                                            <a href=edit_href class="font-medium text-blue-600 hover:underline">"Edit"</a>
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

/// `GET /app/brands/new` and `GET /app/brands/:id/edit` — same form, two
/// modes. Pass `existing = Some(model)` to render an edit form.
#[component]
pub fn BrandFormPage(
    #[prop(optional)] existing: Option<crate::database::entities::apps::Model>,
    #[prop(optional)] error: Option<String>,
    #[prop(default = Vec::new())] sub_users: Vec<crate::services::auth::SubUserRow>,
    #[prop(default = None)] ses_status: Option<crate::services::ses::VerificationStatus>,
) -> impl IntoView {
    let is_edit = existing.is_some();
    let action = match &existing {
        Some(m) => format!("/app/brands/{}/edit", OpaqueId::<Brand>::new(m.id)),
        None => "/app/brands/new".to_string(),
    };
    let title = if is_edit { "Edit brand" } else { "New brand" };
    let m = existing.unwrap_or_else(blank_brand);

    view! {
        <section class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6 max-w-3xl">
            <h1 class="text-2xl font-semibold text-gray-900 dark:text-white mb-6">{title}</h1>
            {error.map(|m| view! {
                <div class="p-3 mb-4 text-sm text-red-800 rounded-lg bg-red-50 border border-red-200" role="alert">
                    {m}
                </div>
            })}
            <form method="POST" action=action class="space-y-6">
                // Flowbite Tabbed Brand Settings Navigation
                <div class="border-b border-gray-200 dark:border-gray-700">
                    <ul class="flex flex-wrap -mb-px text-sm font-medium text-center" id="brand-settings-tabs" role="tablist" data-tabs-toggle="#brand-settings-tab-content" data-tabs-active-classes="text-primary-600 dark:text-primary-300 border-primary-600 dark:border-primary-300" data-tabs-inactive-classes="text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300 border-transparent hover:border-gray-300 dark:hover:border-gray-600">
                        <li class="me-2" role="presentation">
                            <button class="inline-flex items-center gap-2 p-4 border-b-2 rounded-t-lg" id="brand-general-tab" data-tabs-target="#brand-general-panel" type="button" role="tab" aria-controls="brand-general-panel" aria-selected="true">
                                <i class="fa-solid fa-sliders"></i>"General"
                            </button>
                        </li>
                        <li class="me-2" role="presentation">
                            <button class="inline-flex items-center gap-2 p-4 border-b-2 rounded-t-lg" id="brand-smtp-tab" data-tabs-target="#brand-smtp-panel" type="button" role="tab" aria-controls="brand-smtp-panel" aria-selected="false">
                                <i class="fa-solid fa-server"></i>"SMTP & Delivery"
                            </button>
                        </li>
                        <li class="me-2" role="presentation">
                            <button class="inline-flex items-center gap-2 p-4 border-b-2 rounded-t-lg" id="brand-tracking-tab" data-tabs-target="#brand-tracking-panel" type="button" role="tab" aria-controls="brand-tracking-panel" aria-selected="false">
                                <i class="fa-solid fa-chart-line"></i>"Tracking & Quota"
                            </button>
                        </li>
                        <li class="me-2" role="presentation">
                            <button class="inline-flex items-center gap-2 p-4 border-b-2 rounded-t-lg" id="brand-compliance-tab" data-tabs-target="#brand-compliance-panel" type="button" role="tab" aria-controls="brand-compliance-panel" aria-selected="false">
                                <i class="fa-solid fa-shield-halved"></i>"Compliance & Access"
                            </button>
                        </li>
                    </ul>
                </div>

                <div id="brand-settings-tab-content" class="pt-2">
                    // Tab 1: General Info & Defaults
                    <div id="brand-general-panel" role="tabpanel" aria-labelledby="brand-general-tab" class="space-y-5">
                        <Field
                            id="app_name" label="Brand name"
                            placeholder="Acme Newsletter"
                            value=m.app_name.clone().unwrap_or_default()
                            required=true
                        />
                        <div class="grid grid-cols-1 md:grid-cols-2 gap-5">
                            <Field id="from_name" label="From name"
                                value=m.from_name.clone().unwrap_or_default()/>
                            <Field id="from_email" label="From email" input_type="email"
                                value=m.from_email.clone().unwrap_or_default()/>
                        </div>
                        <Field id="reply_to" label="Reply-to" input_type="email"
                            value=m.reply_to.clone().unwrap_or_default()/>

                        <fieldset class="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
                            <legend class="text-sm font-semibold px-2 text-gray-700 dark:text-gray-300">"Send defaults"</legend>
                            <div class="grid grid-cols-1 md:grid-cols-2 gap-5">
                                <Field id="allowed_attachments" label="Allowed attachment extensions (comma-sep)"
                                    value=m.allowed_attachments.clone().unwrap_or_default()
                                    placeholder="pdf,doc,docx,jpg,png"/>
                                <Field id="test_email_prefix" label="Test-send subject prefix"
                                    value=m.test_email_prefix.clone().unwrap_or_default()
                                    placeholder="[TEST]"/>
                                <Field id="query_string" label="Default UTM/query string"
                                    value=m.query_string.clone().unwrap_or_default()
                                    placeholder="utm_source=newsletter&utm_medium=email"/>
                                <SelectField id="opt_in" label="New-list opt-in default"
                                    value=m.opt_in.unwrap_or(1).to_string()
                                    options=vec![("1","Double opt-in"),("0","Single opt-in")]/>
                                <SelectField id="hide_lists" label="Hidden lists in subscribe form"
                                    value=m.hide_lists.unwrap_or(1).to_string()
                                    options=vec![("1","Hide hidden lists"),("0","Show hidden lists")]/>
                                <SelectField id="templates_lists_sorting" label="Sort templates / lists by"
                                    value=m.templates_lists_sorting.clone().unwrap_or_else(|| "name".into())
                                    options=vec![("name","Name (A→Z)"),("date","Date created")]/>
                            </div>
                        </fieldset>
                    </div>

                    // Tab 2: SMTP & Delivery
                    <div class="hidden space-y-5" id="brand-smtp-panel" role="tabpanel" aria-labelledby="brand-smtp-tab">
                        {if is_edit {
                            let token = OpaqueId::<Brand>::new(m.id).to_string();
                            let badge = ses_status.map(|s| match s {
                                crate::services::ses::VerificationStatus::Verified =>
                                    ("bg-green-100 text-green-800", "Verified"),
                                crate::services::ses::VerificationStatus::Pending =>
                                    ("bg-amber-100 text-amber-800", "Pending — check inbox"),
                                crate::services::ses::VerificationStatus::Failed =>
                                    ("bg-red-100 text-red-800", "Failed"),
                                crate::services::ses::VerificationStatus::TemporaryFailure =>
                                    ("bg-orange-100 text-orange-800", "Temporary failure"),
                                crate::services::ses::VerificationStatus::NotStarted =>
                                    ("bg-gray-200 text-gray-700 dark:text-gray-300", "Not started"),
                                crate::services::ses::VerificationStatus::NotFound =>
                                    ("bg-gray-100 text-gray-700 dark:text-gray-300", "Not registered with SES"),
                            });
                            view! {
                                <fieldset class="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
                                    <legend class="text-sm font-semibold px-2 text-gray-700 dark:text-gray-300">"AWS SES verification"</legend>
                                    <div class="flex items-center gap-3">
                                        <p class="text-sm text-gray-600 dark:text-gray-400 grow">
                                            "Verify the brand's "<code class="bg-gray-100 dark:bg-gray-700 px-1.5 rounded">"From email"</code>" so AWS SES will accept outbound mail from this brand. Uses the AWS keys saved on your account."
                                        </p>
                                        {badge.map(|(cls, label)| {
                                            let cls = format!("text-xs font-medium px-2.5 py-0.5 rounded {cls}");
                                            view! { <span class=cls>{label}</span> }
                                        })}
                                        <form method="POST" action=format!("/app/brands/{token}/ses/verify")>
                                            <button type="submit" class="text-white bg-blue-600 hover:bg-blue-700 font-medium rounded-lg text-sm px-4 py-2">
                                                "Send verification email"
                                            </button>
                                        </form>
                                    </div>
                                </fieldset>
                            }.into_any()
                        } else { ().into_any() }}

                        <fieldset class="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
                            <legend class="text-sm font-semibold px-2 text-gray-700 dark:text-gray-300">"SMTP Configuration"</legend>
                            <div class="mb-4">
                                <label for="smtp_preset" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">
                                    "Provider quick-select"
                                </label>
                                <select id="smtp_preset"
                                    class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full md:w-1/2 p-2.5">
                                    <option value="">"Custom (don't auto-fill)"</option>
                                    <option value="ses-us-east-1">"Amazon SES — us-east-1"</option>
                                    <option value="ses-us-west-2">"Amazon SES — us-west-2"</option>
                                    <option value="ses-eu-west-1">"Amazon SES — eu-west-1"</option>
                                    <option value="sendgrid">"SendGrid"</option>
                                    <option value="mailjet">"Mailjet"</option>
                                    <option value="elasticemail">"ElasticEmail"</option>
                                    <option value="postmark">"Postmark"</option>
                                    <option value="mailgun-us">"Mailgun — US"</option>
                                    <option value="mailgun-eu">"Mailgun — EU"</option>
                                    <option value="brevo">"Brevo (Sendinblue)"</option>
                                </select>
                                <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
                                    "Picking a preset only fills Host / Port / Encryption — username + password stay yours to enter."
                                </p>
                            </div>
                            <div class="grid grid-cols-1 md:grid-cols-2 gap-5">
                                <Field id="smtp_host" label="Host"
                                    value=m.smtp_host.clone().unwrap_or_default()/>
                                <Field id="smtp_port" label="Port (typically 587 for TLS, 465 for SSL)"
                                    value=m.smtp_port.clone().unwrap_or_default()/>
                                <SelectField id="smtp_ssl" label="Encryption"
                                    value=m.smtp_ssl.clone().unwrap_or_default()
                                    options=vec![
                                        ("","None — port-driven default"),
                                        ("tls","STARTTLS (port 587)"),
                                        ("ssl","SSL (port 465)"),
                                    ]/>
                                <Field id="smtp_username" label="Username"
                                    value=m.smtp_username.clone().unwrap_or_default()/>
                                <Field id="smtp_password" label="Password" input_type="password"
                                    value=m.smtp_password.clone().unwrap_or_default()/>
                            </div>
                            <script>{smtp_preset_js()}</script>
                        </fieldset>
                    </div>

                    // Tab 3: Tracking & Custom Domain
                    <div class="hidden space-y-5" id="brand-tracking-panel" role="tabpanel" aria-labelledby="brand-tracking-tab">
                        <fieldset class="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
                            <legend class="text-sm font-semibold px-2 text-gray-700 dark:text-gray-300">"Tracking & quota"</legend>
                            <div class="grid grid-cols-1 md:grid-cols-2 gap-5">
                                <SelectField id="opens_tracking" label="Open tracking"
                                    value=m.opens_tracking.unwrap_or(1).to_string()
                                    options=vec![("1","Enabled"),("0","Disabled"),("2","Brand-wide")]/>
                                <SelectField id="links_tracking" label="Click tracking"
                                    value=m.links_tracking.unwrap_or(1).to_string()
                                    options=vec![("1","Enabled"),("0","Disabled"),("2","Brand-wide")]/>
                                <Field id="allocated_quota" label="Monthly quota (-1 = unlimited)"
                                    input_type="number"
                                    value=m.allocated_quota.unwrap_or(-1).to_string()/>
                                <Field id="campaign_report_rows" label="Report rows per page"
                                    input_type="number"
                                    value=m.campaign_report_rows.unwrap_or(10).to_string()/>
                            </div>
                        </fieldset>

                        <fieldset class="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
                            <legend class="text-sm font-semibold px-2 text-gray-700 dark:text-gray-300">"Custom domain (white-labelling)"</legend>
                            <div class="space-y-2 mb-3">
                                <Toggle id="custom_domain_enabled" label="Use a custom domain for tracking + subscribe URLs"
                                    checked=m.custom_domain_enabled.unwrap_or(0) == 1/>
                            </div>
                            <div class="grid grid-cols-3 gap-3">
                                <SelectField id="custom_domain_protocol" label="Protocol"
                                    value=m.custom_domain_protocol.clone().unwrap_or_else(|| "https".into())
                                    options=vec![("https","https://"),("http","http://")]/>
                                <div class="col-span-2">
                                    <Field id="custom_domain" label="Domain (no scheme)"
                                        value=m.custom_domain.clone().unwrap_or_default()
                                        placeholder="mail.example.com"/>
                                </div>
                            </div>
                            <p class="text-xs text-gray-500 dark:text-gray-400 mt-2">
                                "Point a CNAME for this hostname at your Mailer server before enabling. Tracking + subscribe links will use this domain in outbound emails."
                            </p>
                        </fieldset>
                    </div>

                    // Tab 4: Compliance & Sub-users
                    <div class="hidden space-y-5" id="brand-compliance-panel" role="tabpanel" aria-labelledby="brand-compliance-tab">
                        <fieldset class="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
                            <legend class="text-sm font-semibold px-2 text-gray-700 dark:text-gray-300">"GDPR"</legend>
                            <div class="space-y-2">
                                <Toggle id="gdpr_options" label="Show GDPR consent options on subscribe form"
                                    checked=m.gdpr_options.unwrap_or(0) == 1/>
                                <Toggle id="gdpr_only" label="Campaigns: only send to GDPR-confirmed subscribers"
                                    checked=m.gdpr_only.unwrap_or(0) == 1/>
                                <Toggle id="gdpr_only_ar" label="Autoresponders: only send to GDPR-confirmed subscribers"
                                    checked=m.gdpr_only_ar.unwrap_or(0) == 1/>
                            </div>
                        </fieldset>

                        <fieldset class="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
                            <legend class="text-sm font-semibold px-2 text-gray-700 dark:text-gray-300">"reCAPTCHA v2 (subscribe form anti-spam)"</legend>
                            <div class="grid grid-cols-1 md:grid-cols-2 gap-5">
                                <Field id="recaptcha_sitekey" label="Site key"
                                    value=m.recaptcha_sitekey.clone().unwrap_or_default()
                                    placeholder="6Lc..."/>
                                <Field id="recaptcha_secretkey" label="Secret key" input_type="password"
                                    value=m.recaptcha_secretkey.clone().unwrap_or_default()/>
                            </div>
                            <p class="text-xs text-gray-500 dark:text-gray-400 mt-2">
                                "Get your keys at "
                                <a href="https://www.google.com/recaptcha/admin" target="_blank" rel="noopener" class="text-blue-600 hover:underline">"Google reCAPTCHA admin"</a>
                                ". Pick reCAPTCHA v2 — \"I'm not a robot\" Checkbox."
                            </p>
                        </fieldset>

                        <fieldset class="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
                            <legend class="text-sm font-semibold px-2 text-gray-700 dark:text-gray-300">"Sub-user billing (optional)"</legend>
                            <p class="text-xs text-gray-500 dark:text-gray-400 mb-3">
                                "If you're reselling Mailer to clients, configure how each campaign send is billed back to the brand owner."
                            </p>
                            <div class="grid grid-cols-1 md:grid-cols-3 gap-5">
                                <Field id="currency" label="Currency"
                                    value=m.currency.clone().unwrap_or_else(|| "USD".into())
                                    placeholder="USD"/>
                                <Field id="delivery_fee" label="Delivery fee per send (flat)"
                                    value=m.delivery_fee.clone().unwrap_or_default()
                                    placeholder="0.00"/>
                                <Field id="cost_per_recipient" label="Cost per recipient"
                                    value=m.cost_per_recipient.clone().unwrap_or_default()
                                    placeholder="0.0001"/>
                            </div>
                        </fieldset>

                        <fieldset class="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
                            <legend class="text-sm font-semibold px-2 text-gray-700 dark:text-gray-300">"Sub-user UI restrictions"</legend>
                            <p class="text-xs text-gray-500 dark:text-gray-400 mb-3">
                                "When a sub-user signs in to this brand, hide everything except the selected resource. Leave all unchecked to grant full access. Only one of the toggles below should be on at a time."
                            </p>
                            <div class="space-y-2">
                                <Toggle id="campaigns_only" label="Campaigns only (no lists, templates, settings)"
                                    checked=m.campaigns_only.unwrap_or(0) == 1/>
                                <Toggle id="templates_only" label="Templates only (no campaigns, lists, settings)"
                                    checked=m.templates_only.unwrap_or(0) == 1/>
                                <Toggle id="lists_only" label="Lists only (no campaigns, templates, settings)"
                                    checked=m.lists_only.unwrap_or(0) == 1/>
                            </div>
                        </fieldset>
                    </div>
                </div>

                <div class="flex justify-end gap-3 pt-4 border-t border-gray-200 dark:border-gray-700">
                    <a href="/app/brands" class="text-sm font-medium text-gray-600 dark:text-gray-400 hover:underline self-center">"Cancel"</a>
                    <button type="submit"
                        class="text-white bg-blue-600 hover:bg-blue-700 focus:ring-4 focus:outline-none focus:ring-blue-300 font-medium rounded-lg text-sm px-5 py-2.5"
                    >
                        {if is_edit { "Save changes" } else { "Create brand" }}
                    </button>
                </div>
            </form>

            {if is_edit {
                let token = OpaqueId::<Brand>::new(m.id).to_string();
                let delete_action = format!("/app/brands/{}/delete", token);
                let logo_action = format!("/app/brands/{}/logo", token);
                let delete_logo_action = format!("/app/brands/{}/logo/delete", token);
                let logo_url = m.brand_logo_filename.as_deref()
                    .filter(|s| !s.is_empty())
                    .map(|f| format!("/uploads/logos/{}", f));
                view! {
                    <hr class="my-8"/>
                    <div class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-4 mb-4">
                        <h3 class="text-sm font-semibold text-gray-800 mb-2">"Brand logo"</h3>
                        {logo_url.clone().map(|u| view! {
                            <img src=u alt="Logo" class="h-16 mb-3 rounded border border-gray-200 dark:border-gray-700"/>
                        })}
                        <form method="POST" action=logo_action enctype="multipart/form-data" class="flex flex-wrap gap-3 items-end">
                            <div class="grow">
                                <label for="logo" class="block mb-1 text-xs font-medium text-gray-700 dark:text-gray-300">"Upload PNG/JPEG/GIF/SVG"</label>
                                <input type="file" id="logo" name="logo" accept="image/*" required
                                    class="block w-full text-sm text-gray-900 dark:text-white border border-gray-300 dark:border-gray-600 rounded-lg cursor-pointer bg-gray-50 dark:bg-gray-700"/>
                            </div>
                            <button type="submit"
                                class="text-white bg-blue-600 hover:bg-blue-700 font-medium rounded-lg text-sm px-4 py-2.5">
                                "Upload"
                            </button>
                        </form>
                        {logo_url.map(|_| view! {
                            <form method="POST" action=delete_logo_action class="mt-2">
                                <button type="submit" class="text-red-600 hover:underline text-xs">"Remove logo"</button>
                            </form>
                        })}
                    </div>

                    <div class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-4 mb-4">
                        <h3 class="text-sm font-semibold text-gray-800 mb-2">"Brand sub-users"</h3>
                        <p class="text-xs text-gray-500 dark:text-gray-400 mb-3">"Add a login that's restricted to this brand only."</p>

                        <div class="overflow-x-auto mb-3">
                            <table class="w-full text-sm">
                                <thead class="text-xs text-gray-700 dark:text-gray-300 uppercase bg-gray-50 dark:bg-gray-700">
                                    <tr>
                                        <th class="px-3 py-2 text-left">"Email"</th>
                                        <th class="px-3 py-2 text-left">"Name"</th>
                                        <th class="px-3 py-2 text-left">"2FA"</th>
                                        <th class="px-3 py-2 w-1"></th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {if sub_users.is_empty() {
                                        view! { <tr><td colspan="4" class="text-center text-gray-400 dark:text-gray-500 py-6">"No sub-users yet."</td></tr> }.into_any()
                                    } else {
                                        sub_users.into_iter().map(|u| {
                                            let action = format!("/app/users/{}/delete?brand={}", u.id, token);
                                            view! {
                                                <tr class="bg-white dark:bg-gray-800 border-b">
                                                    <td class="px-3 py-2 font-medium text-gray-900 dark:text-white">{u.username}</td>
                                                    <td class="px-3 py-2">{u.name.unwrap_or_default()}</td>
                                                    <td class="px-3 py-2">{if u.auth_enabled { "On" } else { "Off" }}</td>
                                                    <td class="px-3 py-2 text-right">
                                                        <form method="POST" action=action onsubmit="return confirm('Delete this sub-user?');">
                                                            <button type="submit" class="text-red-600 hover:underline text-xs">"Delete"</button>
                                                        </form>
                                                    </td>
                                                </tr>
                                            }
                                        }).collect_view().into_any()
                                    }}
                                </tbody>
                            </table>
                        </div>

                        <form method="POST" action=format!("/app/brands/{}/users/new", token)
                            class="grid grid-cols-1 md:grid-cols-4 gap-3">
                            <div>
                                <label class="block mb-1 text-xs font-medium text-gray-700 dark:text-gray-300" for="su_email">"Email"</label>
                                <input type="email" id="su_email" name="email" required
                                    class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 sm:text-sm rounded-lg block w-full p-2"/>
                            </div>
                            <div>
                                <label class="block mb-1 text-xs font-medium text-gray-700 dark:text-gray-300" for="su_name">"Name"</label>
                                <input type="text" id="su_name" name="name"
                                    class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 sm:text-sm rounded-lg block w-full p-2"/>
                            </div>
                            <div>
                                <label class="block mb-1 text-xs font-medium text-gray-700 dark:text-gray-300" for="su_password">"Initial password (≥8)"</label>
                                <input type="password" id="su_password" name="password" minlength="8" required
                                    class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 sm:text-sm rounded-lg block w-full p-2"/>
                            </div>
                            <div class="flex items-end">
                                <button type="submit" class="text-white bg-blue-600 hover:bg-blue-700 font-medium rounded-lg text-sm px-4 py-2 w-full">"Add sub-user"</button>
                            </div>
                        </form>
                    </div>

                    <div class="border border-red-200 bg-red-50 rounded-lg p-4 flex items-center justify-between">
                        <div>
                            <h3 class="text-sm font-semibold text-red-800">"Danger zone"</h3>
                            <p class="text-xs text-red-700">"Deleting a brand removes every list and subscriber under it. This cannot be undone."</p>
                        </div>
                        <form method="POST" action=delete_action onsubmit="return confirm('Permanently delete this brand and every list/subscriber under it?');">
                            <button type="submit"
                                class="text-white bg-red-600 hover:bg-red-700 focus:ring-4 focus:outline-none focus:ring-red-300 font-medium rounded-lg text-sm px-4 py-2"
                            >
                                "Delete brand"
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

// ── Form-helper components ─────────────────────────────────────────────────

#[component]
fn Field(
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
            <input
                type=input_type
                id=id.clone() name=id
                placeholder=placeholder value=value required=required
                class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg focus:ring-blue-600 focus:border-blue-600 block w-full p-2.5"
            />
        </div>
    }
}

#[component]
fn SelectField(
    #[prop(into)] id: String,
    #[prop(into)] label: String,
    #[prop(into)] value: String,
    options: Vec<(&'static str, &'static str)>,
) -> impl IntoView {
    let id_for = id.clone();
    let selected = value.clone();
    view! {
        <div>
            <label for=id_for class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">{label}</label>
            <select
                id=id.clone() name=id
                class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg focus:ring-blue-600 focus:border-blue-600 block w-full p-2.5"
            >
                {options.into_iter().map(|(v, l)| {
                    let is_selected = v == selected;
                    view! {
                        <option value=v selected=is_selected>{l}</option>
                    }
                }).collect_view()}
            </select>
        </div>
    }
}

#[component]
fn Toggle(
    #[prop(into)] id: String,
    #[prop(into)] label: String,
    #[prop(default = false)] checked: bool,
) -> impl IntoView {
    view! {
        <label class="inline-flex items-center cursor-pointer">
            <input type="checkbox" id=id.clone() name=id value="1" class="sr-only peer" checked=checked/>
            <div class="relative w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white dark:bg-gray-800 after:border-gray-300 dark:border-gray-600 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600"></div>
            <span class="ms-3 text-sm font-medium text-gray-900 dark:text-white">{label}</span>
        </label>
    }
}

// ── Form parsing ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct BrandForm {
    pub app_name: String,
    #[serde(default)]
    pub from_name: Option<String>,
    #[serde(default)]
    pub from_email: Option<String>,
    #[serde(default)]
    pub reply_to: Option<String>,
    #[serde(default)]
    pub smtp_host: Option<String>,
    #[serde(default)]
    pub smtp_port: Option<String>,
    #[serde(default)]
    pub smtp_ssl: Option<String>,
    #[serde(default)]
    pub smtp_username: Option<String>,
    #[serde(default)]
    pub smtp_password: Option<String>,
    #[serde(default)]
    pub allowed_attachments: Option<String>,
    #[serde(default)]
    pub query_string: Option<String>,
    #[serde(default)]
    pub gdpr_only: Option<String>,
    #[serde(default)]
    pub gdpr_only_ar: Option<String>,
    #[serde(default)]
    pub gdpr_options: Option<String>,
    #[serde(default)]
    pub recaptcha_sitekey: Option<String>,
    #[serde(default)]
    pub recaptcha_secretkey: Option<String>,
    #[serde(default)]
    pub test_email_prefix: Option<String>,
    #[serde(default)]
    pub custom_domain: Option<String>,
    #[serde(default)]
    pub custom_domain_protocol: Option<String>,
    #[serde(default)]
    pub custom_domain_enabled: Option<String>,
    #[serde(default)]
    pub templates_lists_sorting: Option<String>,
    #[serde(default = "default_one_str")]
    pub opens_tracking: String,
    #[serde(default = "default_one_str")]
    pub links_tracking: String,
    #[serde(default = "default_one_str")]
    pub hide_lists: String,
    #[serde(default = "default_one_str")]
    pub opt_in: String,
    #[serde(default)]
    pub allocated_quota: Option<String>,
    #[serde(default)]
    pub campaign_report_rows: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub delivery_fee: Option<String>,
    #[serde(default)]
    pub cost_per_recipient: Option<String>,
    #[serde(default)]
    pub campaigns_only: Option<String>,
    #[serde(default)]
    pub templates_only: Option<String>,
    #[serde(default)]
    pub lists_only: Option<String>,
}

fn default_one_str() -> String {
    "1".to_string()
}

impl From<BrandForm> for BrandInput {
    fn from(f: BrandForm) -> Self {
        let parse_i16 = |s: &str, dflt: i16| s.parse().unwrap_or(dflt);
        let parse_i32 = |s: &Option<String>| s.as_deref().and_then(|v| v.parse().ok());
        BrandInput {
            app_name: f.app_name.trim().to_string(),
            from_name: nonempty(f.from_name),
            from_email: nonempty(f.from_email),
            reply_to: nonempty(f.reply_to),
            smtp_host: nonempty(f.smtp_host),
            smtp_port: nonempty(f.smtp_port),
            smtp_ssl: nonempty(f.smtp_ssl),
            smtp_username: nonempty(f.smtp_username),
            smtp_password: nonempty(f.smtp_password),
            allowed_attachments: nonempty(f.allowed_attachments),
            query_string: nonempty(f.query_string),
            gdpr_only: f.gdpr_only.is_some(),
            gdpr_only_ar: f.gdpr_only_ar.is_some(),
            gdpr_options: f.gdpr_options.is_some(),
            recaptcha_sitekey: nonempty(f.recaptcha_sitekey),
            recaptcha_secretkey: nonempty(f.recaptcha_secretkey),
            test_email_prefix: nonempty(f.test_email_prefix),
            custom_domain: nonempty(f.custom_domain),
            custom_domain_protocol: nonempty(f.custom_domain_protocol),
            custom_domain_enabled: matches!(f.custom_domain_enabled.as_deref(), Some("1") | Some("on")),
            templates_lists_sorting: nonempty(f.templates_lists_sorting),
            opens_tracking: parse_i16(&f.opens_tracking, 1),
            links_tracking: parse_i16(&f.links_tracking, 1),
            hide_lists: parse_i16(&f.hide_lists, 1),
            opt_in: parse_i16(&f.opt_in, 1),
            allocated_quota: parse_i32(&f.allocated_quota).or(Some(-1)),
            campaign_report_rows: parse_i32(&f.campaign_report_rows).or(Some(10)),
            currency: nonempty(f.currency),
            delivery_fee: nonempty(f.delivery_fee),
            cost_per_recipient: nonempty(f.cost_per_recipient),
            campaigns_only: f.campaigns_only.is_some(),
            templates_only: f.templates_only.is_some(),
            lists_only: f.lists_only.is_some(),
        }
    }
}

/// Inline JS that wires the SMTP-preset dropdown to the host / port /
/// encryption inputs. Picking a preset fills those three fields;
/// username + password stay user-entered.
fn smtp_preset_js() -> &'static str {
    r#"
(function () {
    var sel = document.getElementById('smtp_preset');
    if (!sel) return;
    var presets = {
        'ses-us-east-1': { host: 'email-smtp.us-east-1.amazonaws.com', port: '587', ssl: 'tls' },
        'ses-us-west-2': { host: 'email-smtp.us-west-2.amazonaws.com', port: '587', ssl: 'tls' },
        'ses-eu-west-1': { host: 'email-smtp.eu-west-1.amazonaws.com', port: '587', ssl: 'tls' },
        'sendgrid':      { host: 'smtp.sendgrid.net',                  port: '587', ssl: 'tls' },
        'mailjet':       { host: 'in-v3.mailjet.com',                  port: '587', ssl: 'tls' },
        'elasticemail':  { host: 'smtp.elasticemail.com',              port: '2525', ssl: 'tls' },
        'postmark':      { host: 'smtp.postmarkapp.com',               port: '587', ssl: 'tls' },
        'mailgun-us':    { host: 'smtp.mailgun.org',                   port: '587', ssl: 'tls' },
        'mailgun-eu':    { host: 'smtp.eu.mailgun.org',                port: '587', ssl: 'tls' },
        'brevo':         { host: 'smtp-relay.brevo.com',               port: '587', ssl: 'tls' },
    };
    sel.addEventListener('change', function () {
        var p = presets[sel.value];
        if (!p) return;
        var host = document.getElementById('smtp_host');
        var port = document.getElementById('smtp_port');
        var ssl  = document.getElementById('smtp_ssl');
        if (host) host.value = p.host;
        if (port) port.value = p.port;
        if (ssl)  ssl.value  = p.ssl;
    });
})();
"#
}

pub(crate) fn nonempty(s: Option<String>) -> Option<String> {
    s.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// A zeroed `apps::Model` used for the blank "New brand" form. SeaORM doesn't
/// derive `Default` on its models so we hand-roll a constructor.
fn blank_brand() -> crate::database::entities::apps::Model {
    crate::database::entities::apps::Model {
        id: 0,
        user_id: None,
        app_name: None,
        from_name: None,
        from_email: None,
        reply_to: None,
        app_key: None,
        brand_logo_filename: None,
        currency: None,
        delivery_fee: None,
        cost_per_recipient: None,
        smtp_host: None,
        smtp_port: None,
        smtp_ssl: None,
        smtp_username: None,
        smtp_password: None,
        allocated_quota: Some(-1),
        current_quota: Some(0),
        day_of_reset: None,
        month_of_next_reset: None,
        year_of_next_reset: None,
        no_expiry: Some(0),
        test_email: None,
        test_email_prefix: None,
        allowed_attachments: None,
        bounce_setup: Some(0),
        complaint_setup: Some(0),
        campaigns_only: Some(0),
        templates_only: Some(0),
        lists_only: Some(0),
        gdpr_only: Some(0),
        gdpr_only_ar: Some(0),
        gdpr_options: Some(0),
        recaptcha_sitekey: None,
        recaptcha_secretkey: None,
        custom_domain: None,
        custom_domain_protocol: None,
        custom_domain_enabled: Some(0),
        templates_lists_sorting: None,
        campaign_report_rows: Some(10),
        query_string: None,
        opens_tracking: Some(1),
        links_tracking: Some(1),
        opt_in: Some(1),
        hide_lists: Some(1),
    }
}

