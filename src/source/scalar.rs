//! Conversion of physical string values into canonical JSON scalar values.

use serde_json::Value;

pub(super) fn parse_scalar(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.into()))
}
