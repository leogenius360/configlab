//! Environment-variable source description and normalization.

use super::{options::SourceOptions, scalar::parse_scalar};
use crate::{
    Config, ConfigError, EnvName, LogicalInput, Operation, Origin, ResolutionLimits, Schema,
    Selector,
};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt::{self, Debug, Formatter};

#[derive(Clone)]
pub(super) enum EnvironmentValues {
    Process,
    Provided(BTreeMap<String, String>),
}

/// Description of an environment-variable source.
#[derive(Clone)]
pub struct EnvironmentSource {
    pub(super) values: EnvironmentValues,
    pub(super) options: SourceOptions,
    pub(super) prefix: Option<String>,
    pub(super) separator: String,
}

impl EnvironmentSource {
    /// Uses the current process environment in the conventional `environment` layer.
    #[must_use]
    pub fn process() -> Self {
        Self {
            values: EnvironmentValues::Process,
            options: SourceOptions::new("environment"),
            prefix: None,
            separator: "_".into(),
        }
    }

    /// Uses deterministic environment values instead of reading the process.
    #[must_use]
    pub fn from<I, K, V>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.values = EnvironmentValues::Provided(
            values
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        );
        self
    }

    /// Assigns this environment source to an explicit layer.
    #[must_use]
    pub fn layer(mut self, layer: impl Into<String>) -> Self {
        self.options.layer = layer.into();
        self
    }

    /// Applies an exact context selector to this environment source.
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

    /// Prefixes automatically generated environment-variable names.
    /// Exact `#[config(env = "NAME")]` bindings are never modified.
    #[must_use]
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Changes the separator between automatically generated name segments.
    #[must_use]
    pub fn separator(mut self, separator: impl Into<String>) -> Self {
        self.separator = separator.into();
        self
    }
}

impl Debug for EnvironmentSource {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let values = match &self.values {
            EnvironmentValues::Process => "process",
            EnvironmentValues::Provided(values) => {
                return formatter
                    .debug_struct("EnvironmentSource")
                    .field(
                        "values",
                        &format_args!("[REDACTED; {} entries]", values.len()),
                    )
                    .field("layer", &self.options.layer)
                    .field("selector", &self.options.selector)
                    .field("order", &self.options.order)
                    .field("prefix", &self.prefix)
                    .field("separator", &self.separator)
                    .finish();
            }
        };
        formatter
            .debug_struct("EnvironmentSource")
            .field("values", &values)
            .field("layer", &self.options.layer)
            .field("selector", &self.options.selector)
            .field("order", &self.options.order)
            .field("prefix", &self.prefix)
            .field("separator", &self.separator)
            .finish()
    }
}

/// Creates a process-environment source description.
#[must_use]
pub fn env() -> EnvironmentSource {
    EnvironmentSource::process()
}

pub(super) fn validate_bindings<T: Config>(
    root_prefix: Option<&str>,
    separator: &str,
) -> Result<(), ConfigError> {
    if separator.is_empty() {
        return Err(ConfigError::Schema(
            "environment separator cannot be empty".into(),
        ));
    }
    let mut names: BTreeMap<String, String> = BTreeMap::new();
    for binding in T::env_bindings("") {
        let name = render_name(binding.name, root_prefix, separator)?;
        let Some(name) = name else { continue };
        if let Some(existing) = names.insert(name.clone(), binding.path.clone())
            && existing != binding.path
        {
            return Err(ConfigError::Schema(format!(
                "environment binding `{name}` is shared by `{existing}` and `{}`",
                binding.path
            )));
        }
    }
    Ok(())
}

pub(super) fn materialize<T: Config>(
    source: &EnvironmentSource,
    index: usize,
    schema: &Schema,
    limits: &ResolutionLimits,
) -> Result<Vec<LogicalInput>, ConfigError> {
    let mut input = LogicalInput::new(
        format!("environment-{index}"),
        source.options.layer.clone(),
        Origin::new("environment"),
    )
    .when(source.options.selector.clone());
    input.order = source.options.order;

    for binding in T::env_bindings("") {
        let Some(name) = render_name(binding.name, source.prefix.as_deref(), &source.separator)?
        else {
            continue;
        };
        let raw = match &source.values {
            EnvironmentValues::Process => process_environment_value(&name)?,
            EnvironmentValues::Provided(values) => values.get(&name).cloned(),
        };
        if let Some(raw) = raw.as_deref() {
            if raw.len() > limits.max_scalar_bytes {
                return Err(ConfigError::Limit {
                    resource: format!("environment binding `{name}` bytes"),
                    limit: limits.max_scalar_bytes,
                });
            }
            schema.get(&binding.path).ok_or_else(|| {
                ConfigError::Schema(format!("env binding references unknown `{}`", binding.path))
            })?;
            input.operations.push(Operation::Set {
                path: binding.path,
                value: parse_scalar(raw),
            });
        }
    }

    Ok(vec![input])
}

fn process_environment_value(name: &str) -> Result<Option<String>, ConfigError> {
    std::env::var_os(name)
        .map(|value| environment_value_to_string(name, value))
        .transpose()
}

fn environment_value_to_string(name: &str, value: OsString) -> Result<String, ConfigError> {
    value.into_string().map_err(|_| {
        ConfigError::Environment(format!(
            "environment binding `{name}` contains a non-Unicode value"
        ))
    })
}

fn render_name(
    name: EnvName,
    root_prefix: Option<&str>,
    separator: &str,
) -> Result<Option<String>, ConfigError> {
    let rendered = match name {
        EnvName::Disabled => return Ok(None),
        EnvName::Exact(name) => name,
        EnvName::Auto(parts) => {
            let mut all = Vec::new();
            if let Some(root) = root_prefix
                && !root.is_empty()
            {
                all.push(root.to_string());
            }
            all.extend(parts);
            all.join(separator)
        }
    };
    if rendered.is_empty() {
        return Err(ConfigError::Schema(
            "environment binding names cannot be empty".into(),
        ));
    }
    if rendered.contains('=') || rendered.contains('\0') {
        return Err(ConfigError::Schema(
            "environment binding names cannot contain `=` or NUL".into(),
        ));
    }
    Ok(Some(rendered))
}

#[cfg(test)]
mod tests {
    use super::environment_value_to_string;
    use std::ffi::OsString;

    #[cfg(unix)]
    #[test]
    fn non_unicode_environment_values_are_reported_without_payload() {
        use std::os::unix::ffi::OsStringExt;

        let error = environment_value_to_string(
            "APP_SECRET",
            OsString::from_vec(vec![b's', b'e', b'c', b'r', b'e', b't', 0xff]),
        )
        .unwrap_err();
        let rendered = error.to_string();
        assert!(rendered.contains("APP_SECRET"));
        assert!(rendered.contains("non-Unicode"));
        assert!(!rendered.contains("secret"));
    }

    #[cfg(windows)]
    #[test]
    fn non_unicode_environment_values_are_reported_without_payload() {
        use std::os::windows::ffi::OsStringExt;

        let error = environment_value_to_string(
            "APP_SECRET",
            OsString::from_wide(&[0x73, 0x65, 0x63, 0x72, 0x65, 0x74, 0xd800]),
        )
        .unwrap_err();
        let rendered = error.to_string();
        assert!(rendered.contains("APP_SECRET"));
        assert!(rendered.contains("non-Unicode"));
        assert!(!rendered.contains("secret"));
    }
}
