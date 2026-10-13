//! Font Awesome 6 icon shortcuts for admin navigation.

use leptos::prelude::*;

pub fn fa_icon(class: &'static str) -> impl IntoView {
    view! {
        <i class=format!("fa-fw {} text-gray-500 dark:text-gray-400 transition duration-75 group-hover:text-gray-900 dark:group-hover:text-white", class)
           aria-hidden="true"></i>
    }
}

pub fn icon_lists() -> impl IntoView { fa_icon("fa-solid fa-list") }
pub fn icon_campaigns() -> impl IntoView { fa_icon("fa-solid fa-paper-plane") }
pub fn icon_templates() -> impl IntoView { fa_icon("fa-solid fa-clone") }
pub fn icon_rules() -> impl IntoView { fa_icon("fa-solid fa-bolt") }
pub fn icon_cog() -> impl IntoView { fa_icon("fa-solid fa-gear") }
