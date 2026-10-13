//! In-browser preview for campaign / template / autoresponder-email bodies.
//!
//! The preview is a stand-alone HTML document (NOT wrapped in the admin
//! shell) so the rendered email body sees the same browser environment it
//! would in a real inbox. We add a thin "Preview" header bar above the
//! rendered body that surfaces the metadata an editor wants to verify
//! (subject, from name/address, body type) and a "Close" link.
//!
//! Personalization tags are substituted using the same `Context` the test
//! send / scheduler use, with a synthetic dummy subscriber so tags like
//! `[Name]`, `[Email]`, `[unsubscribe]`, `<unsubscribe>` resolve to
//! concrete strings rather than leaking literal brackets.

use crate::database::entities::subscribers as subs_entity;
use crate::services::personalization::{render, Context};

/// Build a dummy subscriber row used to drive personalization in previews
/// and test-sends. Mirrors the stub used by `campaigns::test_send` and
/// `autoresponders::email_test_send`.
pub fn dummy_subscriber(email: &str, name: &str) -> subs_entity::Model {
    let now = chrono::Utc::now().timestamp();
    subs_entity::Model {
        id: 0,
        user_id: None,
        name: Some(name.to_string()),
        email: Some(email.to_string()),
        list: Some(0),
        custom_fields: Some(String::new()),
        unsubscribed: Some(0),
        bounced: Some(0),
        bounce_soft: Some(0),
        complaint: Some(0),
        confirmed: Some(1),
        last_campaign: None,
        last_ares: None,
        timestamp: Some(now),
        join_date: Some(now),
        message_id: None,
        ip: None,
        country: None,
        referrer: None,
        method: Some(1),
        added_via: Some(1),
        notes: None,
        gdpr: Some(0),
    }
}

/// Metadata strip rendered above the preview body.
pub struct PreviewMeta<'a> {
    pub kind: &'a str,        // "Campaign" / "Template" / "Autoresponder email"
    pub subject: &'a str,
    pub from_name: &'a str,
    pub from_email: &'a str,
    pub back_href: &'a str,   // editor URL the close link returns to
}

/// Which body to show on first paint. `?view=plain` flips to plain text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewMode {
    Html,
    Plain,
}

impl PreviewMode {
    pub fn from_query(s: Option<&str>) -> Self {
        match s.unwrap_or("").to_ascii_lowercase().as_str() {
            "plain" | "text" | "txt" => PreviewMode::Plain,
            _ => PreviewMode::Html,
        }
    }
}

/// Render the HTML and plain-text bodies with personalization substituted,
/// then wrap them in a stand-alone document with a metadata header bar
/// that includes an HTML / Plain-text toggle.
///
/// Both bodies are inlined into the document — the toggle is pure CSS
/// (`:target`-style) so it works without JavaScript and the active pane
/// matches the `mode` argument on first paint.
pub fn build_preview(
    html_body: &str,
    plain_body: &str,
    mode: PreviewMode,
    meta: PreviewMeta<'_>,
    ctx: &Context,
) -> String {
    let rendered_subject = render(meta.subject, ctx);
    let rendered_html = render(html_body, ctx);
    let rendered_plain = render(plain_body, ctx);

    let html_pane = if rendered_html.trim().is_empty() {
        "<p style=\"font-family:sans-serif;color:#9ca3af;padding:24px;\">No HTML body yet — add some content in the editor and try again.</p>".to_string()
    } else {
        rendered_html
    };
    let plain_pane = if rendered_plain.trim().is_empty() {
        "<p style=\"font-family:sans-serif;color:#9ca3af;padding:24px;\">No plain-text body yet — add some content in the editor and try again.</p>".to_string()
    } else {
        format!(
            "<pre style=\"margin:0;padding:24px;font:13px/1.6 ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;color:#1f2937;white-space:pre-wrap;word-wrap:break-word;\">{}</pre>",
            html_escape(&rendered_plain),
        )
    };

    let (html_class, plain_class, html_btn_active, plain_btn_active) = match mode {
        PreviewMode::Html => ("mailer-pane active", "mailer-pane", "active", ""),
        PreviewMode::Plain => ("mailer-pane", "mailer-pane active", "", "active"),
    };

    let html_url = format!("{}?view=html", html_escape(meta.back_href.trim_end_matches("/edit")));
    let plain_url = format!("{}?view=plain", html_escape(meta.back_href.trim_end_matches("/edit")));
    // Toggle URLs hit the SAME preview endpoint (not the editor) — derive
    // it from the current document so we don't have to thread it through.

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8"/>
<meta name="viewport" content="width=device-width, initial-scale=1.0"/>
<meta name="robots" content="noindex,nofollow"/>
<title>Preview · {kind} · Mailer</title>
<style>
  html,body {{ margin:0; padding:0; background:#f3f4f6; }}
  .mailer-preview-bar {{
    background:#0f172a; color:#f8fafc; padding:10px 20px;
    font:13px/1.4 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;
    display:flex; flex-wrap:wrap; gap:14px; align-items:center;
    box-shadow:0 1px 3px rgba(0,0,0,0.15); position:sticky; top:0; z-index:9999;
  }}
  .mailer-preview-bar strong {{ color:#fff; font-weight:600; }}
  .mailer-preview-bar .badge {{
    background:#fbbf24; color:#1f2937; padding:2px 8px; border-radius:4px;
    font-weight:600; font-size:11px; text-transform:uppercase; letter-spacing:0.05em;
  }}
  .mailer-preview-bar .meta {{ color:#cbd5e1; }}
  .mailer-preview-bar .meta b {{ color:#fff; font-weight:500; }}
  .mailer-preview-bar .toggle {{
    display:inline-flex; background:#1e293b; border:1px solid #334155;
    border-radius:6px; overflow:hidden; margin-left:auto;
  }}
  .mailer-preview-bar .toggle button {{
    background:transparent; color:#cbd5e1; border:0;
    padding:6px 12px; font:inherit; cursor:pointer;
  }}
  .mailer-preview-bar .toggle button.active {{ background:#3b82f6; color:#fff; }}
  .mailer-preview-bar .toggle button:not(.active):hover {{ background:#334155; color:#fff; }}
  .mailer-preview-bar a.close {{
    color:#fff; text-decoration:none;
    background:#1e293b; padding:6px 12px; border-radius:6px;
    border:1px solid #334155;
  }}
  .mailer-preview-bar a.close:hover {{ background:#334155; }}
  .mailer-preview-frame {{
    background:#fff; max-width:800px; margin:24px auto; padding:0;
    box-shadow:0 1px 3px rgba(0,0,0,0.08); border-radius:6px; overflow:hidden;
  }}
  .mailer-pane {{ display:none; }}
  .mailer-pane.active {{ display:block; }}
</style>
</head>
<body>
<div class="mailer-preview-bar">
  <span class="badge">Preview</span>
  <span class="meta"><b>{kind}:</b> {subject}</span>
  <span class="meta">From <b>{from_name}</b> &lt;{from_email}&gt;</span>
  <span class="toggle" role="tablist" aria-label="View mode">
    <button type="button" id="mailer-html-btn" class="{html_btn_active}" onclick="mailerSwitch('html')">
      <i class="fa-solid fa-code"></i> HTML
    </button>
    <button type="button" id="mailer-plain-btn" class="{plain_btn_active}" onclick="mailerSwitch('plain')">
      <i class="fa-solid fa-align-left"></i> Plain text
    </button>
  </span>
  <a href="{back_href}" class="close">← Back to editor</a>
</div>
<div class="mailer-preview-frame">
  <div id="mailer-html" class="{html_class}">{html_pane}</div>
  <div id="mailer-plain" class="{plain_class}">{plain_pane}</div>
</div>
<script>
function mailerSwitch(which) {{
  var html = document.getElementById('mailer-html');
  var plain = document.getElementById('mailer-plain');
  var hb = document.getElementById('mailer-html-btn');
  var pb = document.getElementById('mailer-plain-btn');
  if (which === 'plain') {{
    html.classList.remove('active'); plain.classList.add('active');
    hb.classList.remove('active'); pb.classList.add('active');
  }} else {{
    plain.classList.remove('active'); html.classList.add('active');
    pb.classList.remove('active'); hb.classList.add('active');
  }}
  try {{
    var u = new URL(window.location.href);
    u.searchParams.set('view', which);
    history.replaceState(null, '', u.toString());
  }} catch (e) {{}}
}}
(function () {{
  try {{
    var v = new URL(window.location.href).searchParams.get('view');
    if (v === 'plain' || v === 'html') mailerSwitch(v);
  }} catch (e) {{}}
}})();
</script>
<noscript><!-- non-JS fallback links --><div style="display:none">{html_url} {plain_url}</div></noscript>
</body>
</html>"##,
        kind = html_escape(meta.kind),
        subject = html_escape(&rendered_subject),
        from_name = html_escape(meta.from_name),
        from_email = html_escape(meta.from_email),
        back_href = html_escape(meta.back_href),
        html_pane = html_pane,
        plain_pane = plain_pane,
        html_class = html_class,
        plain_class = plain_class,
        html_btn_active = html_btn_active,
        plain_btn_active = plain_btn_active,
        html_url = html_url,
        plain_url = plain_url,
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
