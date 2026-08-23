//! File source description and materialization.

use super::{document::document_input, format::parse_file, options::SourceOptions};
use crate::{Config, ConfigError, LogicalInput, Origin, ResolutionLimits, Schema, Selector};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Description of one filesystem configuration source.
#[derive(Clone, Debug)]
pub struct FileSource {
    pub(super) path: PathBuf,
    pub(super) options: SourceOptions,
    pub(super) optional: bool,
}

impl FileSource {
    /// Creates a required file in the conventional `base` layer.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            options: SourceOptions::new("base"),
            optional: false,
        }
    }

    /// Assigns the file to an explicitly declared precedence layer.
    #[must_use]
    pub fn layer(mut self, layer: impl Into<String>) -> Self {
        self.options.layer = layer.into();
        self
    }

    /// Makes a missing file a normal no-input outcome.
    #[must_use]
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// Applies an exact context selector to this file input.
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

impl From<PathBuf> for FileSource {
    fn from(value: PathBuf) -> Self {
        Self::new(value)
    }
}

impl From<&Path> for FileSource {
    fn from(value: &Path) -> Self {
        Self::new(value)
    }
}

impl From<&PathBuf> for FileSource {
    fn from(value: &PathBuf) -> Self {
        Self::new(value.clone())
    }
}

impl From<String> for FileSource {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for FileSource {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Creates a configurable file description for use with [`crate::ConfigBuilder::file`].
#[must_use]
pub fn path(value: impl Into<PathBuf>) -> FileSource {
    FileSource::new(value)
}

pub(super) fn materialize<T: Config>(
    source: &FileSource,
    index: usize,
    schema: &Schema,
    limits: &ResolutionLimits,
) -> Result<Vec<LogicalInput>, ConfigError> {
    let file = match std::fs::File::open(&source.path) {
        Ok(file) => file,
        Err(error) if source.optional && error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Vec::new());
        }
        Err(error) => {
            return Err(ConfigError::Io {
                path: source.path.clone(),
                message: error.to_string(),
            });
        }
    };
    let mut text = String::new();
    file.take(limits.max_document_bytes.saturating_add(1) as u64)
        .read_to_string(&mut text)
        .map_err(|error| ConfigError::Io {
            path: source.path.clone(),
            message: error.to_string(),
        })?;
    if text.len() > limits.max_document_bytes {
        return Err(ConfigError::Limit {
            resource: format!("file `{}` bytes", source.path.display()),
            limit: limits.max_document_bytes,
        });
    }
    let value = parse_file(&source.path, &text)?;
    Ok(vec![document_input::<T>(
        format!("file-{index}"),
        &source.options.layer,
        source.options.selector.clone(),
        Origin::new(source.path.display().to_string()),
        source.options.order,
        value,
        schema,
    )?])
}
