//! Template CRUD pages and form-action handlers.

use leptos::prelude::*;
use serde::Deserialize;

use crate::database::entities::template;
use crate::services::templates::TemplateInput;

#[derive(Debug, Deserialize)]
pub struct BrandQuery {
    pub brand: String,
}

#[derive(Debug, Deserialize)]
pub struct TemplateForm {
    pub template_name: String,
    #[serde(default)]
    pub html_text: Option<String>,
    #[serde(default)]
    pub plain_text: Option<String>,
    #[serde(default)]
    pub from_name: Option<String>,
    #[serde(default)]
    pub from_email: Option<String>,
    #[serde(default)]
    pub reply_to: Option<String>,
    /// Brand id (opaque) — required when creating.
    #[serde(default)]
    pub brand: Option<String>,
}

impl From<TemplateForm> for TemplateInput {
    fn from(f: TemplateForm) -> Self {
        TemplateInput {
            template_name: f.template_name.trim().to_string(),
            html_text: nonempty(f.html_text),
            plain_text: nonempty(f.plain_text),
            from_name: nonempty(f.from_name),
            from_email: nonempty(f.from_email),
            reply_to: nonempty(f.reply_to),
        }
    }
}

pub(crate) fn nonempty(s: Option<String>) -> Option<String> {
    s.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

// ── Pages ───────────────────────────────────────────────────────────────────

#[component]
pub fn TemplatesListPage(
    brand_token: String,
    brand_name: String,
    rows: Vec<template::Model>,
) -> impl IntoView {
    let new_href = format!("/app/templates/new?brand={}", brand_token);
    view! {
        <section class="space-y-4">
            <header class="flex justify-between items-center">
                <div>
                    <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">"Templates"</h1>
                    <p class="text-sm text-gray-500 dark:text-gray-400">{format!("Brand: {brand_name}")}</p>
                </div>
                <a href=new_href
                    class="inline-flex items-center text-white bg-blue-600 hover:bg-blue-700 font-medium rounded-lg text-sm px-4 py-2"
                >"+ New template"</a>
            </header>
            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm overflow-x-auto">
                <table class="w-full text-sm text-left text-gray-500 dark:text-gray-400">
                    <thead class="text-xs text-gray-700 dark:text-gray-300 uppercase bg-gray-50 dark:bg-gray-700">
                        <tr>
                            <th class="px-4 py-3">"Name"</th>
                            <th class="px-4 py-3">"From"</th>
                            <th class="px-4 py-3 w-1"></th>
                        </tr>
                    </thead>
                    <tbody>
                        {if rows.is_empty() {
                            view! { <tr><td colspan="3" class="text-center text-gray-400 dark:text-gray-500 py-10">"No templates yet."</td></tr> }.into_any()
                        } else {
                            rows.into_iter().map(|t| {
                                let edit_href = format!("/app/templates/{}/edit", t.id);
                                let use_action = format!("/app/templates/{}/use", t.id);
                                view! {
                                    <tr class="bg-white dark:bg-gray-800 border-b">
                                        <td class="px-4 py-3 font-medium text-gray-900 dark:text-white">{t.template_name.unwrap_or_default()}</td>
                                        <td class="px-4 py-3">
                                            {format!("{} <{}>",
                                                t.from_name.unwrap_or_default(),
                                                t.from_email.unwrap_or_default()
                                            )}
                                        </td>
                                        <td class="px-4 py-3 text-right whitespace-nowrap">
                                            <form method="POST" action=use_action class="inline">
                                                <button type="submit" class="text-emerald-700 hover:underline text-sm me-3">"Use"</button>
                                            </form>
                                            <a href=edit_href class="text-blue-600 hover:underline text-sm">"Edit"</a>
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
pub fn TemplateFormPage(
    #[prop(into)] brand_token: String,
    #[prop(optional)] existing: Option<template::Model>,
) -> impl IntoView {
    let is_edit = existing.is_some();
    let action = match existing.as_ref() {
        Some(m) => format!("/app/templates/{}/edit", m.id),
        None => "/app/templates/new".to_string(),
    };
    let title = if is_edit { "Edit template" } else { "New template" };
    let m = existing.unwrap_or_else(|| template::Model {
        id: 0, user_id: None, app: None,
        template_name: None, html_text: None, plain_text: None,
        from_name: None, from_email: None, reply_to: None,
    });
    let m_id = m.id;

    let back_url = format!("/app/brands/{brand_token}/templates");
    view! {
        <section class="space-y-4">
            <nav class="text-sm text-gray-500 dark:text-gray-400">
                <a href=back_url.clone() class="hover:underline">"← Templates"</a>
            </nav>
            <header>
                <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">{title}</h1>
                <p class="text-sm text-gray-500 dark:text-gray-400">"Reusable email body — pick it from the Templates list when creating a campaign."</p>
            </header>
            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6">
                <form method="POST" action=action class="space-y-4">
                    <input type="hidden" name="brand" value=brand_token.clone()/>
                    <Field id="template_name" label="Template name" required=true value=m.template_name.clone().unwrap_or_default()/>
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-5">
                        <Field id="from_name" label="From name" value=m.from_name.clone().unwrap_or_default()/>
                        <Field id="from_email" label="From email" input_type="email" value=m.from_email.clone().unwrap_or_default()/>
                    </div>
                    <Field id="reply_to" label="Reply-to" input_type="email" value=m.reply_to.clone().unwrap_or_default()/>
                    // Flowbite Tabbed Content: Visual Builder (Templatical), HTML Source & Plain text body
                    <div class="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden bg-gray-50/50 dark:bg-gray-900/50 p-4 space-y-4">
                        <div class="border-b border-gray-200 dark:border-gray-700 flex flex-wrap items-center justify-between gap-3 pb-1">
                            <ul class="flex flex-wrap -mb-px text-sm font-medium text-center" id="template-editor-tabs" role="tablist" data-tabs-toggle="#template-editor-tab-content" data-tabs-active-classes="text-primary-600 dark:text-primary-300 border-primary-600 dark:border-primary-300" data-tabs-inactive-classes="text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300 border-transparent hover:border-gray-300 dark:hover:border-gray-600">
                                <li class="me-2" role="presentation">
                                    <button class="inline-flex items-center gap-2 px-4 py-2.5 border-b-2 rounded-t-lg" id="tpl-visual-tab" data-tabs-target="#tpl-visual-panel" type="button" role="tab" aria-controls="tpl-visual-panel" aria-selected="true">
                                        <i class="fa-solid fa-wand-magic-sparkles text-indigo-500"></i>"Visual Editor (Templatical)"
                                    </button>
                                </li>
                                <li class="me-2" role="presentation">
                                    <button class="inline-flex items-center gap-2 px-4 py-2.5 border-b-2 rounded-t-lg" id="tpl-html-tab" data-tabs-target="#tpl-html-panel" type="button" role="tab" aria-controls="tpl-html-panel" aria-selected="false">
                                        <i class="fa-solid fa-code"></i>"HTML Source"
                                    </button>
                                </li>
                                <li class="me-2" role="presentation">
                                    <button class="inline-flex items-center gap-2 px-4 py-2.5 border-b-2 rounded-t-lg" id="tpl-text-tab" data-tabs-target="#tpl-text-panel" type="button" role="tab" aria-controls="tpl-text-panel" aria-selected="false">
                                        <i class="fa-solid fa-align-left"></i>"Plain-text Body"
                                    </button>
                                </li>
                            </ul>
                        </div>
                        <div id="template-editor-tab-content">
                            <div id="tpl-visual-panel" role="tabpanel" aria-labelledby="tpl-visual-tab" data-templatical-editor data-target-input="html_text">
                                <div class="templatical-inline-container w-full rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 overflow-hidden" style="height: 650px;"></div>
                            </div>
                            <div class="hidden" id="tpl-html-panel" role="tabpanel" aria-labelledby="tpl-html-tab">
                                <Textarea id="html_text" label="" rows=18 wysiwyg=true value=m.html_text.clone().unwrap_or_default()/>
                            </div>
                            <div class="hidden" id="tpl-text-panel" role="tabpanel" aria-labelledby="tpl-text-tab">
                                <Textarea id="plain_text" label="" rows=18 value=m.plain_text.clone().unwrap_or_default()/>
                            </div>
                        </div>
                    </div>
                    <div class="flex justify-end gap-3 pt-2">
                        <a href=back_url class="text-sm font-medium text-gray-600 dark:text-gray-400 hover:underline self-center">"Cancel"</a>
                        {if is_edit {
                            view! {
                                <a href=format!("/app/templates/{m_id}/preview") target="_blank" rel="noopener"
                                    class="text-sm font-medium text-gray-700 dark:text-gray-200 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-600 rounded-lg px-4 py-2.5">
                                    <i class="fa-regular fa-eye me-1"></i>"Preview"
                                </a>
                            }.into_any()
                        } else {
                            ().into_any()
                        }}
                        <button type="submit" class="text-white bg-primary-600 hover:bg-primary-700 font-medium rounded-lg text-sm px-5 py-2.5">
                            {if is_edit {"Save changes"} else {"Create template"}}
                        </button>
                    </div>
                </form>
            </div>
            {if is_edit {
                view! {
                    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-4 flex items-center justify-between">
                        <div>
                            <h3 class="text-sm font-semibold text-red-700 dark:text-red-400">"Delete template"</h3>
                            <p class="text-xs text-gray-500 dark:text-gray-400">"Removes the template — campaigns already created from it stay intact."</p>
                        </div>
                        <form method="POST" action=format!("/app/templates/{m_id}/delete?brand={brand_token}")
                            onsubmit="return confirm('Delete this template?');">
                            <button type="submit" class="text-white bg-red-600 hover:bg-red-700 font-medium rounded-lg text-sm px-4 py-2">"Delete"</button>
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
fn Field(
    #[prop(into)] id: String,
    #[prop(into)] label: String,
    #[prop(into, default = "text".to_string())] input_type: String,
    #[prop(into, optional)] value: String,
    #[prop(default = false)] required: bool,
) -> impl IntoView {
    let id_for = id.clone();
    view! {
        <div>
            <label for=id_for class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">{label}</label>
            <input type=input_type id=id.clone() name=id value=value required=required
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

// ── Handlers ────────────────────────────────────────────────────────────────

