//! GeoIP service — MaxMind GeoLite2 country lookup for subscribers.
//!
//! Loads a MaxMind / DB-IP `.mmdb` once at boot (path comes from the
//! `GEOIP_DB` environment variable, default
//! `./data/GeoLite2-Country.mmdb`) and exposes a single
//! [`country_for_ip`] function that returns the ISO-3166 alpha-2 country
//! code for an IP address.
//!
//! When the DB file is missing, malformed, or the IP doesn't resolve, the
//! function returns `None` and callers fall back to whatever the request
//! supplied. The reader is held in a `OnceCell<Vec<u8>>` of the file
//! contents — `geoip2::Reader::from_bytes` borrows from that buffer so we
//! keep it alive for the lifetime of the process.

use std::net::IpAddr;
use std::path::Path;
use std::sync::OnceLock;

/// Owned MMDB bytes; the `Reader` constructed below borrows from this.
static MMDB_BYTES: OnceLock<Option<Vec<u8>>> = OnceLock::new();

/// Default search path for the GeoLite2-Country database.
const DEFAULT_DB_PATH: &str = "./data/GeoLite2-Country.mmdb";

/// Initialise the Geo lookup. Idempotent — only the first call wins.
///
/// Call this once during application startup. On failure (file missing,
/// unreadable, or the wrong format) we log a warning and disable geo
/// lookups; subsequent calls to [`country_for_ip`] return `None` instead
/// of erroring.
pub fn init(path: Option<&str>) {
    let path = path
        .map(|s| s.to_string())
        .or_else(|| std::env::var("GEOIP_DB").ok())
        .unwrap_or_else(|| DEFAULT_DB_PATH.to_string());

    let bytes = match std::fs::read(Path::new(&path)) {
        Ok(b) => {
            tracing::info!(path = %path, bytes = b.len(), "GeoIP database loaded");
            Some(b)
        }
        Err(e) => {
            tracing::warn!(path = %path, error = ?e, "GeoIP database unavailable; lookups disabled");
            None
        }
    };
    let _ = MMDB_BYTES.set(bytes);
}

/// Look up the ISO-3166 alpha-2 country code for `ip`, e.g. `"US"` or `"DE"`.
///
/// Returns `None` when:
/// - [`init`] hasn't been called or failed to load the database
/// - the IP doesn't parse
/// - the IP is not present in the database
pub fn country_for_ip(ip: &str) -> Option<String> {
    let bytes = MMDB_BYTES.get()?.as_deref()?;
    let parsed: IpAddr = ip.parse().ok()?;

    let reader = geoip2::Reader::<geoip2::Country>::from_bytes(bytes).ok()?;
    let result = reader.lookup(parsed).ok()?;

    let iso = result
        .country
        .and_then(|c| c.iso_code)
        .map(|s| s.to_string())?;

    Some(iso)
}
