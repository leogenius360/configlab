//! Physical source descriptions and normalization into logical inputs.
//!
//! Source descriptions are ordinary owned values. They never resolve
//! precedence; each source only carries explicit layer/selector metadata that
//! the source-neutral resolver consumes later.

mod arguments;
mod document;
mod environment;
mod file;
mod format;
mod inline;
mod materialize;
mod options;
mod scalar;

pub use arguments::{ArgumentsSource, args};
pub use environment::{EnvironmentSource, env};
pub use file::{FileSource, path};
pub use inline::{InlineSource, json, toml, yaml};

pub(crate) use document::{activate_optional_groups, is_internal_path, strip_internal_markers};
pub(crate) use materialize::{Source, materialize_source, validate_bindings};
