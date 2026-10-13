//! Opaque ID encoding.
//!
//! IDs are 32-bit integers internally but we never expose them in URLs or API
//! responses. Each ID is packed into a 16-byte block, encrypted with AES-128
//! (ECB on a single block — equivalent to one CBC block with a zero IV), and
//! base64url-encoded. The cipher key is derived from the install's
//! `secret_key` mixed with a per-domain tag (`"brand"`, `"list"`, …) so that
//! a token issued for one resource type cannot be replayed against another.
//!
//! The 16-byte plaintext layout is:
//!
//! ```text
//!   bytes  0..4   id (i32 little-endian)
//!   bytes  4..8   !id (one's complement, redundancy check)
//!   bytes  8..12  ASCII "SNDY" magic
//!   bytes 12..16  zero padding
//! ```
//!
//! On decrypt we verify both the complement and the magic — random ciphertexts
//! are rejected with overwhelming probability (≈ 2⁻⁶⁴).

use std::sync::OnceLock;

use aes::cipher::{generic_array::GenericArray, BlockDecrypt, BlockEncrypt, KeyInit};
use aes::Aes128;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};

const MAGIC: [u8; 4] = *b"SNDY";

/// Process-wide secret. Must be set once at boot via [`init_secret`].
static SECRET: OnceLock<String> = OnceLock::new();

/// Install the application secret used to derive cipher keys. Idempotent —
/// only the first call wins, subsequent calls are silently ignored.
pub fn init_secret(secret: &str) {
    let _ = SECRET.set(secret.to_string());
}

fn cipher_for(tag: &str) -> Aes128 {
    let secret = SECRET
        .get()
        .map(String::as_str)
        .unwrap_or("change-me-in-production-32bytes!!");
    let mut hash = Sha256::new();
    hash.update(secret.as_bytes());
    hash.update(b"|");
    hash.update(tag.as_bytes());
    let digest = hash.finalize();
    Aes128::new_from_slice(&digest[..16]).expect("Aes128 key length is 16")
}

/// Encrypt `id` into a base64url token using the per-`tag` cipher key.
pub fn encrypt_id(id: i32, tag: &str) -> String {
    let mut block = [0u8; 16];
    block[0..4].copy_from_slice(&id.to_le_bytes());
    block[4..8].copy_from_slice(&(!id).to_le_bytes());
    block[8..12].copy_from_slice(&MAGIC);
    // bytes 12..16 stay zero

    let mut ga = *GenericArray::from_slice(&block);
    cipher_for(tag).encrypt_block(&mut ga);
    URL_SAFE_NO_PAD.encode(ga)
}

/// Decrypt a token produced by [`encrypt_id`] for the same `tag`. Returns
/// `None` if the token is malformed, the wrong size, or the redundancy check
/// fails.
pub fn decrypt_id(token: &str, tag: &str) -> Option<i32> {
    let bytes = URL_SAFE_NO_PAD.decode(token.trim()).ok()?;
    if bytes.len() != 16 {
        return None;
    }
    let mut ga = *GenericArray::from_slice(&bytes);
    cipher_for(tag).decrypt_block(&mut ga);

    if ga[8..12] != MAGIC {
        return None;
    }
    let id = i32::from_le_bytes(ga[0..4].try_into().ok()?);
    let inv = i32::from_le_bytes(ga[4..8].try_into().ok()?);
    if !id != inv {
        return None;
    }
    Some(id)
}

/// Encrypt a `(entity_id, related_id)` pair into a tracking token. Used for
/// click and open URLs — the resulting token is base64url-encoded, exactly
/// 22 characters, and binds both ids together so they cannot be tampered
/// with independently.
pub fn encode_tracking_token(id1: i32, id2: i32) -> String {
    let mut block = [0u8; 16];
    block[0..4].copy_from_slice(&id1.to_le_bytes());
    block[4..8].copy_from_slice(&id2.to_le_bytes());
    block[8..12].copy_from_slice(&MAGIC);
    let checksum = id1.wrapping_mul(0x9E3779B1u32 as i32).wrapping_add(id2);
    block[12..16].copy_from_slice(&checksum.to_le_bytes());

    let mut ga = *GenericArray::from_slice(&block);
    cipher_for("track").encrypt_block(&mut ga);
    URL_SAFE_NO_PAD.encode(ga)
}

/// Decode a token produced by [`encode_tracking_token`]. Returns `None` if
/// the token is malformed, the wrong size, or fails the checksum/magic check.
pub fn decode_tracking_token(token: &str) -> Option<(i32, i32)> {
    let bytes = URL_SAFE_NO_PAD.decode(token.trim()).ok()?;
    if bytes.len() != 16 {
        return None;
    }
    let mut ga = *GenericArray::from_slice(&bytes);
    cipher_for("track").decrypt_block(&mut ga);

    if ga[8..12] != MAGIC {
        return None;
    }
    let id1 = i32::from_le_bytes(ga[0..4].try_into().ok()?);
    let id2 = i32::from_le_bytes(ga[4..8].try_into().ok()?);
    let want = i32::from_le_bytes(ga[12..16].try_into().ok()?);
    let have = id1.wrapping_mul(0x9E3779B1u32 as i32).wrapping_add(id2);
    if want != have {
        return None;
    }
    Some((id1, id2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        init_secret("test-secret-key");
        for id in [0, 1, -1, 12345, i32::MIN, i32::MAX] {
            let tok = encrypt_id(id, "list");
            assert_eq!(decrypt_id(&tok, "list"), Some(id));
        }
    }

    #[test]
    fn cross_tag_rejected() {
        init_secret("test-secret-key");
        let tok = encrypt_id(42, "brand");
        assert_eq!(decrypt_id(&tok, "list"), None);
    }

    #[test]
    fn random_garbage_rejected() {
        init_secret("test-secret-key");
        let garbage = URL_SAFE_NO_PAD.encode([0u8; 16]);
        assert_eq!(decrypt_id(&garbage, "list"), None);
    }

    #[test]
    fn invalid_base64_rejected() {
        init_secret("test-secret-key");
        assert_eq!(decrypt_id("!!!not-valid-base64!!!", "list"), None);
        assert_eq!(decrypt_id("", "list"), None);
    }

    #[test]
    fn truncated_token_rejected() {
        init_secret("test-secret-key");
        let tok = encrypt_id(123, "list");
        let truncated = &tok[..tok.len().saturating_sub(2)];
        assert_eq!(decrypt_id(truncated, "list"), None);
    }

    #[test]
    fn tampered_token_rejected() {
        init_secret("test-secret-key");
        let tok = encrypt_id(123, "list");
        let mut bytes: Vec<u8> = tok.into_bytes();
        // flip a byte in the middle
        let mid = bytes.len() / 2;
        bytes[mid] = if bytes[mid] == b'A' { b'B' } else { b'A' };
        let tampered = String::from_utf8(bytes).unwrap();
        assert_eq!(decrypt_id(&tampered, "list"), None);
    }

    #[test]
    fn tracking_token_round_trip() {
        init_secret("test-secret-key");
        for (a, b) in [(0, 0), (1, 2), (-1, 1), (i32::MAX, i32::MIN), (12345, 67890)] {
            let tok = encode_tracking_token(a, b);
            assert_eq!(decode_tracking_token(&tok), Some((a, b)));
        }
    }

    #[test]
    fn tracking_token_garbage_rejected() {
        init_secret("test-secret-key");
        assert_eq!(decode_tracking_token("garbage"), None);
        assert_eq!(decode_tracking_token(""), None);
        let bogus = URL_SAFE_NO_PAD.encode([0u8; 16]);
        assert_eq!(decode_tracking_token(&bogus), None);
    }

    #[test]
    fn deterministic_encrypt() {
        init_secret("test-secret-key");
        let a = encrypt_id(42, "list");
        let b = encrypt_id(42, "list");
        // single-block AES with no IV → deterministic output for same id+tag
        assert_eq!(a, b);
    }

    #[test]
    fn different_tags_produce_different_tokens() {
        init_secret("test-secret-key");
        let a = encrypt_id(42, "list");
        let b = encrypt_id(42, "brand");
        assert_ne!(a, b);
    }
}
