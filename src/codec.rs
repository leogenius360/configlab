//! Serialization helpers shared with generated derive implementations.

use crate::ConfigError;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

/// Decodes a canonical kernel value into a generated Rust field.
#[doc(hidden)]
pub fn decode<T: DeserializeOwned>(value: &Value, path: &str) -> Result<T, ConfigError> {
    serde_json::from_value(value.clone()).map_err(|error| ConfigError::Decode {
        path: path.into(),
        message: error.to_string(),
    })
}

/// Encodes one Rust value into the canonical kernel tree for generated code.
#[doc(hidden)]
pub fn encode<T: Serialize>(value: &T, path: &str) -> Result<Value, ConfigError> {
    serde_json::to_value(value).map_err(|error| ConfigError::Decode {
        path: path.into(),
        message: error.to_string(),
    })
}

/// Joins generated schema path segments using the kernel dot-path convention.
#[doc(hidden)]
pub fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.into()
    } else {
        format!("{prefix}.{name}")
    }
}
