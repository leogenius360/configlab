//! Secret-reference and redacted-value wrappers.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Debug, Display, Formatter};

/// Reference to protected material, never the resolved secret payload itself.
///
/// `Debug` and `Display` are redacted by construction. Serde serialization
/// preserves the actual reference identifier and must not be treated as a
/// redacted logging path. `expose()` exists for the owning consumer that must
/// pass the reference to a secret resolver.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretRef(String);

impl SecretRef {
    /// Creates a secret reference identifier.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrows the reference for the owning consumer or secret resolver.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl Debug for SecretRef {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretRef([REDACTED])")
    }
}

impl Display for SecretRef {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

/// Redacting wrapper for sensitive application values.
///
/// Unlike [`SecretRef`], this may contain actual material. The configuration
/// model still recommends references for protected payloads; this wrapper is a
/// defense-in-depth utility for application-owned sensitive values. `Debug` and
/// `Display` redact, while Serde serialization intentionally preserves the
/// wrapped value for configuration encoding.
///
/// `Secret<T>` intentionally does not implement `PartialEq` or `Eq`: ordinary
/// equality is not a suitable authentication operation for secret material.
/// Expose the value only at the trust boundary that needs it and use a
/// domain-appropriate constant-time verifier there.
///
/// This is a redaction wrapper, not a secure-memory container. In particular,
/// it does not zeroize the wrapped value on drop.
///
/// ```compile_fail
/// use configlab::Secret;
///
/// let candidate = Secret::new(b"candidate".to_vec());
/// let expected = Secret::new(b"expected".to_vec());
/// let _authenticated = candidate == expected;
/// ```
#[derive(Clone)]
pub struct Secret<T>(T);

impl<T> Secret<T> {
    /// Wraps a sensitive application-owned value with redacting formatting.
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Borrows the wrapped value for the owning consumer.
    pub fn expose(&self) -> &T {
        &self.0
    }
}

impl<T> Debug for Secret<T> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("Secret([REDACTED])")
    }
}

impl<T> Display for Secret<T> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

impl<T: Serialize> Serialize for Secret<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Secret<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Self)
    }
}
