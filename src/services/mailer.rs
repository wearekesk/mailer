//! Outbound email — SMTP via lettre, with the `EmailProvider` trait kept for
//! tests (`StubMailer`) and future SES integration.

use async_trait::async_trait;
use lettre::message::header::ContentType;
use lettre::message::{Attachment, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use rand::Rng;

use crate::database::entities::apps;
use crate::utils::errors::AppError;

/// One outbound transactional email.
#[derive(Debug, Clone)]
pub struct OutboundEmail {
    pub from_name: String,
    pub from_email: String,
    pub to_email: String,
    pub to_name: Option<String>,
    pub reply_to: String,
    pub subject: String,
    pub html: String,
    pub plain: String,
    /// File attachments, included on the outbound MIME message as a
    /// `multipart/mixed` wrapping the alternative HTML/plain body.
    #[allow(clippy::type_complexity)]
    pub attachments: Vec<EmailAttachment>,
}

/// One file attachment for an outbound email.
#[derive(Debug, Clone)]
pub struct EmailAttachment {
    pub filename: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

/// SMTP credentials resolved from a brand row.
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub use_starttls: bool,
}

impl SmtpConfig {
    /// Build an `SmtpConfig` from an `apps` row, returning `None` when the
    /// brand has no SMTP host configured.
    pub fn from_brand(brand: &apps::Model) -> Option<Self> {
        let host = brand.smtp_host.clone().filter(|s| !s.is_empty())?;
        let port = brand
            .smtp_port
            .clone()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(587);
        let username = brand.smtp_username.clone().unwrap_or_default();
        let password = brand.smtp_password.clone().unwrap_or_default();
        let ssl = brand.smtp_ssl.clone().unwrap_or_default();
        let use_starttls = matches!(
            ssl.to_ascii_lowercase().as_str(),
            "tls" | "starttls" | "1" | "true"
        ) || port == 587;
        Some(Self {
            host,
            port,
            username,
            password,
            use_starttls,
        })
    }
}

/// Provider abstraction so tests can swap in `StubMailer`.
#[async_trait]
pub trait EmailProvider: Send + Sync {
    async fn send(&self, email: OutboundEmail) -> anyhow::Result<String>;
}

// ── Lettre SMTP impl ────────────────────────────────────────────────────────

pub struct SmtpMailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
}

impl SmtpMailer {
    pub fn new(cfg: &SmtpConfig) -> anyhow::Result<Self> {
        let creds = if cfg.username.is_empty() {
            None
        } else {
            Some(Credentials::new(cfg.username.clone(), cfg.password.clone()))
        };
        let mut builder = if cfg.use_starttls {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.host)?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&cfg.host)
        };
        builder = builder.port(cfg.port);
        if let Some(c) = creds {
            builder = builder.credentials(c);
        }
        Ok(Self {
            transport: builder.build(),
        })
    }
}

#[async_trait]
impl EmailProvider for SmtpMailer {
    async fn send(&self, email: OutboundEmail) -> anyhow::Result<String> {
        let from: Mailbox = format!("{} <{}>", email.from_name, email.from_email).parse()?;
        let to: Mailbox = match email.to_name.as_deref() {
            Some(name) if !name.is_empty() => format!("{} <{}>", name, email.to_email).parse()?,
            _ => email.to_email.parse()?,
        };
        let reply_to: Mailbox = email.reply_to.parse()?;

        let plain = if email.plain.is_empty() {
            strip_html(&email.html)
        } else {
            email.plain.clone()
        };
        let alternative = MultiPart::alternative_plain_html(plain, email.html.clone());

        let body = if email.attachments.is_empty() {
            alternative
        } else {
            let mut mixed = MultiPart::mixed().multipart(alternative);
            for att in &email.attachments {
                let ct: ContentType = att
                    .content_type
                    .parse()
                    .unwrap_or_else(|_| ContentType::parse("application/octet-stream").unwrap());
                let part: SinglePart = Attachment::new(att.filename.clone())
                    .body(att.bytes.clone(), ct);
                mixed = mixed.singlepart(part);
            }
            mixed
        };

        let msg = Message::builder()
            .from(from)
            .to(to)
            .reply_to(reply_to)
            .subject(email.subject.clone())
            .multipart(body)?;

        let result = self.transport.send(msg).await?;
        Ok(format!("{:?}", result.code()))
    }
}

/// Crude `<tag>` stripper used as the plain-text fallback when the HTML body
/// has no accompanying plain version.
fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

// ── Stub provider (logs + returns a fake id) ────────────────────────────────

pub struct StubMailer;

#[async_trait]
impl EmailProvider for StubMailer {
    async fn send(&self, email: OutboundEmail) -> anyhow::Result<String> {
        tracing::info!(
            to = %email.to_email,
            subject = %email.subject,
            "StubMailer: would send email (not actually sent)"
        );
        Ok(format!("stub-{}", random_hex_id()))
    }
}

// ── Per-brand dispatcher ─────────────────────────────────────────────────────

/// Build a mailer for a brand, falling back to [`StubMailer`] if the brand
/// has no SMTP configured.
pub fn mailer_for_brand(brand: &apps::Model) -> Box<dyn EmailProvider> {
    match SmtpConfig::from_brand(brand) {
        Some(cfg) => match SmtpMailer::new(&cfg) {
            Ok(m) => Box::new(m),
            Err(e) => {
                tracing::warn!(error = ?e, brand = brand.id, "SMTP init failed; falling back to stub");
                Box::new(StubMailer)
            }
        },
        None => Box::new(StubMailer),
    }
}

/// Helper used by the scheduler.
pub async fn send_via_brand(
    brand: &apps::Model,
    email: OutboundEmail,
) -> Result<String, AppError> {
    let mailer = mailer_for_brand(brand);
    mailer
        .send(email)
        .await
        .map_err(|e| AppError::Internal(format!("smtp send failed: {e}")))
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn random_hex_id() -> String {
    let mut rng = rand::thread_rng();
    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        rng.r#gen::<u32>(),
        rng.r#gen::<u16>(),
        rng.r#gen::<u16>() & 0x0fff,
        (rng.r#gen::<u16>() & 0x3fff) | 0x8000,
        rng.r#gen::<u64>() & 0x0000_ffff_ffff_ffff,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_brand() -> apps::Model {
        apps::Model {
            id: 1,
            user_id: None,
            app_name: None,
            from_name: None,
            from_email: None,
            reply_to: None,
            app_key: None,
            brand_logo_filename: None,
            currency: None,
            delivery_fee: None,
            cost_per_recipient: None,
            smtp_host: None,
            smtp_port: None,
            smtp_ssl: None,
            smtp_username: None,
            smtp_password: None,
            allocated_quota: None,
            current_quota: None,
            day_of_reset: None,
            month_of_next_reset: None,
            year_of_next_reset: None,
            no_expiry: None,
            test_email: None,
            test_email_prefix: None,
            allowed_attachments: None,
            bounce_setup: None,
            complaint_setup: None,
            campaigns_only: None,
            templates_only: None,
            lists_only: None,
            gdpr_only: None,
            gdpr_only_ar: None,
            gdpr_options: None,
            recaptcha_sitekey: None,
            recaptcha_secretkey: None,
            custom_domain: None,
            custom_domain_protocol: None,
            custom_domain_enabled: None,
            templates_lists_sorting: None,
            campaign_report_rows: None,
            query_string: None,
            opens_tracking: None,
            links_tracking: None,
            opt_in: None,
            hide_lists: None,
        }
    }

    #[test]
    fn from_brand_returns_none_without_host() {
        let b = empty_brand();
        assert!(SmtpConfig::from_brand(&b).is_none());
    }

    #[test]
    fn from_brand_returns_none_with_empty_host() {
        let mut b = empty_brand();
        b.smtp_host = Some(String::new());
        assert!(SmtpConfig::from_brand(&b).is_none());
    }

    #[test]
    fn from_brand_defaults_port_to_587_and_starttls() {
        let mut b = empty_brand();
        b.smtp_host = Some("smtp.example.com".into());
        let cfg = SmtpConfig::from_brand(&b).expect("config built");
        assert_eq!(cfg.host, "smtp.example.com");
        assert_eq!(cfg.port, 587);
        assert!(cfg.use_starttls, "port 587 implies starttls by default");
    }

    #[test]
    fn from_brand_explicit_starttls_flag_variants() {
        for v in ["tls", "STARTTLS", "1", "TRUE"] {
            let mut b = empty_brand();
            b.smtp_host = Some("smtp.example.com".into());
            b.smtp_port = Some("2525".into());
            b.smtp_ssl = Some(v.into());
            let cfg = SmtpConfig::from_brand(&b).unwrap();
            assert!(cfg.use_starttls, "value {v} should enable starttls");
        }
    }

    #[test]
    fn from_brand_no_starttls_when_explicit_off() {
        let mut b = empty_brand();
        b.smtp_host = Some("smtp.example.com".into());
        b.smtp_port = Some("465".into());
        b.smtp_ssl = Some("ssl".into());
        let cfg = SmtpConfig::from_brand(&b).unwrap();
        assert_eq!(cfg.port, 465);
        assert!(
            !cfg.use_starttls,
            "ssl on 465 should not enable starttls"
        );
    }

    #[test]
    fn from_brand_invalid_port_falls_back_to_587() {
        let mut b = empty_brand();
        b.smtp_host = Some("smtp.example.com".into());
        b.smtp_port = Some("not-a-port".into());
        let cfg = SmtpConfig::from_brand(&b).unwrap();
        assert_eq!(cfg.port, 587);
    }

    #[test]
    fn mailer_for_brand_falls_back_to_stub_without_smtp() {
        let b = empty_brand();
        let _m = mailer_for_brand(&b);
    }
}
