//! Personalization: render `[Tag]`, `[Tag, fallback=…]`, `<unsubscribe>`,
//! `[unsubscribe]`, `[webversion]`, and date tags into outbound email
//! HTML/plain. Click-tracking link rewrite lives here too.

use std::collections::HashMap;
use std::sync::OnceLock;

use chrono::Datelike;
use regex::Regex;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::database::entities::{links, subscribers as subscribers_entity};
use crate::services::lists::{parse_custom_fields, CustomFieldType};
use crate::utils::errors::AppResult;
use crate::utils::opaque_id::{Campaign, Link, List, OpaqueId, Subscriber};

/// Per-recipient context used when rendering a campaign body.
#[derive(Debug, Clone, Default)]
pub struct Context {
    pub subscriber_id: i32,
    pub list_id: i32,
    pub campaign_id: i32,
    pub email: String,
    pub name: String,
    /// Decoded custom-field values keyed by case-insensitive field name.
    pub custom_fields: HashMap<String, String>,
    /// Custom-field types for date formatting.
    pub custom_field_types: HashMap<String, CustomFieldType>,
    /// Base URL (e.g. `https://mailer.example.com`) used to build tracking
    /// + unsubscribe links.
    pub app_url: String,
}

impl Context {
    /// Build a `Context` from raw model + list strings.
    pub fn build(
        subscriber: &subscribers_entity::Model,
        list_id: i32,
        campaign_id: i32,
        list_custom_fields: &str,
        app_url: impl Into<String>,
    ) -> Self {
        let names: Vec<_> = parse_custom_fields(list_custom_fields);
        let values: Vec<&str> = subscriber
            .custom_fields
            .as_deref()
            .unwrap_or("")
            .split("%s%")
            .collect();

        let mut custom_fields = HashMap::new();
        let mut custom_field_types = HashMap::new();
        for (idx, cf) in names.iter().enumerate() {
            let v = values.get(idx).copied().unwrap_or("").to_string();
            custom_fields.insert(cf.name.to_ascii_lowercase(), v);
            custom_field_types.insert(cf.name.to_ascii_lowercase(), cf.kind);
        }

        Context {
            subscriber_id: subscriber.id,
            list_id,
            campaign_id,
            email: subscriber.email.clone().unwrap_or_default(),
            name: subscriber.name.clone().unwrap_or_default(),
            custom_fields,
            custom_field_types,
            app_url: app_url.into(),
        }
    }

    /// Render an `[unsubscribe]` URL pointing at our public endpoint.
    pub fn unsubscribe_url(&self) -> String {
        format!(
            "{}/unsubscribe/{}/{}",
            trim_slash(&self.app_url),
            OpaqueId::<List>::new(self.list_id),
            OpaqueId::<Subscriber>::new(self.subscriber_id),
        )
    }

    pub fn webversion_url(&self) -> String {
        format!(
            "{}/w/{}/{}",
            trim_slash(&self.app_url),
            OpaqueId::<Campaign>::new(self.campaign_id),
            OpaqueId::<Subscriber>::new(self.subscriber_id),
        )
    }

    /// Render a `[preferences]` URL — points at the brand-wide subscription
    /// preferences page so the subscriber can toggle which lists they're on.
    pub fn preferences_url(&self) -> String {
        format!(
            "{}/subscription/{}/{}",
            trim_slash(&self.app_url),
            OpaqueId::<List>::new(self.list_id),
            OpaqueId::<Subscriber>::new(self.subscriber_id),
        )
    }
}

fn trim_slash(s: &str) -> &str {
    s.trim_end_matches('/')
}

/// Render a body with all personalization tags substituted.
///
/// Substitutions performed:
/// 1. `[Email]`, `[Name]` → subscriber values
/// 2. `[Field, fallback=value]` → custom field value or fallback
/// 3. `[Field]` (alone) → custom field value or empty string
/// 4. `[unsubscribe]` → URL
/// 5. `<unsubscribe>…</unsubscribe>` → `<a href="…">…</a>`
/// 6. `[webversion]` → URL
/// 7. `[currentday]`, `[currentmonth]`, `[currentyear]`,
///    `[currentdaynumber]`, `[currentmonthnumber]` → today's values
pub fn render(body: &str, ctx: &Context) -> String {
    let mut out = body.to_string();

    // Date tags first (cheap, no regex).
    let now = chrono::Local::now();
    out = out
        .replace("[currentdaynumber]", &format!("{:02}", now.day()))
        .replace("[currentday]", weekday_name(now.weekday()))
        .replace("[currentmonthnumber]", &format!("{:02}", now.month()))
        .replace("[currentmonth]", month_name(now.month()))
        .replace("[currentyear]", &now.year().to_string());

    // [Tag, fallback=...] — must come before the bare-tag pass.
    out = fallback_re()
        .replace_all(&out, |caps: &regex::Captures| {
            let field = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            let fallback = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            substitute_field(field, ctx).unwrap_or_else(|| fallback.to_string())
        })
        .into_owned();

    // <unsubscribe>label</unsubscribe> → <a href="…">label</a>
    out = unsub_html_re()
        .replace_all(&out, |caps: &regex::Captures| {
            let inner = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            format!(r#"<a href="{}">{inner}</a>"#, ctx.unsubscribe_url())
        })
        .into_owned();

    // Bare [Tag] — Email/Name first, then custom fields, then known special tags.
    out = bare_re()
        .replace_all(&out, |caps: &regex::Captures| {
            let raw = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            substitute_field(raw, ctx).unwrap_or_else(|| caps[0].to_string())
        })
        .into_owned();

    // If the template is written in MJML, compile it to responsive HTML.
    if out.contains("<mjml") || out.contains("<mj-") {
        if let Ok(root) = mrml::parse(&out) {
            let opts = mrml::prelude::render::RenderOptions::default();
            if let Ok(html) = root.element.render(&opts) {
                return html;
            }
        }
    }

    out
}

fn substitute_field(field: &str, ctx: &Context) -> Option<String> {
    match field.to_ascii_lowercase().as_str() {
        "email" => Some(ctx.email.clone()),
        "name" => Some(ctx.name.clone()),
        "unsubscribe" => Some(ctx.unsubscribe_url()),
        "webversion" => Some(ctx.webversion_url()),
        "preferences" => Some(ctx.preferences_url()),
        other => match ctx.custom_fields.get(other) {
            Some(v) => Some(format_custom_field(other, v, ctx)),
            None => None,
        },
    }
}

fn format_custom_field(key: &str, raw: &str, ctx: &Context) -> String {
    if raw.is_empty() {
        return String::new();
    }
    match ctx.custom_field_types.get(key) {
        Some(CustomFieldType::Date) => {
            // Date custom fields are stored as Unix epochs.
            if let Ok(ts) = raw.parse::<i64>() {
                if let Some(dt) = chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0) {
                    return dt.format("%b %d, %Y").to_string();
                }
            }
            raw.to_string()
        }
        _ => raw.to_string(),
    }
}

// ── Click-tracking link rewrite ─────────────────────────────────────────────

/// Walk every `<a href="…">` URL in `html`, insert a row into the `links`
/// table for each unique URL (so the campaign can later count clicks),
/// then rewrite the href to point at our `/t/:link/:subscriber/:campaign`
/// tracker which redirects to the original URL.
///
/// Called once per (campaign × subscriber) pair by the scheduler. The first
/// time a campaign is sent it pre-creates the link rows; subsequent
/// recipients reuse those rows.
pub async fn rewrite_links_for_send(
    db: &DatabaseConnection,
    html: &str,
    ctx: &Context,
) -> AppResult<String> {
    let urls = extract_hrefs(html);

    // Map original URL -> link id (cached per call).
    let mut url_to_link: HashMap<String, i32> = HashMap::new();
    for url in &urls {
        if !is_trackable(url) {
            continue;
        }
        let existing = links::Entity::find()
            .filter(links::Column::CampaignId.eq(ctx.campaign_id))
            .filter(links::Column::Link.eq(url))
            .one(db)
            .await?;
        let id = match existing {
            Some(l) => l.id,
            None => {
                let active = links::ActiveModel {
                    campaign_id: Set(Some(ctx.campaign_id)),
                    link: Set(Some(url.clone())),
                    clicks: Set(Some(String::new())),
                    ..Default::default()
                };
                active.insert(db).await?.id
            }
        };
        url_to_link.insert(url.clone(), id);
    }

    let app_url = trim_slash(&ctx.app_url).to_string();
    let subscriber_token = OpaqueId::<Subscriber>::new(ctx.subscriber_id).to_string();
    let campaign_token = OpaqueId::<Campaign>::new(ctx.campaign_id).to_string();

    // Replace each href with a tracked URL. We rewrite a fresh copy rather
    // than streaming so that multiple identical hrefs share the same tracker.
    let rewritten = href_re().replace_all(html, |caps: &regex::Captures| {
        let original = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let Some(&link_id) = url_to_link.get(original) else {
            return caps[0].to_string();
        };
        let link_token = OpaqueId::<Link>::new(link_id).to_string();
        let tracked = format!("{app_url}/t/{link_token}/{subscriber_token}/{campaign_token}");
        format!(r#"href="{tracked}""#)
    });

    Ok(rewritten.into_owned())
}

fn extract_hrefs(html: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for caps in href_re().captures_iter(html) {
        let url = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
        if !url.is_empty() && seen.insert(url.clone()) {
            out.push(url);
        }
    }
    out
}

fn is_trackable(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    !lower.starts_with('#')
        && !lower.starts_with("mailto:")
        && !lower.starts_with("tel:")
        && !lower.starts_with("sms:")
        && !lower.starts_with("ftp:")
        && !lower.contains("[unsubscribe]")
        && !lower.contains("[webversion]")
        && !lower.contains("[reconsent]")
}

/// Insert a 1×1 open-tracking pixel just before `</body>` (or at the end if
/// there is no body close tag).
pub fn insert_open_pixel(html: &str, ctx: &Context) -> String {
    let pixel_url = format!(
        "{}/o/{}/{}",
        trim_slash(&ctx.app_url),
        OpaqueId::<Campaign>::new(ctx.campaign_id),
        OpaqueId::<Subscriber>::new(ctx.subscriber_id),
    );
    let img = format!(
        r#"<img src="{pixel_url}" alt="" width="1" height="1" style="display:none;" />"#
    );
    if let Some(idx) = html.to_ascii_lowercase().rfind("</body>") {
        let mut out = html.to_string();
        out.insert_str(idx, &img);
        out
    } else {
        format!("{html}{img}")
    }
}

// ── Pre-compiled regexes ────────────────────────────────────────────────────

fn fallback_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"\[([^\]\[,]+),\s*fallback=([^\]]*)\]").unwrap())
}

fn bare_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    // Tags are letters/digits/underscore/space, no commas/brackets.
    R.get_or_init(|| Regex::new(r"\[([A-Za-z][A-Za-z0-9_ ]*)\]").unwrap())
}

fn unsub_html_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"<unsubscribe[^>]*>(.*?)</unsubscribe>").unwrap())
}

fn href_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r#"href=["']([^"']+)["']"#).unwrap())
}

fn weekday_name(d: chrono::Weekday) -> &'static str {
    match d {
        chrono::Weekday::Mon => "Monday",
        chrono::Weekday::Tue => "Tuesday",
        chrono::Weekday::Wed => "Wednesday",
        chrono::Weekday::Thu => "Thursday",
        chrono::Weekday::Fri => "Friday",
        chrono::Weekday::Sat => "Saturday",
        chrono::Weekday::Sun => "Sunday",
    }
}

fn month_name(m: u32) -> &'static str {
    match m {
        1 => "January", 2 => "February", 3 => "March", 4 => "April",
        5 => "May", 6 => "June", 7 => "July", 8 => "August",
        9 => "September", 10 => "October", 11 => "November", 12 => "December",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Context {
        let mut cf = HashMap::new();
        cf.insert("country".into(), "US".into());
        let mut tys = HashMap::new();
        tys.insert("country".into(), CustomFieldType::Text);
        Context {
            subscriber_id: 1,
            list_id: 1,
            campaign_id: 1,
            email: "a@b.com".into(),
            name: "Alice".into(),
            custom_fields: cf,
            custom_field_types: tys,
            app_url: "http://app.test".into(),
        }
    }

    #[test]
    fn substitutes_name_and_email() {
        crate::utils::crypto::init_secret("test");
        let out = render("Hi [Name] at [Email]", &ctx());
        assert_eq!(out, "Hi Alice at a@b.com");
    }

    #[test]
    fn fallback_used_when_field_missing() {
        crate::utils::crypto::init_secret("test");
        let out = render("Country: [country, fallback=Earth]", &ctx());
        assert_eq!(out, "Country: US");
        let out = render("Tier: [tier, fallback=basic]", &ctx());
        assert_eq!(out, "Tier: basic");
    }

    #[test]
    fn unsubscribe_html_rewritten() {
        crate::utils::crypto::init_secret("test");
        let out = render("<unsubscribe>opt out</unsubscribe>", &ctx());
        assert!(out.contains("<a href=\"http://app.test/unsubscribe/"));
        assert!(out.contains(">opt out</a>"));
    }
}
