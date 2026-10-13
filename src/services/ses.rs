//! AWS SES (Simple Email Service) management — identity verification,
//! verification status, send-quota lookup.
//!
//! Outbound email itself goes through lettre/SMTP; this module
//! allows brands to verify their from-addresses directly inside Mailer without
//! leaving the admin UI.

use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;
use aws_sdk_sesv2::{
    config::Builder as SesConfigBuilder,
    types::{IdentityType, VerificationStatus as SdkStatus},
    Client,
};

use crate::utils::errors::{AppError, AppResult};

/// Verification status mirrored across SES values + a `NotFound` variant
/// for identities the SDK doesn't recognise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Verified,
    Pending,
    Failed,
    TemporaryFailure,
    NotStarted,
    NotFound,
}

impl From<SdkStatus> for VerificationStatus {
    fn from(s: SdkStatus) -> Self {
        match s {
            SdkStatus::Success => Self::Verified,
            SdkStatus::Pending => Self::Pending,
            SdkStatus::Failed => Self::Failed,
            SdkStatus::TemporaryFailure => Self::TemporaryFailure,
            SdkStatus::NotStarted => Self::NotStarted,
            _ => Self::NotFound,
        }
    }
}

/// Build an SES client from explicit static credentials so each brand can
/// use its own AWS keys (no global config dependence).
fn client(access_key: &str, secret_key: &str, region: &str) -> Client {
    let creds = Credentials::new(access_key, secret_key, None, None, "mailer-brand");
    let region_owned = region.to_string();
    let cfg = SesConfigBuilder::new()
        .behavior_version(BehaviorVersion::latest())
        .credentials_provider(creds)
        .region(Region::new(region_owned))
        .build();
    Client::from_conf(cfg)
}

/// Region encoded in an SES SMTP/API endpoint string. Defaults to
/// `us-east-1` when the brand stored only a hostname or left the field
/// empty.
pub fn region_from_endpoint(endpoint: &str) -> String {
    // Common formats:
    //   email.us-east-1.amazonaws.com
    //   email-smtp.eu-west-1.amazonaws.com
    //   us-east-1
    if endpoint.is_empty() {
        return "us-east-1".to_string();
    }
    if !endpoint.contains('.') {
        return endpoint.to_string();
    }
    for part in endpoint.split('.') {
        if part.starts_with("us-")
            || part.starts_with("eu-")
            || part.starts_with("ap-")
            || part.starts_with("sa-")
            || part.starts_with("ca-")
            || part.starts_with("af-")
            || part.starts_with("me-")
        {
            return part.to_string();
        }
    }
    "us-east-1".to_string()
}

/// Trigger a `VerifyEmailIdentity` call. SES sends a confirmation email to
/// the address; the recipient must click the link before SES will accept
/// outbound mail from that address.
pub async fn verify_identity(
    access_key: &str,
    secret_key: &str,
    region: &str,
    email: &str,
) -> AppResult<()> {
    let client = client(access_key, secret_key, region);
    client
        .create_email_identity()
        .email_identity(email)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("ses verify failed: {e}")))?;
    Ok(())
}

/// Look up the current verification status for `email`. Returns
/// `VerificationStatus::NotFound` when SES doesn't recognise the address.
pub async fn verification_status(
    access_key: &str,
    secret_key: &str,
    region: &str,
    email: &str,
) -> AppResult<VerificationStatus> {
    let client = client(access_key, secret_key, region);
    match client
        .get_email_identity()
        .email_identity(email)
        .send()
        .await
    {
        Ok(out) => {
            if out.verified_for_sending_status() {
                return Ok(VerificationStatus::Verified);
            }
            // SDK exposes verification status via the verification_status field.
            if let Some(status) = out.verification_status {
                return Ok(VerificationStatus::from(status));
            }
            Ok(VerificationStatus::Pending)
        }
        Err(_) => Ok(VerificationStatus::NotFound),
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SendQuota {
    /// Max sends per 24h.
    pub max_24_hour: f64,
    /// Sends in the last 24h.
    pub sent_last_24_hours: f64,
    /// Max send rate per second.
    pub max_send_rate: f64,
}

pub async fn get_send_quota(
    access_key: &str,
    secret_key: &str,
    region: &str,
) -> AppResult<SendQuota> {
    let client = client(access_key, secret_key, region);
    let out = client
        .get_account()
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("ses get_account failed: {e}")))?;

    let send_quota = out.send_quota;
    Ok(SendQuota {
        max_24_hour: send_quota.as_ref().map(|q| q.max24_hour_send).unwrap_or(0.0),
        sent_last_24_hours: send_quota.as_ref().map(|q| q.sent_last24_hours).unwrap_or(0.0),
        max_send_rate: send_quota.as_ref().map(|q| q.max_send_rate).unwrap_or(0.0),
    })
}

/// List every email identity registered with SES under these credentials.
pub async fn list_identities(
    access_key: &str,
    secret_key: &str,
    region: &str,
) -> AppResult<Vec<String>> {
    let client = client(access_key, secret_key, region);
    let out = client
        .list_email_identities()
        .page_size(1000)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("ses list_email_identities failed: {e}")))?;

    let mut identities = Vec::new();
    for it in out.email_identities() {
        if matches!(it.identity_type(), Some(IdentityType::EmailAddress)) {
            if let Some(name) = it.identity_name() {
                identities.push(name.to_string());
            }
        }
    }
    Ok(identities)
}
