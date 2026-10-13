//! Type-safe opaque resource identifiers.
//!
//! `OpaqueId<Tag>` wraps an internal `i32` row id and exposes it externally
//! as an AES-encrypted base64url token. The phantom `Tag` provides domain
//! separation: a token issued for [`Brand`] cannot be used in place of a
//! [`List`] token even by callers who hold both — the cipher key is derived
//! from `secret_key || tag` and the round-trip will fail.

use std::{fmt, marker::PhantomData, str::FromStr};

use serde::{de::Error as DeError, Deserialize, Deserializer, Serialize, Serializer};

use crate::utils::crypto;

/// Marker trait carrying the per-resource cipher tag.
pub trait OpaqueTag {
    const TAG: &'static str;
}

macro_rules! define_tag {
    ($(#[$m:meta])* $name:ident => $tag:literal) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name;

        impl OpaqueTag for $name {
            const TAG: &'static str = $tag;
        }
    };
}

define_tag!(
    /// Tag for `apps` rows.
    Brand    => "brand"
);
define_tag!(
    /// Tag for `lists` rows.
    List     => "list"
);
define_tag!(
    /// Tag for `campaigns` rows.
    Campaign => "campaign"
);
define_tag!(
    /// Tag for `seg` rows.
    Segment  => "segment"
);
define_tag!(
    /// Tag for `subscribers` rows.
    Subscriber => "subscriber"
);
define_tag!(
    /// Tag for `links` rows.
    Link => "link"
);
define_tag!(
    /// Tag for `template` rows.
    Template => "template"
);

/// Opaque public identifier for a resource of type `Tag`.
#[derive(Default)]
pub struct OpaqueId<Tag> {
    raw: i32,
    _t: PhantomData<fn() -> Tag>,
}

impl<T> OpaqueId<T> {
    /// Wrap a raw row id without performing any encoding work.
    pub const fn new(raw: i32) -> Self {
        Self {
            raw,
            _t: PhantomData,
        }
    }

    /// Unwrap the inner row id. Use only inside the data-access layer.
    pub const fn raw(&self) -> i32 {
        self.raw
    }
}

// Manual derives so the `Tag` parameter doesn't need to satisfy them.
impl<T> Clone for OpaqueId<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for OpaqueId<T> {}
impl<T> PartialEq for OpaqueId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}
impl<T> Eq for OpaqueId<T> {}
impl<T> std::hash::Hash for OpaqueId<T> {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.raw.hash(h);
    }
}
impl<T> fmt::Debug for OpaqueId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OpaqueId({})", self.raw)
    }
}

impl<T: OpaqueTag> fmt::Display for OpaqueId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&crypto::encrypt_id(self.raw, T::TAG))
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid opaque id token")]
pub struct OpaqueIdError;

impl<T: OpaqueTag> FromStr for OpaqueId<T> {
    type Err = OpaqueIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        crypto::decrypt_id(s, T::TAG)
            .map(OpaqueId::new)
            .ok_or(OpaqueIdError)
    }
}

impl<T: OpaqueTag> Serialize for OpaqueId<T> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.collect_str(&crypto::encrypt_id(self.raw, T::TAG))
    }
}

impl<'de, T: OpaqueTag> Deserialize<'de> for OpaqueId<T> {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = String::deserialize(de)?;
        s.parse().map_err(D::Error::custom)
    }
}

impl<T: OpaqueTag> schemars::JsonSchema for OpaqueId<T> {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("OpaqueId")
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        String::json_schema(generator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_via_serde() {
        crypto::init_secret("test-secret");
        let id: OpaqueId<List> = OpaqueId::new(42);
        let tok = id.to_string();
        let parsed: OpaqueId<List> = tok.parse().unwrap();
        assert_eq!(parsed.raw(), 42);
    }

    #[test]
    fn cross_tag_rejected() {
        crypto::init_secret("test-secret");
        let id: OpaqueId<Brand> = OpaqueId::new(7);
        let tok = id.to_string();
        assert!(tok.parse::<OpaqueId<List>>().is_err());
    }
}
