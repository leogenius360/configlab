//! In-memory structured document source description and materialization.

use super::{document::document_input, format::parse_inline, options::SourceOptions};
use crate::{Config, ConfigError, LogicalInput, Origin, ResolutionLimits, Schema, Selector};
use std::fmt::{self, Debug, Formatter};

#[derive(Clone, Copy, Debug)]
pub(super) enum Format {
    Toml,
    Json,
    Yaml,
}

/// Description of one in-memory configuration document.
#[derive(Clone)]
pub struct InlineSource {
    pub(super) name: String,
    pub(super) text: String,
    pub(super) format: Format,
    pub(super) options: SourceOptions,
}

impl InlineSource {
    fn new(name: impl Into<String>, text: impl Into<String>, format: Format) -> Self {
        Self {
            name: name.into(),
            text: text.into(),
            format,
            options: SourceOptions::new("base"),
        }
    }

    /// Assigns the document to an explicitly declared precedence layer.
    #[must_use]
    pub fn layer(mut self, layer: impl Into<String>) -> Self {
        self.options.layer = layer.into();
        self
    }

    /// Applies an exact context selector to this document.
    #[must_use]
    pub fn when(mut self, selector: Selector) -> Self {
        self.options.selector = selector;
        self
    }

    /// Declares deterministic peer order for accumulating settings.
    #[must_use]
    pub fn order(mut self, order: u32) -> Self {
        self.options.order = Some(order);
        self
    }
}

impl Debug for InlineSource {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InlineSource")
            .field("name", &self.name)
            .field("format", &self.format)
            .field("layer", &self.options.layer)
            .field("selector", &self.options.selector)
            .field("order", &self.options.order)
            .field("text", &"[REDACTED]")
            .finish()
    }
}

/// Creates an inline TOML document.
#[must_use]
pub fn toml(name: impl Into<String>, text: impl Into<String>) -> InlineSource {
    InlineSource::new(name, text, Format::Toml)
}

/// Creates an inline JSON document.
#[must_use]
pub fn json(name: impl Into<String>, text: impl Into<String>) -> InlineSource {
    InlineSource::new(name, text, Format::Json)
}

/// Creates an inline YAML document.
#[must_use]
pub fn yaml(name: impl Into<String>, text: impl Into<String>) -> InlineSource {
    InlineSource::new(name, text, Format::Yaml)
}

pub(super) fn materialize<T: Config>(
    source: &InlineSource,
    index: usize,
    schema: &Schema,
    limits: &ResolutionLimits,
) -> Result<Vec<LogicalInput>, ConfigError> {
    if source.text.len() > limits.max_document_bytes {
        return Err(ConfigError::Limit {
            resource: format!("inline document `{}` bytes", source.name),
            limit: limits.max_document_bytes,
        });
    }
    let value = parse_inline(source.format, &source.name, &source.text)?;
    Ok(vec![document_input::<T>(
        format!("inline-{index}"),
        &source.options.layer,
        source.options.selector.clone(),
        Origin::new(source.name.clone()),
        source.options.order,
        value,
        schema,
    )?])
}
