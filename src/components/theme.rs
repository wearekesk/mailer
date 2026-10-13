//! Theme initialization and dark/light mode toggle.

use leptos::prelude::*;

/// Inline JS that runs in `<head>` before paint. Reads the persisted theme
/// preference from `localStorage` (or `prefers-color-scheme` if not set) and
/// applies the `dark` class to `<html>` so dark-mode utilities fire on the
/// very first frame — no FOUC.
pub fn theme_init_js() -> &'static str {
    r#"
(function () {
    try {
        var stored = localStorage.getItem('theme');
        var prefersDark = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches;
        var dark = stored === 'dark' || (stored === null && prefersDark);
        if (dark) document.documentElement.classList.add('dark');
    } catch (_) {}
})();
"#
}

/// Inline JS that wires the topbar dark/light toggle button. Persists the
/// user's choice in `localStorage` so it survives page reloads.
pub fn theme_toggle_js() -> &'static str {
    r#"
(function () {
    var btn = document.getElementById('theme-toggle');
    if (!btn) return;
    var lightIcon = document.getElementById('theme-toggle-light-icon');
    var darkIcon = document.getElementById('theme-toggle-dark-icon');
    function sync() {
        var dark = document.documentElement.classList.contains('dark');
        if (lightIcon) lightIcon.classList.toggle('hidden', !dark);
        if (darkIcon) darkIcon.classList.toggle('hidden', dark);
    }
    sync();
    btn.addEventListener('click', function () {
        var dark = document.documentElement.classList.toggle('dark');
        try { localStorage.setItem('theme', dark ? 'dark' : 'light'); } catch (_) {}
        sync();
    });
})();
"#
}

#[component]
pub fn ThemeToggle() -> impl IntoView {
    view! {
        <button
            id="theme-toggle"
            type="button"
            class="text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 focus:outline-none focus:ring-4 focus:ring-gray-200 dark:focus:ring-gray-700 rounded-lg text-sm p-2"
            aria-label="Toggle dark mode"
        >
            <i id="theme-toggle-light-icon" class="hidden fa-solid fa-sun" aria-hidden="true"></i>
            <i id="theme-toggle-dark-icon" class="hidden fa-solid fa-moon" aria-hidden="true"></i>
        </button>
    }
}
