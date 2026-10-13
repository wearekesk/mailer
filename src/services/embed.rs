//! Subscribe-form embed generator.
//!
//! Generates subscription forms and integration snippets:
//!
//! - **Hosted form URL** — a stand-alone subscription page Mailer serves at
//!   `/subscribe/:list_token`, suitable for sharing as a link or embedding
//!   in an iframe.
//! - **Embeddable HTML form code** — a copy-pasteable `<form>` snippet
//!   posting to that same URL with name + email + per-list custom fields
//!   + GDPR consent checkbox + reCAPTCHA placeholder, ready to drop into
//!   any HTML page.
//!
//! Custom-domain handling: if the brand has a custom domain enabled, the
//! form action is rewritten to point at it (so subscribers don't see the
//! Mailer install URL). Falls back to the configured `app_url` otherwise.

use crate::database::entities::{apps, lists};
use crate::services::lists::{parse_custom_fields, CustomField};
use crate::utils::opaque_id::{List, OpaqueId};

/// Settings the embed code reflects, derived from the brand + list rows.
#[derive(Debug, Clone)]
pub struct EmbedSettings {
    pub list_id: i32,
    pub list_name: String,
    pub list_token: String,
    /// URL the form action posts to — accounts for custom-domain rewrite.
    pub form_action: String,
    /// Same URL but without the `/subscribe/...` suffix — the hosted page
    /// where end users land if you share the link directly.
    pub hosted_url: String,
    pub custom_fields: Vec<CustomField>,
    pub gdpr_enabled: bool,
    pub marketing_permission: String,
    pub what_to_expect: String,
    pub recaptcha_sitekey: Option<String>,
    pub from_email: String,
    pub app_name: String,
}

impl EmbedSettings {
    pub fn from_models(app: &apps::Model, list: &lists::Model, default_app_url: &str) -> Self {
        let list_id = list.id;
        let token = OpaqueId::<List>::new(list_id).to_string();
        let base = effective_base_url(app, default_app_url);
        let form_action = format!("{}/subscribe/{}", trim_slash(&base), token);
        let hosted_url = form_action.clone();
        let recap = app
            .recaptcha_sitekey
            .clone()
            .filter(|s| !s.trim().is_empty());
        EmbedSettings {
            list_id,
            list_name: list.name.clone().unwrap_or_default(),
            list_token: token,
            form_action,
            hosted_url,
            custom_fields: parse_custom_fields(list.custom_fields.as_deref().unwrap_or("")),
            gdpr_enabled: list.gdpr_enabled.unwrap_or(0) == 1,
            marketing_permission: list.marketing_permission.clone().unwrap_or_default(),
            what_to_expect: list.what_to_expect.clone().unwrap_or_default(),
            recaptcha_sitekey: recap,
            from_email: app.from_email.clone().unwrap_or_default(),
            app_name: app.app_name.clone().unwrap_or_default(),
        }
    }

    /// Build the embeddable `<form>` snippet — exactly what the user
    /// copy-pastes onto their own website.
    pub fn html_form_code(&self) -> String {
        let mut out = String::new();
        if self.recaptcha_sitekey.is_some() {
            out.push_str("<script src=\"https://www.google.com/recaptcha/api.js\"></script>\n");
        }
        out.push_str(&format!(
            "<form action=\"{action}\" method=\"POST\" accept-charset=\"utf-8\">\n",
            action = html_escape(&self.form_action),
        ));
        out.push_str("    <label for=\"name\">Name</label><br/>\n");
        out.push_str("    <input type=\"text\" name=\"name\" id=\"name\"/>\n");
        out.push_str("    <br/>\n");
        out.push_str("    <label for=\"email\">Email</label><br/>\n");
        out.push_str("    <input type=\"email\" name=\"email\" id=\"email\"/>\n");

        for cf in &self.custom_fields {
            let id = cf.name.replace(' ', "");
            out.push_str("    <br/>\n");
            out.push_str(&format!(
                "    <label for=\"{id}\">{label}</label><br/>\n",
                id = html_escape(&id),
                label = html_escape(&cf.name),
            ));
            let input_type = match cf.kind {
                crate::services::lists::CustomFieldType::Date => "date",
                _ => "text",
            };
            out.push_str(&format!(
                "    <input type=\"{ty}\" name=\"{id}\" id=\"{id}\"/>\n",
                ty = input_type,
                id = html_escape(&id),
            ));
        }

        if self.gdpr_enabled {
            let perm = if self.marketing_permission.trim().is_empty() {
                format!(
                    "I give my consent to {app} to be in touch with me via email using the information I have provided in this form for the purpose of news, updates and marketing.",
                    app = html_escape(&self.app_name),
                )
            } else {
                html_escape(&self.marketing_permission)
            };
            let what = if self.what_to_expect.trim().is_empty() {
                format!(
                    "If you wish to withdraw your consent and stop hearing from us, simply click the unsubscribe link at the bottom of every email we send or contact us at {email}. We value and respect your personal data and privacy.",
                    email = html_escape(&self.from_email),
                )
            } else {
                html_escape(&self.what_to_expect)
            };
            out.push_str("    <br/><br/>\n");
            out.push_str("    <input type=\"checkbox\" name=\"gdpr\" id=\"gdpr\"/>\n");
            out.push_str(&format!(
                "    <span><strong>Marketing permission</strong>: {perm}</span>\n"
            ));
            out.push_str("    <br/><br/>\n");
            out.push_str(&format!(
                "    <span><strong>What to expect</strong>: {what}</span>\n"
            ));
            out.push_str("    <br/>\n");
        }

        out.push_str("    <br/>\n");
        if let Some(key) = &self.recaptcha_sitekey {
            out.push_str(&format!(
                "    <p class=\"g-recaptcha\" data-sitekey=\"{key}\"></p>\n",
                key = html_escape(key),
            ));
        }
        out.push_str("    <div style=\"display:none;\">\n");
        out.push_str("        <label for=\"hp\">HP</label><br/>\n");
        out.push_str("        <input type=\"text\" name=\"hp\" id=\"hp\"/>\n");
        out.push_str("    </div>\n");
        out.push_str("    <input type=\"submit\" name=\"submit\" id=\"submit\" value=\"Subscribe\"/>\n");
        out.push_str("</form>\n");
        out
    }

    /// Iframe snippet for embedding the hosted page directly.
    pub fn iframe_code(&self) -> String {
        format!(
            "<iframe src=\"{src}\" width=\"100%\" height=\"480\" frameborder=\"0\" style=\"border:0;\" loading=\"lazy\"></iframe>\n",
            src = html_escape(&self.hosted_url),
        )
    }
}

fn effective_base_url(app: &apps::Model, default_app_url: &str) -> String {
    if app.custom_domain_enabled.unwrap_or(0) == 1 {
        let host = app.custom_domain.clone().unwrap_or_default();
        if !host.trim().is_empty() {
            let proto = app
                .custom_domain_protocol
                .clone()
                .unwrap_or_else(|| "https".into());
            let proto = if proto.is_empty() { "https".into() } else { proto };
            return format!("{}://{}", proto.trim_end_matches("://"), host.trim_matches('/'));
        }
    }
    default_app_url.to_string()
}

fn trim_slash(s: &str) -> &str {
    s.trim_end_matches('/')
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
