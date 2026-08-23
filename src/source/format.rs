//! Feature-gated parsing of supported structured document formats.

use super::inline::Format;
use crate::ConfigError;
use serde_json::Value;
use std::path::Path;

pub(super) fn parse_file(path: &Path, text: &str) -> Result<Value, ConfigError> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "toml" => parse_inline(Format::Toml, &path.display().to_string(), text),
        "json" => parse_inline(Format::Json, &path.display().to_string(), text),
        "yaml" | "yml" => parse_inline(Format::Yaml, &path.display().to_string(), text),
        _ => Err(ConfigError::UnsupportedFormat(path.to_path_buf())),
    }
}

pub(super) fn parse_inline(format: Format, name: &str, text: &str) -> Result<Value, ConfigError> {
    match format {
        Format::Json => {
            #[cfg(feature = "json")]
            {
                serde_json::from_str(text).map_err(|error| ConfigError::Parse {
                    source: name.into(),
                    message: format!(
                        "invalid JSON at line {}, column {}",
                        error.line(),
                        error.column()
                    ),
                })
            }
            #[cfg(not(feature = "json"))]
            {
                let _ = text;
                Err(ConfigError::Parse {
                    source: name.into(),
                    message: "JSON source feature disabled".into(),
                })
            }
        }
        Format::Toml => {
            #[cfg(feature = "toml")]
            {
                let value: toml::Value = toml::from_str(text).map_err(|_| ConfigError::Parse {
                    source: name.into(),
                    message: "invalid TOML configuration".into(),
                })?;
                serde_json::to_value(value).map_err(|_| ConfigError::Parse {
                    source: name.into(),
                    message: "could not normalize TOML configuration".into(),
                })
            }
            #[cfg(not(feature = "toml"))]
            {
                let _ = text;
                Err(ConfigError::Parse {
                    source: name.into(),
                    message: "TOML feature disabled".into(),
                })
            }
        }
        Format::Yaml => {
            #[cfg(feature = "yaml")]
            {
                serde_yaml_ng::from_str(text).map_err(|_| ConfigError::Parse {
                    source: name.into(),
                    message: "invalid YAML configuration".into(),
                })
            }
            #[cfg(not(feature = "yaml"))]
            {
                let _ = text;
                Err(ConfigError::Parse {
                    source: name.into(),
                    message: "YAML feature disabled".into(),
                })
            }
        }
    }
}
