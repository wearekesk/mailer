//! Personalization tags reference card.

use leptos::prelude::*;

/// Reference card listing every personalization tag recognized by the template renderer.
#[component]
pub fn PersonalizationTagsCard() -> impl IntoView {
    let tags: &[(&str, &str)] = &[
        ("[Name]", "Subscriber's first name (or blank)"),
        ("[Email]", "Subscriber's email address"),
        ("[Field, fallback=value]", "Custom field by name; uses fallback when blank"),
        ("[unsubscribe]", "URL — one-click unsubscribe link for this list"),
        ("<unsubscribe>label</unsubscribe>", "Wraps any text/HTML in an unsubscribe link"),
        ("[preferences]", "URL — subscription preferences page (toggle lists)"),
        ("[webversion]", "URL — view-this-email-in-browser link"),
        ("<webversion>label</webversion>", "Wraps any text/HTML in a webversion link"),
        ("[confirmation_link]", "URL — only valid in the double-opt-in confirmation email body"),
        ("[currentday]", "Today's weekday name (Monday, Tuesday…)"),
        ("[currentdaynumber]", "Today's day-of-month, zero-padded (01–31)"),
        ("[currentmonth]", "Current month name (January, February…)"),
        ("[currentmonthnumber]", "Current month number, zero-padded (01–12)"),
        ("[currentyear]", "Current year (e.g. 2026)"),
    ];
    view! {
        <details class="rounded-lg border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-700/40 p-3 text-sm">
            <summary class="cursor-pointer font-medium text-gray-700 dark:text-gray-300">
                <i class="fa-solid fa-tags me-2 text-gray-500 dark:text-gray-400" aria-hidden="true"></i>
                "Personalization tags reference"
            </summary>
            <div class="mt-3 overflow-x-auto">
                <table class="w-full text-xs">
                    <thead class="text-[10px] uppercase tracking-wider text-gray-500 dark:text-gray-400">
                        <tr><th class="text-left py-1.5 pe-4">"Tag"</th><th class="text-left py-1.5">"Resolves to"</th></tr>
                    </thead>
                    <tbody>
                        {tags.iter().map(|(t, d)| view! {
                            <tr class="border-t border-gray-200 dark:border-gray-600">
                                <td class="py-1.5 pe-4 align-top"><code class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded px-1.5 py-0.5">{*t}</code></td>
                                <td class="py-1.5 text-gray-600 dark:text-gray-400">{*d}</td>
                            </tr>
                        }).collect_view()}
                    </tbody>
                </table>
            </div>
        </details>
    }
}
