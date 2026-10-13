//! Authenticated administrative application shell and layout.

use leptos::prelude::*;

use crate::components::icons::*;
use crate::components::nav::*;
use crate::components::theme::*;

/// Per-page admin chrome — the brand selector + "currently viewing" brand.
#[derive(Debug, Clone, Default)]
pub struct AdminChrome {
    pub brands: Vec<ChromeBrand>,
    pub current: Option<ChromeBrand>,
    pub current_path: String,
}

#[derive(Debug, Clone)]
pub struct ChromeBrand {
    pub id: i32,
    pub token: String,
    pub name: String,
    pub restriction: Option<BrandRestriction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrandRestriction {
    CampaignsOnly,
    TemplatesOnly,
    ListsOnly,
}

/// HTML document used by every authenticated admin page.
#[component]
pub fn AdminDocument(
    #[prop(into)] title: String,
    children: Children,
) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en" class="h-full">
            <head>
                <meta charset="UTF-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1.0"/>
                <title>{title}" — Mailer"</title>
                <link rel="stylesheet" href="/static/dist/mailer.css"/>
                <script>{theme_init_js()}</script>
            </head>
            <body class="h-full bg-gray-50 text-gray-900 antialiased dark:bg-gray-900 dark:text-gray-100">
                {children()}
                <Footer/>
                <script type="module" src="/static/dist/mailer.js"></script>
                <script>{theme_toggle_js()}</script>
            </body>
        </html>
    }
}

/// Application shell with top navbar, brand switcher, and sidebar.
#[component]
pub fn AdminLayout(children: Children) -> impl IntoView {
    let chrome = use_context::<AdminChrome>().unwrap_or_default();
    let with_sidebar = is_brand_scoped(&chrome.current_path);
    let main_class = if with_sidebar {
        "p-4 md:ml-64 h-auto pt-20"
    } else {
        "p-4 h-auto pt-20 max-w-7xl mx-auto"
    };
    view! {
        <div class="antialiased bg-gray-50 dark:bg-gray-900 min-h-screen">
            <Topbar/>
            {if with_sidebar {
                view! { <Sidebar/> }.into_any()
            } else {
                ().into_any()
            }}
            <main class=main_class>
                {children()}
            </main>
        </div>
    }
}

#[component]
fn Topbar() -> impl IntoView {
    view! {
        <nav class="bg-white/80 dark:bg-gray-900/80 backdrop-blur-md border-b border-gray-200/80 dark:border-gray-800 px-4 py-2.5 fixed left-0 right-0 top-0 z-50 transition-colors">
            <div class="flex flex-wrap justify-between items-center w-full">
                <div class="flex items-center justify-start gap-2 sm:gap-4">
                    <button
                        data-drawer-target="logo-sidebar"
                        data-drawer-toggle="logo-sidebar"
                        aria-controls="logo-sidebar"
                        type="button"
                        class="inline-flex items-center p-2 text-sm text-gray-500 rounded-lg md:hidden hover:bg-gray-100 dark:text-gray-400 dark:hover:bg-gray-800 focus:outline-none focus:ring-2 focus:ring-gray-200 dark:focus:ring-gray-700"
                    >
                        <span class="sr-only">"Open sidebar"</span>
                        <i class="fa-solid fa-bars text-lg" aria-hidden="true"></i>
                    </button>
                    <a href="/app/brands" class="flex items-center gap-2.5 group">
                        <div class="w-8 h-8 rounded-lg bg-gray-900 dark:bg-white text-white dark:text-gray-900 flex items-center justify-center font-bold text-base shadow-sm group-hover:scale-105 transition-transform">
                            <i class="fa-solid fa-paper-plane text-xs"></i>
                        </div>
                        <span class="self-center text-lg font-bold tracking-tight text-gray-900 dark:text-white">"Mailer"</span>
                    </a>
                    <BrandSwitcher/>
                </div>
                <div class="flex items-center gap-3">
                    <a href="/app/brands" class="hidden sm:inline-flex items-center gap-1.5 text-xs font-medium text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white px-2.5 py-1.5 rounded-md hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors">
                        <i class="fa-solid fa-layer-group text-xs"></i>"Brands"
                    </a>
                    <a href="/app/settings" class="hidden sm:inline-flex items-center gap-1.5 text-xs font-medium text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white px-2.5 py-1.5 rounded-md hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors">
                        <i class="fa-solid fa-gear text-xs"></i>"Settings"
                    </a>
                    <div class="h-4 w-px bg-gray-200 dark:bg-gray-800 hidden sm:block"></div>
                    <ThemeToggle/>
                    <button
                        type="button"
                        class="flex items-center gap-2 p-1 text-sm rounded-full focus:ring-4 focus:ring-gray-200 dark:focus:ring-gray-800"
                        aria-expanded="false"
                        data-dropdown-toggle="dropdown-user"
                    >
                        <span class="sr-only">"Open user menu"</span>
                        <span class="flex w-8 h-8 items-center justify-center rounded-full bg-zinc-800 dark:bg-zinc-700 text-white text-xs font-semibold shadow-inner border border-zinc-700 dark:border-zinc-600">
                            "A"
                        </span>
                    </button>
                    <div
                        class="z-50 hidden my-2 text-base list-none bg-white dark:bg-gray-900 divide-y divide-gray-100 dark:divide-gray-800 rounded-xl shadow-xl border border-gray-100 dark:border-gray-800 min-w-48"
                        id="dropdown-user"
                    >
                        <div class="px-4 py-3" role="none">
                            <p class="text-xs font-semibold text-gray-900 dark:text-white" role="none">"Administrator"</p>
                            <p class="text-xs text-gray-500 dark:text-gray-400 truncate" role="none">"admin@mailer.local"</p>
                        </div>
                        <ul class="py-1" role="none">
                            <li>
                                <a href="/app/settings" class="flex items-center gap-2 px-4 py-2 text-xs font-medium text-gray-700 hover:bg-gray-50 dark:text-gray-300 dark:hover:bg-gray-800/60 dark:hover:text-white">
                                    <i class="fa-solid fa-sliders text-xs w-4"></i>"Account settings"
                                </a>
                            </li>
                            <li>
                                <a href="/logout" class="flex items-center gap-2 px-4 py-2 text-xs font-medium text-red-600 hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-950/30">
                                    <i class="fa-solid fa-arrow-right-from-bracket text-xs w-4"></i>"Sign out"
                                </a>
                            </li>
                        </ul>
                    </div>
                </div>
            </div>
        </nav>
    }
}

#[component]
fn BrandSwitcher() -> impl IntoView {
    let chrome = use_context::<AdminChrome>().unwrap_or_default();
    if chrome.brands.is_empty() || !is_brand_scoped(&chrome.current_path) {
        return ().into_any();
    }
    let current_label = chrome
        .current
        .as_ref()
        .map(|b| b.name.clone())
        .unwrap_or_else(|| "Select brand".to_string());

    let items = chrome
        .brands
        .iter()
        .map(|b| {
            let is_current = chrome.current.as_ref().is_some_and(|c| c.id == b.id);
            let class = if is_current {
                "flex items-center justify-between px-3.5 py-2 text-xs font-semibold bg-gray-100 dark:bg-gray-800 text-gray-900 dark:text-white rounded-md mx-1"
            } else {
                "flex items-center justify-between px-3.5 py-2 text-xs text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800/60 dark:hover:text-white rounded-md mx-1"
            };
            view! {
                <li>
                    <a href=format!("/app/brands/{}/select", b.token) class=class>
                        <span>{b.name.clone()}</span>
                        {if is_current {
                            view! { <i class="fa-solid fa-check text-xs text-primary-500"></i> }.into_any()
                        } else {
                            ().into_any()
                        }}
                    </a>
                </li>
            }
        })
        .collect_view();

    view! {
        <div class="hidden sm:block">
            <button
                id="brand-switcher-button"
                data-dropdown-toggle="brand-switcher-menu"
                class="inline-flex items-center gap-2 px-3 py-1.5 text-xs font-semibold text-gray-800 bg-gray-100 hover:bg-gray-200 dark:text-gray-200 dark:bg-gray-800 dark:hover:bg-gray-700 rounded-lg border border-gray-200/60 dark:border-gray-700/60 shadow-xs focus:outline-none"
                type="button"
            >
                <i class="fa-solid fa-building text-gray-400 dark:text-gray-500 text-xs"></i>
                <span>{current_label}</span>
                <i class="fa-solid fa-chevron-down text-[10px] text-gray-400 dark:text-gray-500" aria-hidden="true"></i>
            </button>
            <div id="brand-switcher-menu" class="z-50 hidden bg-white dark:bg-gray-900 divide-y divide-gray-100 dark:divide-gray-800 rounded-xl shadow-xl border border-gray-100 dark:border-gray-800 w-60 py-1">
                <div class="px-3.5 py-2">
                    <p class="text-[10px] uppercase font-bold tracking-wider text-gray-400 dark:text-gray-500">"Switch brand"</p>
                </div>
                <ul class="py-1 max-h-72 overflow-y-auto space-y-0.5" aria-labelledby="brand-switcher-button">
                    {items}
                </ul>
                <div class="p-1.5">
                    <a href="/app/brands" class="flex items-center gap-2 px-3 py-1.5 text-xs text-gray-600 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-800 rounded-md font-medium">
                        <i class="fa-solid fa-list-check text-xs"></i>"Manage all brands"
                    </a>
                </div>
            </div>
        </div>
    }
    .into_any()
}

#[component]
fn Sidebar() -> impl IntoView {
    let chrome = use_context::<AdminChrome>().unwrap_or_default();
    let Some(b) = chrome.current.as_ref() else {
        return ().into_any();
    };
    let token = b.token.clone();
    let name = b.name.clone();
    let back_url = "/app/brands".to_string();
    let restriction = b.restriction;
    let show = |target: BrandRestriction| {
        restriction.map(|r| r == target).unwrap_or(true)
    };
    let unrestricted = restriction.is_none();

    view! {
        <aside
            id="logo-sidebar"
            class="fixed top-0 left-0 z-40 w-64 h-screen pt-14 transition-transform -translate-x-full bg-white border-r border-gray-200/80 md:translate-x-0 dark:bg-gray-900 dark:border-gray-800"
            aria-label="Sidebar"
        >
            <div class="h-full px-3 pb-4 overflow-y-auto bg-white dark:bg-gray-900">
                <a href=back_url class="flex items-center px-3 py-2 mt-3 text-xs text-gray-500 dark:text-gray-400 hover:underline">
                    <i class="fa-solid fa-chevron-left me-2" aria-hidden="true"></i>
                    "All brands"
                </a>
                <p class="mt-2 px-3 text-[11px] uppercase tracking-wider text-gray-400 dark:text-gray-500 truncate">{name}</p>
                <ul class="mt-2 space-y-1.5 font-medium">
                    {if show(BrandRestriction::ListsOnly) {
                        Some(view! { <SidebarLink href=format!("/app/brands/{token}/lists") label="Lists">{icon_lists()}</SidebarLink> })
                    } else { None }}
                    {if show(BrandRestriction::CampaignsOnly) {
                        Some(view! { <SidebarLink href=format!("/app/brands/{token}/campaigns") label="Campaigns">{icon_campaigns()}</SidebarLink> })
                    } else { None }}
                    {if show(BrandRestriction::TemplatesOnly) {
                        Some(view! { <SidebarLink href=format!("/app/brands/{token}/templates") label="Templates">{icon_templates()}</SidebarLink> })
                    } else { None }}
                    {if unrestricted {
                        Some(view! {
                            <SidebarLink href=format!("/app/brands/{token}/rules") label="Rules">{icon_rules()}</SidebarLink>
                            <SidebarLink href=format!("/app/brands/{token}/suppression-list") label="Suppression list">
                                <i class="fa-solid fa-ban text-xs w-4"></i>
                            </SidebarLink>
                            <SidebarLink href=format!("/app/brands/{token}/blocked-domains") label="Blocked domains">
                                <i class="fa-solid fa-globe text-xs w-4"></i>
                            </SidebarLink>
                            <SidebarLink href=format!("/app/brands/{token}/edit") label="Brand settings">{icon_cog()}</SidebarLink>
                        })
                    } else { None }}
                </ul>
                {if !unrestricted {
                    Some(view! {
                        <p class="mt-3 px-3 text-xs text-amber-600 dark:text-amber-400">
                            "Sub-user mode — restricted view. Ask the brand owner to lift the restriction in Brand settings."
                        </p>
                    })
                } else {
                    None
                }}
            </div>
        </aside>
    }
    .into_any()
}

#[component]
fn Footer() -> impl IntoView {
    let chrome = use_context::<AdminChrome>().unwrap_or_default();
    let class = if is_brand_scoped(&chrome.current_path) {
        "ml-0 md:ml-64 mt-12 mb-6 mx-4 border-t border-gray-200/80 dark:border-gray-800 pt-6 flex flex-col sm:flex-row items-center justify-between gap-4 text-xs text-gray-500 dark:text-gray-400"
    } else {
        "max-w-7xl mx-auto mt-12 mb-6 px-4 border-t border-gray-200/80 dark:border-gray-800 pt-6 flex flex-col sm:flex-row items-center justify-between gap-4 text-xs text-gray-500 dark:text-gray-400"
    };
    view! {
        <footer class=class>
            <div class="flex items-center gap-2">
                <span class="font-bold text-gray-800 dark:text-gray-200">"Mailer"</span>
                <span>"•"</span>
                <span>"Self-hosted email marketing engine"</span>
            </div>
            <div class="flex items-center gap-4">
                <a href="/app/settings" class="hover:text-gray-900 dark:hover:text-white transition-colors">"Settings"</a>
                <a href="https://github.com/wearekesk/mailer" target="_blank" rel="noopener"
                   class="inline-flex items-center gap-1.5 hover:text-gray-900 dark:hover:text-white transition-colors">
                    <i class="fa-brands fa-github text-sm" aria-hidden="true"></i>
                    <span>"GitHub"</span>
                </a>
            </div>
        </footer>
    }
}

pub fn is_brand_scoped(path: &str) -> bool {
    let p = path.split('?').next().unwrap_or(path);
    if p == "/app/brands"
        || p == "/app/brands/"
        || p == "/app/brands/new"
        || p.starts_with("/app/settings")
    {
        return false;
    }
    if p.starts_with("/app/brands/") {
        return true;
    }
    p.starts_with("/app/lists/")
        || p.starts_with("/app/campaigns/")
        || p.starts_with("/app/templates/")
        || p.starts_with("/app/segments/")
        || p.starts_with("/app/autoresponders/")
        || p.starts_with("/app/autoresponder-emails/")
        || p.starts_with("/app/subscribers/")
        || p.starts_with("/app/rules/")
}
