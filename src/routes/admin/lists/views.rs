//! List admin pages and form-action handlers.

use leptos::prelude::*;
use serde::Deserialize;

use crate::services::lists::{
    self, CustomField, ListInput, ListSummary,
};
use crate::utils::opaque_id::{List, OpaqueId};

// ── Pages ────────────────────────────────────────────────────────────────────

#[component]
pub fn ListsIndexPage(
    #[prop(into)] brand_token: String,
    #[prop(into)] brand_name: String,
    lists: Vec<ListSummary>,
) -> impl IntoView {
    let new_href = format!("/app/brands/{brand_token}/lists/new");
    let brands_href = "/app/brands".to_string();
    let bulk_action = format!("/app/brands/{brand_token}/lists/bulk-toggle");
    view! {
        <section class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6">
            <nav class="text-sm text-gray-500 dark:text-gray-400 mb-4">
                <a href=brands_href class="hover:underline">"Brands"</a>
                " / "<span class="text-gray-700 dark:text-gray-300">{brand_name.clone()}</span>
            </nav>
            <header class="flex flex-col sm:flex-row justify-between items-start sm:items-center mb-6 gap-3">
                <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">"Lists"</h1>
                <a href=new_href
                    class="inline-flex items-center text-white bg-blue-600 hover:bg-blue-700 focus:ring-4 focus:outline-none focus:ring-blue-300 font-medium rounded-lg text-sm px-4 py-2"
                >"+ New list"</a>
            </header>
            <form method="POST" action=bulk_action>
                <div class="relative overflow-x-auto sm:rounded-lg">
                    <table class="w-full text-sm text-left text-gray-500 dark:text-gray-400">
                        <thead class="text-xs text-gray-700 dark:text-gray-300 uppercase bg-gray-50 dark:bg-gray-700">
                            <tr>
                                <th class="px-4 py-3 w-1"><input type="checkbox" id="bulk-select-all" class="h-4 w-4"/></th>
                                <th class="px-4 py-3">"Name"</th>
                                <th class="px-4 py-3 text-right">"Active subscribers"</th>
                                <th class="px-4 py-3 text-center">"Status"</th>
                                <th class="px-4 py-3"><span class="sr-only">"Actions"</span></th>
                            </tr>
                        </thead>
                        <tbody>
                            {if lists.is_empty() {
                                view! {
                                    <tr><td colspan="5" class="text-center text-gray-400 dark:text-gray-500 py-10">"No lists yet."</td></tr>
                                }.into_any()
                            } else {
                                lists.into_iter().map(|l| {
                                    let token = OpaqueId::<List>::new(l.id).to_string();
                                    let view_href = format!("/app/lists/{token}");
                                    let edit_href = format!("/app/lists/{token}/edit");
                                    view! {
                                        <tr class="bg-white dark:bg-gray-800 border-b hover:bg-gray-50 dark:hover:bg-gray-600">
                                            <td class="px-4 py-3">
                                                <input type="checkbox" name="list_ids" value=l.id.to_string() class="bulk-row h-4 w-4"/>
                                            </td>
                                            <th scope="row" class="px-4 py-3 font-medium text-gray-900 dark:text-white whitespace-nowrap">
                                                <a href=view_href class="hover:underline">{l.name}</a>
                                            </th>
                                            <td class="px-4 py-3 text-right">{l.subscriber_count}</td>
                                            <td class="px-4 py-3 text-center">
                                                {if l.hide {
                                                    view! { <span class="bg-gray-100 text-gray-700 dark:text-gray-300 text-xs font-medium px-2.5 py-0.5 rounded">"Hidden"</span> }.into_any()
                                                } else {
                                                    view! { <span class="bg-green-100 text-green-800 text-xs font-medium px-2.5 py-0.5 rounded">"Active"</span> }.into_any()
                                                }}
                                            </td>
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
                <div class="flex gap-2 justify-end pt-3 border-t border-gray-200 dark:border-gray-700 mt-3">
                    <button type="submit" name="op" value="hide"
                        class="text-sm font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 rounded-lg px-3 py-2">
                        "Hide selected"
                    </button>
                    <button type="submit" name="op" value="show"
                        class="text-sm font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 rounded-lg px-3 py-2">
                        "Show selected"
                    </button>
                </div>
            </form>
            <script>{r#"document.getElementById('bulk-select-all')?.addEventListener('change',function(e){document.querySelectorAll('input.bulk-row').forEach(function(cb){cb.checked=e.target.checked;});});"#}</script>
        </section>
    }
}

#[component]
pub fn ListFormPage(
    #[prop(into)] brand_token: String,
    #[prop(optional)] existing: Option<crate::database::entities::lists::Model>,
) -> impl IntoView {
    let is_edit = existing.is_some();
    let action = match existing.as_ref() {
        Some(m) => format!("/app/lists/{}/edit", OpaqueId::<List>::new(m.id)),
        None => format!("/app/brands/{}/lists/new", brand_token),
    };
    let title = if is_edit { "Edit list" } else { "New list" };
    let m = existing.unwrap_or_else(blank_list);
    let custom_fields = lists::parse_custom_fields(m.custom_fields.as_deref().unwrap_or(""));
    let edit_id = m.id;

    view! {
        <section class="space-y-6">
            <header>
                <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">{title}</h1>
            </header>

            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6">
                <form method="POST" action=action class="space-y-5 max-w-3xl">
                    <Field id="name" label="List name" required=true
                        value=m.name.clone().unwrap_or_default()
                        placeholder="My newsletter"/>

                    <crate::components::layout::PersonalizationTagsCard/>

                    <fieldset class="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
                        <legend class="text-sm font-semibold px-2 text-gray-700 dark:text-gray-300">"Subscribe behaviour"</legend>
                        <SelectField id="opt_in" label="Opt-in"
                            value=m.opt_in.unwrap_or(1).to_string()
                            options=vec![("1","Double opt-in"),("0","Single opt-in")]/>
                        <Field id="confirmation_subject" label="Double-opt-in confirmation email — subject"
                            value=m.confirmation_subject.clone().unwrap_or_default()/>
                        <TextareaField id="confirmation_email" label="Double-opt-in confirmation email — HTML body"
                            value=m.confirmation_email.clone().unwrap_or_default()
                            rows=8 wysiwyg=true
                            hint="Use [confirmation_link] in the body — it's replaced with the per-subscriber confirm URL at send time."/>
                        <Field id="subscribed_url" label="Subscribed redirect URL"
                            value=m.subscribed_url.clone().unwrap_or_default()/>
                        <Field id="confirm_url" label="Confirmation redirect URL"
                            value=m.confirm_url.clone().unwrap_or_default()/>
                        <Field id="already_subscribed_url" label="Already-subscribed redirect URL"
                            value=m.already_subscribed_url.clone().unwrap_or_default()/>
                    </fieldset>

                    <fieldset class="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
                        <legend class="text-sm font-semibold px-2 text-gray-700 dark:text-gray-300">"Thank-you email (sent after confirm)"</legend>
                        <CheckboxField id="thankyou" label="Send a thank-you email"
                            checked=m.thankyou.unwrap_or(0) == 1/>
                        <Field id="thankyou_subject" label="Subject"
                            value=m.thankyou_subject.clone().unwrap_or_default()/>
                        <TextareaField id="thankyou_message" label="HTML body"
                            value=m.thankyou_message.clone().unwrap_or_default()
                            rows=6 wysiwyg=true
                            hint="Personalization tags like [Name] and [Email] are supported."/>
                    </fieldset>

                    <fieldset class="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
                        <legend class="text-sm font-semibold px-2 text-gray-700 dark:text-gray-300">"Unsubscribe"</legend>
                        <SelectField id="unsubscribe_confirm" label="Double opt-out"
                            value=m.unsubscribe_confirm.unwrap_or(0).to_string()
                            options=vec![("0","Off — single click"),("1","Confirm before unsubscribing")]/>
                        <SelectField id="unsubscribe_all_list" label="Unsubscribe scope"
                            value=m.unsubscribe_all_list.unwrap_or(0).to_string()
                            options=vec![("0","Just this list"),("1","All lists in this brand")]/>
                        <Field id="unsubscribed_url" label="Unsubscribed redirect URL"
                            value=m.unsubscribed_url.clone().unwrap_or_default()/>
                    </fieldset>

                    <fieldset class="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
                        <legend class="text-sm font-semibold px-2 text-gray-700 dark:text-gray-300">"Goodbye email (sent after unsubscribe)"</legend>
                        <CheckboxField id="goodbye" label="Send a goodbye email"
                            checked=m.goodbye.unwrap_or(0) == 1/>
                        <Field id="goodbye_subject" label="Subject"
                            value=m.goodbye_subject.clone().unwrap_or_default()/>
                        <TextareaField id="goodbye_message" label="HTML body"
                            value=m.goodbye_message.clone().unwrap_or_default()
                            rows=6 wysiwyg=true
                            hint="Personalization tags work here too."/>
                    </fieldset>

                    <fieldset class="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
                        <legend class="text-sm font-semibold px-2 text-gray-700 dark:text-gray-300">"GDPR"</legend>
                        <SelectField id="gdpr_enabled" label="GDPR consent on subscribe form"
                            value=m.gdpr_enabled.unwrap_or(0).to_string()
                            options=vec![("0","Disabled"),("1","Enabled")]/>
                        <Field id="marketing_permission" label="Marketing permission text"
                            value=m.marketing_permission.clone().unwrap_or_default()/>
                        <Field id="what_to_expect" label="What to expect"
                            value=m.what_to_expect.clone().unwrap_or_default()/>
                        <Field id="reconsent_success_url" label="Reconsent success URL"
                            value=m.reconsent_success_url.clone().unwrap_or_default()/>
                        <Field id="no_consent_url" label="Consent-not-given URL"
                            value=m.no_consent_url.clone().unwrap_or_default()/>
                    </fieldset>

                    <div class="flex justify-end gap-3 pt-2">
                        <a href="/app/brands" class="text-sm font-medium text-gray-600 dark:text-gray-400 hover:underline self-center">"Cancel"</a>
                        <button type="submit"
                            class="text-white bg-blue-600 hover:bg-blue-700 focus:ring-4 focus:outline-none focus:ring-blue-300 font-medium rounded-lg text-sm px-5 py-2.5"
                        >{if is_edit {"Save changes"} else {"Create list"}}</button>
                    </div>
                </form>
            </div>

            {if is_edit {
                let token = OpaqueId::<List>::new(edit_id).to_string();
                view! {
                    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6">
                        <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">"Custom fields"</h2>
                        <CustomFieldsTable list_token=token.clone() fields=custom_fields/>
                        <form method="POST" action=format!("/app/lists/{}/custom-fields/add", token) class="mt-4 flex flex-wrap gap-3 items-end">
                            <div class="grow">
                                <label for="c_field" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"New field name"</label>
                                <input type="text" id="c_field" name="c_field" required
                                    class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
                            </div>
                            <div>
                                <label for="c_type" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Type"</label>
                                <select id="c_type" name="c_type"
                                    class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block p-2.5">
                                    <option value="Text">"Text"</option>
                                    <option value="Date">"Date"</option>
                                </select>
                            </div>
                            <button type="submit"
                                class="text-white bg-blue-600 hover:bg-blue-700 font-medium rounded-lg text-sm px-4 py-2.5">
                                "Add field"
                            </button>
                        </form>
                    </div>
                    <div class="border border-red-200 bg-red-50 rounded-lg p-4 flex items-center justify-between">
                        <div>
                            <h3 class="text-sm font-semibold text-red-800">"Danger zone"</h3>
                            <p class="text-xs text-red-700">"Deleting this list removes every subscriber on it."</p>
                        </div>
                        <form method="POST" action=format!("/app/lists/{}/delete", OpaqueId::<List>::new(edit_id))
                            onsubmit="return confirm('Delete this list and all its subscribers?');">
                            <button type="submit"
                                class="text-white bg-red-600 hover:bg-red-700 font-medium rounded-lg text-sm px-4 py-2">
                                "Delete list"
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
fn CustomFieldsTable(
    #[prop(into)] list_token: String,
    fields: Vec<CustomField>,
) -> impl IntoView {
    if fields.is_empty() {
        return view! {
            <p class="text-sm text-gray-500 dark:text-gray-400">"No custom fields defined yet."</p>
        }.into_any();
    }
    view! {
        <table class="w-full text-sm">
            <thead class="text-xs text-gray-700 dark:text-gray-300 uppercase bg-gray-50 dark:bg-gray-700">
                <tr>
                    <th class="px-4 py-2 text-left">"Name"</th>
                    <th class="px-4 py-2 text-left">"Type"</th>
                    <th class="px-4 py-2 w-1"><span class="sr-only">"Actions"</span></th>
                </tr>
            </thead>
            <tbody>
                {fields.into_iter().map(|f| {
                    let action = format!("/app/lists/{}/custom-fields/delete", list_token);
                    view! {
                        <tr class="bg-white dark:bg-gray-800 border-b">
                            <td class="px-4 py-2 font-medium text-gray-900 dark:text-white">{f.name.clone()}</td>
                            <td class="px-4 py-2 text-gray-500 dark:text-gray-400">{f.kind.as_str()}</td>
                            <td class="px-4 py-2 text-right">
                                <form method="POST" action=action.clone()>
                                    <input type="hidden" name="name" value=f.name.clone()/>
                                    <button type="submit" class="text-red-600 hover:underline text-sm">
                                        "Remove"
                                    </button>
                                </form>
                            </td>
                        </tr>
                    }
                }).collect_view()}
            </tbody>
        </table>
    }.into_any()
}

// ── Form helpers ─────────────────────────────────────────────────────────────

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
        <div class="mb-3">
            <label for=id_for class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">{label}</label>
            <input type=input_type id=id.clone() name=id placeholder=placeholder value=value required=required
                class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg focus:ring-blue-600 focus:border-blue-600 block w-full p-2.5"/>
        </div>
    }
}

#[component]
fn TextareaField(
    #[prop(into)] id: String,
    #[prop(into)] label: String,
    #[prop(into, optional)] value: String,
    #[prop(default = 5)] rows: u32,
    #[prop(default = false)] wysiwyg: bool,
    #[prop(into, optional)] hint: String,
) -> impl IntoView {
    let id_for = id.clone();
    let class = if wysiwyg {
        "wysiwyg bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5 font-mono"
    } else {
        "bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5 font-mono"
    };
    let target_id = id.clone();
    view! {
        <div class="mb-4">
            <label for=id_for class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">{label}</label>
            {if wysiwyg {
                view! {
                    <div class="space-y-2" data-templatical-editor data-target-input=target_id.clone()>
                        <div class="templatical-inline-container w-full rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 overflow-hidden" style="height: 380px;"></div>
                        <details class="text-xs text-gray-500 dark:text-gray-400">
                            <summary class="cursor-pointer hover:underline py-1">"View / edit raw HTML source"</summary>
                            <textarea id=id.clone() name=id rows=rows.to_string() class=class>{value}</textarea>
                        </details>
                    </div>
                }.into_any()
            } else {
                view! {
                    <textarea id=id.clone() name=id rows=rows.to_string() class=class>{value}</textarea>
                }.into_any()
            }}
            {if hint.is_empty() {
                ().into_any()
            } else {
                view! { <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">{hint}</p> }.into_any()
            }}
        </div>
    }
}

#[component]
fn CheckboxField(
    #[prop(into)] id: String,
    #[prop(into)] label: String,
    #[prop(default = false)] checked: bool,
) -> impl IntoView {
    let id_for = id.clone();
    view! {
        <div class="mb-3">
            <label for=id_for class="inline-flex items-center gap-2 text-sm font-medium text-gray-900 dark:text-white">
                <input type="checkbox" id=id.clone() name=id value="1" checked=checked
                    class="h-4 w-4 text-primary-600 border-gray-300 dark:border-gray-600 rounded"/>
                {label}
            </label>
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
    let selected = value;
    view! {
        <div class="mb-3">
            <label for=id_for class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">{label}</label>
            <select id=id.clone() name=id
                class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5">
                {options.into_iter().map(|(v, l)| {
                    let is_selected = v == selected;
                    view! { <option value=v selected=is_selected>{l}</option> }
                }).collect_view()}
            </select>
        </div>
    }
}

// ── Form payload + parsing ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListForm {
    pub name: String,
    #[serde(default = "default_one_str")]
    pub opt_in: String,
    #[serde(default)]
    pub subscribed_url: Option<String>,
    #[serde(default)]
    pub confirm_url: Option<String>,
    #[serde(default)]
    pub already_subscribed_url: Option<String>,
    #[serde(default)]
    pub no_consent_url: Option<String>,
    #[serde(default)]
    pub reconsent_success_url: Option<String>,
    #[serde(default)]
    pub thankyou: Option<String>,
    #[serde(default)]
    pub thankyou_subject: Option<String>,
    #[serde(default)]
    pub thankyou_message: Option<String>,
    #[serde(default = "default_zero_str")]
    pub unsubscribe_all_list: String,
    #[serde(default = "default_zero_str")]
    pub unsubscribe_confirm: String,
    #[serde(default)]
    pub unsubscribed_url: Option<String>,
    #[serde(default)]
    pub goodbye: Option<String>,
    #[serde(default)]
    pub goodbye_subject: Option<String>,
    #[serde(default)]
    pub goodbye_message: Option<String>,
    #[serde(default)]
    pub confirmation_subject: Option<String>,
    #[serde(default)]
    pub confirmation_email: Option<String>,
    #[serde(default = "default_zero_str")]
    pub gdpr_enabled: String,
    #[serde(default)]
    pub marketing_permission: Option<String>,
    #[serde(default)]
    pub what_to_expect: Option<String>,
}

fn default_one_str() -> String { "1".into() }
fn default_zero_str() -> String { "0".into() }

impl From<ListForm> for ListInput {
    fn from(f: ListForm) -> Self {
        let i16_or = |s: &str, d: i16| s.parse().unwrap_or(d);
        ListInput {
            name: f.name.trim().to_string(),
            opt_in: i16_or(&f.opt_in, 1),
            subscribed_url: nonempty(f.subscribed_url),
            confirm_url: nonempty(f.confirm_url),
            already_subscribed_url: nonempty(f.already_subscribed_url),
            no_consent_url: nonempty(f.no_consent_url),
            reconsent_success_url: nonempty(f.reconsent_success_url),
            thankyou: if f.thankyou.is_some() { 1 } else { 0 },
            thankyou_subject: nonempty(f.thankyou_subject),
            thankyou_message: nonempty(f.thankyou_message),
            unsubscribe_all_list: i16_or(&f.unsubscribe_all_list, 0),
            unsubscribe_confirm: i16_or(&f.unsubscribe_confirm, 0),
            unsubscribed_url: nonempty(f.unsubscribed_url),
            goodbye: if f.goodbye.is_some() { 1 } else { 0 },
            goodbye_subject: nonempty(f.goodbye_subject),
            goodbye_message: nonempty(f.goodbye_message),
            confirmation_subject: nonempty(f.confirmation_subject),
            confirmation_email: nonempty(f.confirmation_email),
            gdpr_enabled: i16_or(&f.gdpr_enabled, 0),
            marketing_permission: nonempty(f.marketing_permission),
            what_to_expect: nonempty(f.what_to_expect),
            gdpr: 0,
        }
    }
}

pub(crate) fn nonempty(s: Option<String>) -> Option<String> {
    s.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

// ── GET handlers ────────────────────────────────────────────────────────────


fn blank_list() -> crate::database::entities::lists::Model {
    crate::database::entities::lists::Model {
        id: 0, app: None, user_id: None, name: None,
        opt_in: Some(1),
        confirm_url: None, subscribed_url: None, unsubscribed_url: None,
        no_consent_url: None, already_subscribed_url: None, reconsent_success_url: None,
        thankyou: Some(0), thankyou_subject: None, thankyou_message: None,
        goodbye: Some(0), goodbye_subject: None, goodbye_message: None,
        unsubscribe_all_list: Some(0), unsubscribe_confirm: Some(0),
        confirmation_subject: None, confirmation_email: None,
        custom_fields: Some(String::new()),
        prev_count: Some(0), currently_processing: Some(0), total_records: Some(0),
        gdpr_enabled: Some(0), marketing_permission: None, what_to_expect: None,
        gdpr: Some(0), hide: Some(0),
    }
}
