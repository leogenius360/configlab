//! User-facing facade errors.

use crate::{ResolveError, ValidationIssue};
use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

/// Errors produced while building, loading, resolving, decoding, or validating typed configuration.
#[derive(Debug)]
pub enum ConfigError {
    /// Source-agnostic kernel resolution failure.
    Resolve(ResolveError),
    /// Local filesystem/discovery failure.
    Io { path: PathBuf, message: String },
    /// File extension is not enabled/supported.
    UnsupportedFormat(PathBuf),
    /// Structured source failed to parse or contained unknown configuration paths.
    Parse { source: String, message: String },
    /// Definition-valid kernel value could not be decoded into the Rust field type.
    Decode { path: String, message: String },
    /// Typed application validation failed after decoding.
    Validation(Vec<ValidationIssue>),
    /// Generated or builder-customized definition is invalid.
    Schema(String),
    /// Generic or field-specific command-line configuration is invalid.
    Argument(String),
    /// Environment mapping or value conversion is invalid.
    Environment(String),
    /// Physical source exceeded the configured bounded-work policy.
    Limit { resource: String, limit: usize },
}

impl ConfigError {
    /// Stable machine-readable identifier for this facade failure category.
    ///
    /// Resolver failures retain their more specific [`ResolveError::code`].
    pub fn code(&self) -> &'static str {
        match self {
            Self::Resolve(error) => error.code(),
            Self::Io { .. } => "io",
            Self::UnsupportedFormat(_) => "unsupported_format",
            Self::Parse { .. } => "parse",
            Self::Decode { .. } => "decode",
            Self::Validation(_) => "typed_validation",
            Self::Schema(_) => "schema",
            Self::Argument(_) => "argument",
            Self::Environment(_) => "environment",
            Self::Limit { .. } => "limit_exceeded",
        }
    }
}

impl Display for ConfigError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Resolve(error) => Display::fmt(error, formatter),
            Self::Io { path, message } => {
                write!(formatter, "failed to read {}: {message}", path.display())
            }
            Self::UnsupportedFormat(path) => {
                write!(formatter, "unsupported config format: {}", path.display())
            }
            Self::Parse { source, message } => {
                write!(formatter, "failed to parse {source}: {message}")
            }
            Self::Decode { path, message } => {
                write!(formatter, "invalid value for `{path}`: {message}")
            }
            Self::Validation(issues) => {
                write!(formatter, "configuration validation failed")?;
                for issue in issues {
                    let prefix = issue
                        .path
                        .as_deref()
                        .map(|path| format!("{path}: "))
                        .unwrap_or_default();
                    write!(formatter, "; {prefix}{}", issue.message)?;
                }
                Ok(())
            }
            Self::Schema(message) => write!(formatter, "invalid configuration schema: {message}"),
            Self::Argument(message) => {
                write!(formatter, "invalid configuration argument: {message}")
            }
            Self::Environment(message) => {
                write!(formatter, "invalid environment configuration: {message}")
            }
            Self::Limit { resource, limit } => {
                write!(formatter, "configuration {resource} exceeds limit {limit}")
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Resolve(error) => Some(error),
            Self::Io { .. }
            | Self::UnsupportedFormat(_)
            | Self::Parse { .. }
            | Self::Decode { .. }
            | Self::Validation(_)
            | Self::Schema(_)
            | Self::Argument(_)
            | Self::Environment(_)
            | Self::Limit { .. } => None,
        }
    }
}

impl From<ResolveError> for ConfigError {
    fn from(value: ResolveError) -> Self {
        Self::Resolve(value)
    }
}
