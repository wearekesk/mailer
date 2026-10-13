//! Internationalization scaffolding — Fluent-based, via poem 3's built-in
//! `i18n` feature.
//!
//! Fluent files live under `i18n/<lang-id>/*.ftl`. Add new keys to
//! the matching .ftl file (start with `en-US/app.ftl`) and read them in a
//! handler via `Locale::text("key")`.
//!
//! Wiring (already done in `routes::build`):
//!
//! ```ignore
//! use poem::EndpointExt;
//! let app = router.data(services::i18n::resources());
//! ```
//!
//! poem 3.1 ships no `I18NMiddleware` — the `Locale` extractor pulls the
//! `I18NResources` out of request `Data`, so attaching via `.data(…)` is
//! the supported wiring.
//!
//! Usage in a handler:
//!
//! ```ignore
//! use poem::{handler, i18n::Locale};
//!
//! #[handler]
//! async fn page(locale: Locale) -> String {
//!     locale.text("login-title").unwrap_or_else(|_| "Sign in".into())
//! }
//! ```
//!
//! The `Locale` extractor reads `Accept-Language` and negotiates against
//! the available languages, falling back to the configured default.

use poem::i18n::I18NResources;

/// Build the [`I18NResources`] bundle. Loads every `.ftl` file under
/// `i18n/` (relative to the working directory) at startup. Default
/// language is `en-US`; new languages just need a sibling directory.
pub fn resources() -> I18NResources {
    I18NResources::builder()
        .add_path("./i18n")
        .default_language(
            "en-US"
                .parse()
                .expect("en-US is a valid language identifier"),
        )
        .build()
        .expect("i18n resources failed to build (check ./i18n/*.ftl files)")
}
