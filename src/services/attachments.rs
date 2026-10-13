//! Per-campaign file attachments.
//!
//! Files live on disk under `uploads/campaign-attachments/<campaign_id>/`.
//! The directory listing is the source of truth for attachments attached to a campaign.

use std::path::{Path, PathBuf};

use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::services::mailer::EmailAttachment;

/// Maximum size of a single attachment, in bytes (10 MB).
pub const MAX_ATTACHMENT_BYTES: usize = 10 * 1024 * 1024;

/// Root directory under which all campaign attachments live. Resolved at
/// runtime relative to the working directory so deployments can mount a
/// volume here without recompiling.
fn root() -> PathBuf {
    PathBuf::from("uploads").join("campaign-attachments")
}

fn dir_for(campaign_id: i32) -> PathBuf {
    root().join(campaign_id.to_string())
}

/// Storage root for autoresponder-email attachments. Same shape as
/// the campaign storage but under a separate directory so a campaign
/// id and an ares-email id can collide harmlessly.
fn ares_root() -> PathBuf {
    PathBuf::from("uploads").join("ares-attachments")
}

fn ares_dir_for(email_id: i32) -> PathBuf {
    ares_root().join(email_id.to_string())
}

/// AR-email mirror of [`save`].
pub async fn save_for_ares(email_id: i32, filename: &str, bytes: &[u8]) -> std::io::Result<String> {
    let dir = ares_dir_for(email_id);
    fs::create_dir_all(&dir).await?;
    let safe = sanitize_filename(filename);
    let path = dir.join(&safe);
    let mut f = fs::File::create(&path).await?;
    f.write_all(bytes).await?;
    f.sync_all().await?;
    Ok(safe)
}

/// AR-email mirror of [`delete`].
pub async fn delete_for_ares(email_id: i32, filename: &str) -> std::io::Result<()> {
    let safe = sanitize_filename(filename);
    let path = ares_dir_for(email_id).join(safe);
    match fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// AR-email mirror of [`list`].
pub async fn list_for_ares(email_id: i32) -> Vec<String> {
    let dir = ares_dir_for(email_id);
    let mut out = Vec::new();
    let mut rd = match fs::read_dir(&dir).await {
        Ok(rd) => rd,
        Err(_) => return out,
    };
    while let Ok(Some(entry)) = rd.next_entry().await {
        if entry.file_type().await.map(|t| t.is_file()).unwrap_or(false) {
            if let Some(name) = entry.file_name().to_str() {
                out.push(name.to_string());
            }
        }
    }
    out.sort();
    out
}

/// AR-email mirror of [`load_for_campaign`].
pub async fn load_for_ares_email(email_id: i32) -> Vec<EmailAttachment> {
    let names = list_for_ares(email_id).await;
    let dir = ares_dir_for(email_id);
    let mut out = Vec::with_capacity(names.len());
    for name in names {
        let path = dir.join(&name);
        let bytes = match fs::read(&path).await {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(error = ?e, ?path, "skip ares attachment: read failed");
                continue;
            }
        };
        out.push(EmailAttachment {
            filename: name,
            content_type: guess_content_type(&path),
            bytes,
        });
    }
    out
}

/// Sanitize an upload filename: strip path separators, drop leading dots,
/// keep only `[A-Za-z0-9._-]`. Empty result → `attachment`.
pub fn sanitize_filename(name: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '.' | '_' | '-' => c,
            _ => '_',
        })
        .collect();
    while s.starts_with('.') {
        s.remove(0);
    }
    if s.is_empty() {
        "attachment".to_string()
    } else {
        s
    }
}

/// Save bytes as a new attachment for a campaign. Returns the on-disk
/// filename actually written (sanitized).
pub async fn save(campaign_id: i32, filename: &str, bytes: &[u8]) -> std::io::Result<String> {
    let dir = dir_for(campaign_id);
    fs::create_dir_all(&dir).await?;
    let safe = sanitize_filename(filename);
    let path = dir.join(&safe);
    let mut f = fs::File::create(&path).await?;
    f.write_all(bytes).await?;
    f.sync_all().await?;
    Ok(safe)
}

/// Delete one attachment by name.
pub async fn delete(campaign_id: i32, filename: &str) -> std::io::Result<()> {
    let safe = sanitize_filename(filename);
    let path = dir_for(campaign_id).join(safe);
    match fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// List attachment filenames currently on disk for a campaign.
pub async fn list(campaign_id: i32) -> Vec<String> {
    let dir = dir_for(campaign_id);
    let mut out = Vec::new();
    let mut rd = match fs::read_dir(&dir).await {
        Ok(rd) => rd,
        Err(_) => return out,
    };
    while let Ok(Some(entry)) = rd.next_entry().await {
        if entry.file_type().await.map(|t| t.is_file()).unwrap_or(false) {
            if let Some(name) = entry.file_name().to_str() {
                out.push(name.to_string());
            }
        }
    }
    out.sort();
    out
}

/// Load every campaign attachment from disk into memory as a vector of
/// `EmailAttachment` values ready to ship via lettre. Returns an empty
/// vector if the directory does not exist or is empty.
pub async fn load_for_campaign(campaign_id: i32) -> Vec<EmailAttachment> {
    let names = list(campaign_id).await;
    let dir = dir_for(campaign_id);
    let mut out = Vec::with_capacity(names.len());
    for name in names {
        let path = dir.join(&name);
        let bytes = match fs::read(&path).await {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(error = ?e, ?path, "skip attachment: read failed");
                continue;
            }
        };
        let content_type = guess_content_type(&path);
        out.push(EmailAttachment {
            filename: name,
            content_type,
            bytes,
        });
    }
    out
}

fn guess_content_type(path: &Path) -> String {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "csv" => "text/csv",
        "txt" => "text/plain",
        "html" | "htm" => "text/html",
        "doc" => "application/msword",
        "docx" => {
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        }
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
    .to_string()
}

/// True if a file extension is in the brand's `allowed_attachments` list.
/// `allowed` is a comma-separated list (case-insensitive) of bare extensions
/// like `"pdf,doc,docx,jpg"`. Empty means "allow anything".
pub fn extension_allowed(allowed: Option<&str>, filename: &str) -> bool {
    let raw = match allowed {
        Some(s) if !s.trim().is_empty() => s,
        _ => return true,
    };
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    raw.split(',')
        .map(|s| s.trim().to_ascii_lowercase())
        .any(|allowed_ext| allowed_ext == ext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_path_separators() {
        // Slashes become underscores; leading dots are dropped so the result
        // can never be a hidden file. Embedded dots survive (they're harmless
        // inside a single path component — PathBuf::join doesn't interpret
        // substring ".." as a parent reference).
        assert_eq!(sanitize_filename("../../etc/passwd"), "_.._etc_passwd");
        assert_eq!(sanitize_filename("name.pdf"), "name.pdf");
        assert_eq!(sanitize_filename("file with spaces.pdf"), "file_with_spaces.pdf");
        assert_eq!(sanitize_filename(""), "attachment");
        assert_eq!(sanitize_filename(".hidden"), "hidden");
    }

    #[test]
    fn extension_allowed_when_allowlist_empty() {
        assert!(extension_allowed(None, "file.exe"));
        assert!(extension_allowed(Some(""), "file.exe"));
        assert!(extension_allowed(Some("   "), "file.exe"));
    }

    #[test]
    fn extension_allowed_filters_correctly() {
        let allow = Some("pdf,doc,docx,jpg");
        assert!(extension_allowed(allow, "report.pdf"));
        assert!(extension_allowed(allow, "report.PDF"));
        assert!(extension_allowed(allow, "photo.jpg"));
        assert!(!extension_allowed(allow, "danger.exe"));
        assert!(!extension_allowed(allow, "noext"));
    }
}
