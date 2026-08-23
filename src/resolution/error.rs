//! Source-neutral resolver errors.

use std::fmt::{self, Display, Formatter};

/// Deterministic resolver failures that prevent an effective configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResolveError {
    /// Layer names/order are invalid.
    InvalidLayerOrder(String),
    /// An input names a layer not present in the explicit order.
    UnknownLayer(String),
    /// A selector references context that was not stable before resolution.
    UnavailableContext { attribute: String },
    /// An input names a component target not composed by the application schema.
    UnknownTarget(String),
    /// An input targets a setting not declared by the schema.
    UnknownSetting(String),
    /// Definition is internally inconsistent.
    InvalidSchema(String),
    /// Dot-separated setting path is malformed.
    InvalidPath(String),
    /// Equal-layer, equal-specificity peers disagree on one setting.
    Ambiguous {
        path: String,
        layer: String,
        specificity: usize,
        inputs: Vec<String>,
    },
    /// Accumulating peers require an explicit deterministic order.
    UndefinedOrder { path: String, inputs: Vec<String> },
    /// Compatibility fail-fast representation of a missing-required diagnostic.
    MissingRequired(String),
    /// Compatibility fail-fast representation of a value diagnostic.
    InvalidValue { path: String, message: String },
    /// An input attempted a removal form the setting definition does not permit.
    RemovalNotAllowed { path: String, operation: String },
    /// Configured bounded-work policy was exceeded.
    LimitExceeded { resource: String, limit: usize },
}

impl ResolveError {
    /// Stable machine-readable identifier for this resolver failure category.
    ///
    /// The identifier is intended for tests, CLI adapters, and telemetry that
    /// should not depend on human-readable error wording.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidLayerOrder(_) => "invalid_layer_order",
            Self::UnknownLayer(_) => "unknown_layer",
            Self::UnavailableContext { .. } => "unavailable_context",
            Self::UnknownTarget(_) => "unknown_target",
            Self::UnknownSetting(_) => "unknown_setting",
            Self::InvalidSchema(_) => "invalid_schema",
            Self::InvalidPath(_) => "invalid_path",
            Self::Ambiguous { .. } => "ambiguous_peer",
            Self::UndefinedOrder { .. } => "undefined_order",
            Self::MissingRequired(_) => "missing_required",
            Self::InvalidValue { .. } => "invalid_value",
            Self::RemovalNotAllowed { .. } => "removal_not_allowed",
            Self::LimitExceeded { .. } => "limit_exceeded",
        }
    }
}

impl Display for ResolveError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLayerOrder(message) => {
                write!(formatter, "invalid layer order: {message}")
            }
            Self::UnknownLayer(layer) => write!(formatter, "unknown layer `{layer}`"),
            Self::UnavailableContext { attribute } => write!(
                formatter,
                "selector attribute `{attribute}` is unavailable in stable context"
            ),
            Self::UnknownTarget(target) => write!(formatter, "unknown input target `{target}`"),
            Self::UnknownSetting(path) => write!(formatter, "unknown setting `{path}`"),
            Self::InvalidSchema(message) => write!(formatter, "invalid schema: {message}"),
            Self::InvalidPath(path) => write!(formatter, "invalid path `{path}`"),
            Self::Ambiguous {
                path,
                layer,
                specificity,
                inputs,
            } => write!(
                formatter,
                "ambiguous `{path}` in layer `{layer}` at specificity {specificity}: {}",
                inputs.join(", ")
            ),
            Self::UndefinedOrder { path, inputs } => write!(
                formatter,
                "undefined contribution order for `{path}`: {}",
                inputs.join(", ")
            ),
            Self::MissingRequired(path) => {
                write!(formatter, "missing required setting `{path}`")
            }
            Self::InvalidValue { path, message } => {
                write!(formatter, "invalid value for `{path}`: {message}")
            }
            Self::RemovalNotAllowed { path, operation } => write!(
                formatter,
                "removal operation `{operation}` is not permitted for `{path}`"
            ),
            Self::LimitExceeded { resource, limit } => {
                write!(formatter, "configuration {resource} exceeds limit {limit}")
            }
        }
    }
}

impl std::error::Error for ResolveError {}
