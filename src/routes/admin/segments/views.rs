//! Segments admin pages.

use leptos::prelude::*;
use serde::Deserialize;

use crate::database::entities::seg;

#[component]
pub fn ListSegmentsPage(
    list_token: String,
    list_name: String,
    rows: Vec<seg::Model>,
) -> impl IntoView {
    view! {
        <section class="space-y-4">
            <header>
                <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">{list_name}</h1>
                <p class="text-sm text-gray-500 dark:text-gray-400">"Segments scoped to this list."</p>
            </header>
            <crate::components::layout::ListSubNav list_token=list_token.clone()/>

            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6">
                <form method="POST" action=format!("/app/lists/{list_token}/segments/new") class="flex gap-3 items-end flex-wrap">
                    <div class="grow">
                        <label for="name" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Segment name"</label>
                        <input type="text" id="name" name="name" required
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
                            <th class="px-4 py-3 w-1"></th>
                        </tr>
                    </thead>
                    <tbody>
                        {if rows.is_empty() {
                            view! { <tr><td colspan="2" class="text-center text-gray-400 dark:text-gray-500 py-10">"No segments yet."</td></tr> }.into_any()
                        } else {
                            let token = list_token.clone();
                            rows.into_iter().map(|s| {
                                let edit_href = format!("/app/segments/{}/edit", s.id);
                                let delete_action = format!("/app/segments/{}/delete?list={}", s.id, token);
                                view! {
                                    <tr class="bg-white dark:bg-gray-800 border-b">
                                        <td class="px-4 py-3 font-medium text-gray-900 dark:text-white">
                                            <a href=edit_href class="hover:underline">{s.name.unwrap_or_default()}</a>
                                        </td>
                                        <td class="px-4 py-3 text-right">
                                            <form method="POST" action=delete_action onsubmit="return confirm('Delete this segment?');">
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
pub fn SegmentsListPage(
    brand_token: String,
    brand_name: String,
    rows: Vec<seg::Model>,
) -> impl IntoView {
    view! {
        <section class="space-y-4">
            <header class="flex justify-between items-center">
                <div>
                    <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">"Segments"</h1>
                    <p class="text-sm text-gray-500 dark:text-gray-400">{format!("Brand: {brand_name}")}</p>
                </div>
            </header>
            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6">
                <form method="POST" action=format!("/app/brands/{brand_token}/segments/new") class="flex gap-3 items-end flex-wrap">
                    <div class="grow">
                        <label for="name" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Segment name"</label>
                        <input type="text" id="name" name="name" required
                            class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2.5"/>
                    </div>
                    <div>
                        <label for="list_id" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"List ID (numeric)"</label>
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
                            <th class="px-4 py-3">"List"</th>
                            <th class="px-4 py-3">"Last updated"</th>
                            <th class="px-4 py-3 w-1"></th>
                        </tr>
                    </thead>
                    <tbody>
                        {if rows.is_empty() {
                            view! { <tr><td colspan="4" class="text-center text-gray-400 dark:text-gray-500 py-10">"No segments defined yet."</td></tr> }.into_any()
                        } else {
                            rows.into_iter().map(|s| {
                                let action = format!("/app/segments/{}/delete?brand={}", s.id, brand_token);
                                let edit_href = format!("/app/segments/{}/edit", s.id);
                                view! {
                                    <tr class="bg-white dark:bg-gray-800 border-b">
                                        <td class="px-4 py-3 font-medium text-gray-900 dark:text-white">
                                            <a href=edit_href.clone() class="hover:underline">{s.name.unwrap_or_default()}</a>
                                        </td>
                                        <td class="px-4 py-3">{s.list.unwrap_or_default()}</td>
                                        <td class="px-4 py-3 text-xs text-gray-500 dark:text-gray-400">{s.last_updated.unwrap_or_default()}</td>
                                        <td class="px-4 py-3 text-right">
                                            <div class="flex gap-3 justify-end">
                                                <a href=edit_href class="text-blue-600 hover:underline text-sm">"Edit"</a>
                                                <form method="POST" action=action onsubmit="return confirm('Delete this segment?');">
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

// ── Edit page (conditions builder) ─────────────────────────────────────────

/// One row in the field-picker dropdown.
#[derive(Debug, Clone)]
pub struct CustomFieldOption {
    /// Display label and the value POSTed back as `field`.
    pub name: String,
    /// "Text" or "Date" — used to switch the value-input widget.
    pub kind: String,
}

#[component]
pub fn SegmentEditorPage(
    segment: crate::database::entities::seg::Model,
    conditions: Vec<crate::database::entities::seg_cons::Model>,
    brand_token: String,
    #[prop(default = Vec::new())] custom_fields: Vec<CustomFieldOption>,
) -> impl IntoView {
    let seg_id = segment.id;
    let name = segment.name.unwrap_or_default();
    let list_id = segment.list.unwrap_or_default();
    let back_href = format!("/app/brands/{brand_token}/segments");
    let csv_url = format!("/app/segments/{seg_id}/export.csv");
    // Distinct group ids already in use, ascending — feeds the
    // "Group #" select on the add-condition form.
    let mut existing_groups: Vec<i32> = conditions
        .iter()
        .map(|c| c.group_id.unwrap_or(0))
        .collect();
    existing_groups.sort();
    existing_groups.dedup();
    view! {
        <section class="space-y-4">
            <nav class="text-sm text-gray-500 dark:text-gray-400">
                <a href=back_href class="hover:underline">"← Segments"</a>
            </nav>
            <header class="flex justify-between items-end gap-3 flex-wrap">
                <div>
                    <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">{name}</h1>
                    <p class="text-sm text-gray-500 dark:text-gray-400">{format!("Segment #{seg_id} on list {list_id}. Conditions inside a group are AND-ed; groups are OR-ed.")}</p>
                </div>
                <a href=csv_url
                    class="text-sm font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 rounded-lg px-3 py-2">
                    "Export members CSV"
                </a>
            </header>

            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm overflow-x-auto">
                <table class="w-full text-sm text-left text-gray-500 dark:text-gray-400">
                    <thead class="text-xs text-gray-700 dark:text-gray-300 uppercase bg-gray-50 dark:bg-gray-700">
                        <tr>
                            <th class="px-4 py-3">"Group"</th>
                            <th class="px-4 py-3">"Operator"</th>
                            <th class="px-4 py-3">"Field"</th>
                            <th class="px-4 py-3">"Comparison"</th>
                            <th class="px-4 py-3">"Value"</th>
                            <th class="px-4 py-3 w-1"></th>
                        </tr>
                    </thead>
                    <tbody>
                        {if conditions.is_empty() {
                            view! { <tr><td colspan="6" class="text-center text-gray-400 dark:text-gray-500 py-10">"No conditions yet."</td></tr> }.into_any()
                        } else {
                            conditions.into_iter().map(|c| {
                                let action = format!("/app/segments/{seg_id}/conditions/{}/delete", c.id);
                                view! {
                                    <tr class="bg-white dark:bg-gray-800 border-b">
                                        <td class="px-4 py-3">{c.group_id.unwrap_or(0)}</td>
                                        <td class="px-4 py-3">{c.operator.unwrap_or_default()}</td>
                                        <td class="px-4 py-3 font-medium text-gray-900 dark:text-white">{c.field.unwrap_or_default()}</td>
                                        <td class="px-4 py-3">{c.comparison.unwrap_or_default()}</td>
                                        <td class="px-4 py-3 font-mono text-xs">{c.val.unwrap_or_default()}</td>
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

            <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-6">
                <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-1">"Add condition"</h2>
                <p class="text-xs text-gray-500 dark:text-gray-400 mb-3">
                    "Conditions inside the same Group # are AND-ed together; different Group #s are OR-ed."
                </p>
                <AddConditionForm seg_id=seg_id existing_conditions=existing_groups custom_fields=custom_fields/>
            </div>
        </section>
    }
}

#[component]
fn AddConditionForm(
    seg_id: i32,
    /// Distinct group ids already in use, ascending. Empty when the
    /// segment has no conditions yet.
    #[prop(into)]
    existing_conditions: Vec<i32>,
    custom_fields: Vec<CustomFieldOption>,
) -> impl IntoView {
    let max_group = existing_conditions.iter().copied().max().unwrap_or(-1);
    // Default to either the highest existing group (continue AND-ing into
    // it) or 0 if there's nothing yet.
    let default_group = max_group.max(0);
    // Suggested "new group" id for the helper button.
    let new_group = max_group + 1;
    let group_options: Vec<i32> = if existing_conditions.is_empty() {
        vec![0]
    } else {
        let mut v = existing_conditions.clone();
        v.push(new_group);
        v
    };

    let date_field_names: Vec<String> = custom_fields
        .iter()
        .filter(|f| f.kind.eq_ignore_ascii_case("date"))
        .map(|f| f.name.clone())
        .collect();
    let date_fields_json = serde_json::to_string(&date_field_names).unwrap_or_else(|_| "[]".into());

    view! {
        <form method="POST" action=format!("/app/segments/{seg_id}/conditions/add") class="grid grid-cols-1 md:grid-cols-5 gap-3">
            <div>
                <label for="group_id" class="block mb-1 text-xs font-medium text-gray-700 dark:text-gray-300">"Group #"</label>
                <select id="group_id" name="group_id" required
                    class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2">
                    {group_options.iter().map(|g| {
                        let label = if *g == new_group && max_group >= 0 {
                            format!("Group {g} (new — OR'd)")
                        } else {
                            format!("Group {g}")
                        };
                        let selected = *g == default_group;
                        view! { <option value=g.to_string() selected=selected>{label}</option> }
                    }).collect_view()}
                </select>
            </div>
            <div>
                <label for="operator" class="block mb-1 text-xs font-medium text-gray-700 dark:text-gray-300">"Operator"</label>
                <select id="operator" name="operator"
                    class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2">
                    <option value="AND" selected>"AND"</option>
                    <option value="OR">"OR"</option>
                </select>
            </div>
            <div>
                <label for="field" class="block mb-1 text-xs font-medium text-gray-700 dark:text-gray-300">"Field"</label>
                <select id="field" name="field"
                    class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2">
                    <optgroup label="Built-in">
                        <option value="email">"email"</option>
                        <option value="name">"name"</option>
                        <option value="country">"country"</option>
                        <option value="ip">"ip"</option>
                        <option value="referrer">"referrer"</option>
                        <option value="gdpr">"gdpr"</option>
                        <option value="timestamp">"timestamp"</option>
                        <option value="join_date">"join_date"</option>
                    </optgroup>
                    {if custom_fields.is_empty() {
                        ().into_any()
                    } else {
                        view! {
                            <optgroup label="Custom fields">
                                {custom_fields.into_iter().map(|f| view! {
                                    <option value=f.name.clone()>{format!("{} ({})", f.name, f.kind)}</option>
                                }).collect_view()}
                            </optgroup>
                        }.into_any()
                    }}
                </select>
            </div>
            <div>
                <label for="comparison" class="block mb-1 text-xs font-medium text-gray-700 dark:text-gray-300">"Comparison"</label>
                <select id="comparison" name="comparison"
                    class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2">
                    <option value="is">"is"</option>
                    <option value="is_not">"is not"</option>
                    <option value="contains">"contains"</option>
                    <option value="not_contains">"not contains"</option>
                    <option value="starts_with">"starts with"</option>
                    <option value="ends_with">"ends with"</option>
                    <option value="is_empty">"is empty"</option>
                    <option value="is_not_empty">"is not empty"</option>
                    <option value="greater_than">"greater than"</option>
                    <option value="less_than">"less than"</option>
                </select>
            </div>
            <div>
                <label for="val" class="block mb-1 text-xs font-medium text-gray-700 dark:text-gray-300">"Value"</label>
                <input type="text" id="val" name="val"
                    class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg block w-full p-2"/>
            </div>
            <div class="md:col-span-5">
                <button type="submit" class="text-white bg-primary-600 hover:bg-primary-700 font-medium rounded-lg text-sm px-4 py-2">"Add condition"</button>
            </div>
        </form>
        // Switch the value input to a date picker when the selected
        // field is a custom Date field, OR a built-in timestamp/join_date.
        <script>{format!(r#"
(function(){{
  var dateCustom = {date_fields_json};
  var f = document.getElementById('field');
  var v = document.getElementById('val');
  var c = document.getElementById('comparison');
  if (!f || !v) return;
  function sync() {{
    var name = f.value;
    var isBuiltinDate = (name === 'timestamp' || name === 'join_date');
    var isDateCustom  = dateCustom.indexOf(name) !== -1;
    v.type = (isBuiltinDate || isDateCustom) ? 'date' : 'text';
    if (c) {{
      var emptyOnly = (c.value === 'is_empty' || c.value === 'is_not_empty');
      v.disabled = emptyOnly;
    }}
  }}
  f.addEventListener('change', sync);
  if (c) c.addEventListener('change', sync);
  sync();
}})();
"#)}</script>
    }
}

// ── Handlers ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SegmentForm {
    pub name: String,
    pub list_id: i32,
}

#[derive(Debug, Deserialize)]
pub struct BrandQuery {
    pub brand: String,
}

