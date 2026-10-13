//! Authentication document and card shells.

use leptos::prelude::*;

#[component]
pub fn AuthDocument(
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
                <script>{crate::components::theme::theme_init_js()}</script>
            </head>
            <body class="h-full bg-gray-50/60 dark:bg-gray-950 text-gray-900 antialiased dark:text-gray-100 flex flex-col justify-between transition-colors">
                <header class="w-full px-6 py-4 flex justify-between items-center max-w-7xl mx-auto">
                    <a href="/" class="flex items-center gap-2.5 group">
                        <div class="w-8 h-8 rounded-lg bg-gray-900 dark:bg-white text-white dark:text-gray-900 flex items-center justify-center font-bold text-sm shadow-sm group-hover:scale-105 transition-transform">
                            <i class="fa-solid fa-paper-plane text-xs"></i>
                        </div>
                        <span class="text-lg font-bold tracking-tight text-gray-900 dark:text-white">"Mailer"</span>
                    </a>
                    <div class="flex items-center gap-2">
                        {crate::components::theme::ThemeToggle()}
                    </div>
                </header>

                <main class="flex-1 flex flex-col items-center justify-center px-4 py-8 sm:px-6">
                    {children()}
                </main>

                <footer class="w-full py-6 text-center text-xs text-gray-400 dark:text-gray-600">
                    <p>"© 2026 Mailer. Self-hosted email marketing platform."</p>
                </footer>

                <script type="module" src="/static/dist/mailer.js"></script>
                <script>{crate::components::theme::theme_toggle_js()}</script>
            </body>
        </html>
    }
}

/// Modern elevated card for login, 2FA, and password reset.
#[component]
pub fn AuthCard(
    #[prop(into)] heading: String,
    #[prop(into, optional)] subtitle: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="w-full sm:max-w-md bg-white dark:bg-gray-900/90 rounded-2xl shadow-xl shadow-gray-200/50 dark:shadow-none border border-gray-200/80 dark:border-gray-800 p-8 sm:p-10 backdrop-blur-sm transition-all">
            <div class="mb-6 text-center sm:text-left">
                <h1 class="text-2xl font-bold tracking-tight text-gray-900 dark:text-white">
                    {heading}
                </h1>
                {if !subtitle.is_empty() {
                    view! { <p class="text-sm text-gray-500 dark:text-gray-400 mt-1.5">{subtitle}</p> }.into_any()
                } else {
                    ().into_any()
                }}
            </div>
            {children()}
        </div>
    }
}
