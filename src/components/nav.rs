//! Navigation helpers and sub-navigation bars.

use leptos::prelude::*;

use crate::components::admin::AdminChrome;

/// Reusable sidebar link displaying active status.
#[component]
pub fn SidebarLink(
    #[prop(into)] href: String,
    #[prop(into)] label: String,
    children: Children,
) -> impl IntoView {
    let chrome = use_context::<AdminChrome>().unwrap_or_default();
    let active = path_matches(&chrome.current_path, &href);
    let class = if active {
        "flex items-center p-2 rounded-lg group bg-primary-50 text-primary-700 dark:bg-primary-900/40 dark:text-primary-300"
    } else {
        "flex items-center p-2 rounded-lg group text-gray-900 dark:text-white hover:bg-gray-100 dark:hover:bg-gray-700"
    };
    view! {
        <li>
            <a href=href>
                <span class=class>
                    {children()}
                    <span class="ms-3">{label}</span>
                </span>
            </a>
        </li>
    }
}

/// Tabbed sub-nav rendered at the top of every list-scoped page
/// (subscribers, autoresponders, segments, housekeeping, edit).
#[component]
pub fn ListSubNav(
    /// Opaque list token (not the raw id).
    #[prop(into)]
    list_token: String,
) -> impl IntoView {
    let chrome = use_context::<AdminChrome>().unwrap_or_default();
    let active_class = "px-3 py-2 text-sm font-medium border-b-2 border-primary-600 text-primary-700 dark:text-primary-300";
    let inactive_class = "px-3 py-2 text-sm font-medium border-b-2 border-transparent text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white hover:border-gray-300 dark:hover:border-gray-600";
    let tab = move |href: String, label: &'static str| {
        let active = path_matches(&chrome.current_path, &href);
        let class = if active { active_class } else { inactive_class };
        view! { <a href=href class=class>{label}</a> }
    };
    view! {
        <nav class="flex flex-wrap border-b border-gray-200 dark:border-gray-700 -mt-2 mb-4">
            {tab(format!("/app/subscribers/{list_token}"), "Subscribers")}
            {tab(format!("/app/lists/{list_token}/autoresponders"), "Autoresponders")}
            {tab(format!("/app/lists/{list_token}/segments"), "Segments")}
            {tab(format!("/app/lists/{list_token}/embed"), "Embed form")}
            {tab(format!("/app/lists/{list_token}/housekeeping"), "Housekeeping")}
            {tab(format!("/app/lists/{list_token}/edit"), "Settings")}
        </nav>
    }
}

/// True when `current` matches a sidebar link's `href`.
pub fn path_matches(current: &str, href: &str) -> bool {
    let current = current.split('?').next().unwrap_or(current);
    if current == href {
        return true;
    }
    if href == "/app/brands" {
        return current == "/app/brands" || current == "/app/brands/new";
    }
    if href.starts_with("/app/brands/") {
        return current.starts_with(href);
    }
    current.starts_with(href)
}
