//! Dispatch from physical source descriptions to source-neutral logical inputs.

use super::{arguments, document::is_internal_path, environment, file, inline};
use crate::{Config, ConfigError, LogicalInput, ResolutionLimits};

#[derive(Clone)]
pub(crate) enum Source {
    File(file::FileSource),
    Inline(inline::InlineSource),
    Environment(environment::EnvironmentSource),
    Arguments(arguments::ArgumentsSource),
    Input(LogicalInput),
}

impl Source {
    pub(crate) fn layer(&self) -> &str {
        match self {
            Self::File(source) => &source.options.layer,
            Self::Inline(source) => &source.options.layer,
            Self::Environment(source) => &source.options.layer,
            Self::Arguments(source) => &source.options.layer,
            Self::Input(input) => &input.layer,
        }
    }
}

pub(crate) fn validate_bindings<T: Config>(sources: &[Source]) -> Result<(), ConfigError> {
    // Generated bindings are definition metadata, so collisions are rejected
    // even when this particular load does not register that source type.
    environment::validate_bindings::<T>(None, "_")?;
    arguments::validate_bindings::<T>()?;

    // Per-source environment naming customizations can create additional
    // collisions and therefore receive the same definition-time validation.
    for source in sources {
        if let Source::Environment(source) = source
            && (source.prefix.is_some() || source.separator != "_")
        {
            environment::validate_bindings::<T>(source.prefix.as_deref(), &source.separator)?;
        }
    }
    Ok(())
}

pub(crate) fn materialize_source<T: Config>(
    source: &Source,
    index: usize,
    schema: &crate::Schema,
    limits: &ResolutionLimits,
) -> Result<Vec<LogicalInput>, ConfigError> {
    match source {
        Source::Input(input) => {
            if input
                .operations
                .iter()
                .any(|operation| is_internal_path(operation.path()))
            {
                return Err(ConfigError::Schema(
                    "internal configuration paths cannot be supplied through raw inputs".into(),
                ));
            }
            Ok(vec![input.clone()])
        }
        Source::File(source) => file::materialize::<T>(source, index, schema, limits),
        Source::Inline(source) => inline::materialize::<T>(source, index, schema, limits),
        Source::Environment(source) => environment::materialize::<T>(source, index, schema, limits),
        Source::Arguments(source) => arguments::materialize::<T>(source, index, schema, limits),
    }
}
