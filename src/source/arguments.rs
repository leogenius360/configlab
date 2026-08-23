//! Command-line argument source description and normalization.

use super::{document::is_internal_path, options::SourceOptions, scalar::parse_scalar};
use crate::{
    CliBinding, Config, ConfigError, LogicalInput, Operation, Origin, ResolutionLimits, Schema,
    Selector,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fmt::{self, Debug, Formatter};

#[derive(Clone)]
pub(super) enum ArgumentValues {
    Process,
    Provided(Vec<String>),
}

/// Description of a command-line argument source.
#[derive(Clone)]
pub struct ArgumentsSource {
    pub(super) values: ArgumentValues,
    pub(super) options: SourceOptions,
    pub(super) strict: bool,
}

impl ArgumentsSource {
    /// Uses the current process arguments in the conventional `invocation` layer.
    #[must_use]
    pub fn process() -> Self {
        Self {
            values: ArgumentValues::Process,
            options: SourceOptions::new("invocation"),
            strict: true,
        }
    }

    /// Uses deterministic arguments instead of reading the process.
    ///
    /// Values correspond to arguments after the executable name; callers should
    /// not include an `argv[0]` program-name element.
    #[must_use]
    pub fn from<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.values = ArgumentValues::Provided(args.into_iter().map(Into::into).collect());
        self
    }

    /// Assigns this argument source to an explicit layer.
    #[must_use]
    pub fn layer(mut self, layer: impl Into<String>) -> Self {
        self.options.layer = layer.into();
        self
    }

    /// Applies an exact context selector to this argument source.
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

    /// Allows arguments unrelated to generated ConfigLab bindings.
    ///
    /// Strict parsing is the default so misspelled configuration flags do not
    /// silently fall back to defaults. Use this only when another parser owns
    /// the remaining process arguments.
    #[must_use]
    pub fn ignore_unknown(mut self) -> Self {
        self.strict = false;
        self
    }
}

impl Debug for ArgumentsSource {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match &self.values {
            ArgumentValues::Process => formatter
                .debug_struct("ArgumentsSource")
                .field("values", &"process")
                .field("layer", &self.options.layer)
                .field("selector", &self.options.selector)
                .field("order", &self.options.order)
                .field("strict", &self.strict)
                .finish(),
            ArgumentValues::Provided(values) => formatter
                .debug_struct("ArgumentsSource")
                .field(
                    "values",
                    &format_args!("[REDACTED; {} arguments]", values.len()),
                )
                .field("layer", &self.options.layer)
                .field("selector", &self.options.selector)
                .field("order", &self.options.order)
                .field("strict", &self.strict)
                .finish(),
        }
    }
}

/// Creates a process-argument source description.
#[must_use]
pub fn args() -> ArgumentsSource {
    ArgumentsSource::process()
}

pub(super) fn validate_bindings<T: Config>() -> Result<(), ConfigError> {
    let mut flags: BTreeMap<String, String> = BTreeMap::new();
    for binding in T::cli_bindings("") {
        if !valid_cli_flag(&binding.flag) {
            return Err(ConfigError::Schema(format!(
                "invalid CLI flag for `{}`; names must start with an ASCII letter or digit and contain only ASCII letters, digits, `-`, or `_`",
                binding.path
            )));
        }
        if matches!(
            binding.flag.as_str(),
            "config" | "config-unset" | "config-file"
        ) {
            return Err(ConfigError::Schema(format!(
                "CLI flag `--{}` is reserved",
                binding.flag
            )));
        }
        if let Some(existing) = flags.insert(binding.flag.clone(), binding.path.clone())
            && existing != binding.path
        {
            return Err(ConfigError::Schema(format!(
                "CLI flag `--{}` is shared by `{existing}` and `{}`",
                binding.flag, binding.path
            )));
        }
    }
    Ok(())
}

pub(super) fn materialize<T: Config>(
    source: &ArgumentsSource,
    source_index: usize,
    schema: &Schema,
    limits: &ResolutionLimits,
) -> Result<Vec<LogicalInput>, ConfigError> {
    let process_values;
    let values = match &source.values {
        ArgumentValues::Process => {
            process_values = process_arguments()?;
            process_values.as_slice()
        }
        ArgumentValues::Provided(values) => values.as_slice(),
    };
    validate_argument_limits(values, limits)?;

    let mut input = argument_input(source, source_index);
    let bindings = collect_bindings::<T>();
    let mut seen_paths = BTreeSet::new();
    let mut position = 0;

    while position < values.len() {
        let parsed = parse_argument(
            &values[position],
            values.get(position + 1),
            position,
            source.strict,
            schema,
            &bindings,
        )?;
        if let Some(operation) = parsed.operation {
            push_unique_operation(&mut input, &mut seen_paths, operation)?;
        }
        position += 1 + usize::from(parsed.consumed_next);
    }

    Ok(vec![input])
}

fn validate_argument_limits(
    values: &[String],
    limits: &ResolutionLimits,
) -> Result<(), ConfigError> {
    if values.len() > limits.max_arguments {
        return Err(ConfigError::Limit {
            resource: "argument count".into(),
            limit: limits.max_arguments,
        });
    }
    if values
        .iter()
        .any(|argument| argument.len() > limits.max_scalar_bytes)
    {
        return Err(ConfigError::Limit {
            resource: "argument bytes".into(),
            limit: limits.max_scalar_bytes,
        });
    }
    Ok(())
}

fn argument_input(source: &ArgumentsSource, source_index: usize) -> LogicalInput {
    let mut input = LogicalInput::new(
        format!("arguments-{source_index}"),
        source.options.layer.clone(),
        Origin::new("arguments"),
    )
    .when(source.options.selector.clone());
    input.order = source.options.order;
    input
}

struct ParsedArgument {
    operation: Option<Operation>,
    consumed_next: bool,
}

impl ParsedArgument {
    fn ignored() -> Self {
        Self {
            operation: None,
            consumed_next: false,
        }
    }

    fn operation(operation: Operation, consumed_next: bool) -> Self {
        Self {
            operation: Some(operation),
            consumed_next,
        }
    }
}

fn parse_argument(
    argument: &str,
    next: Option<&String>,
    position: usize,
    strict: bool,
    schema: &Schema,
    bindings: &BTreeMap<String, CliBinding>,
) -> Result<ParsedArgument, ConfigError> {
    if argument == "--config" {
        let pair =
            next.ok_or_else(|| ConfigError::Argument("--config requires path=value".into()))?;
        return Ok(ParsedArgument::operation(
            argument_pair_operation(pair, schema)?,
            true,
        ));
    }
    if let Some(pair) = argument.strip_prefix("--config=") {
        return Ok(ParsedArgument::operation(
            argument_pair_operation(pair, schema)?,
            false,
        ));
    }
    if argument == "--config-unset" {
        let path =
            next.ok_or_else(|| ConfigError::Argument("--config-unset requires a path".into()))?;
        return Ok(ParsedArgument::operation(
            unset_operation(path, schema)?,
            true,
        ));
    }
    if let Some(flag) = argument.strip_prefix("--no-") {
        return parse_negated_argument(flag, strict, bindings);
    }
    if let Some(name) = argument.strip_prefix("--") {
        return parse_named_argument(name, next, strict, schema, bindings);
    }
    if strict {
        return Err(ConfigError::Argument(format!(
            "unexpected positional argument at position {}",
            position + 1
        )));
    }
    Ok(ParsedArgument::ignored())
}

fn parse_negated_argument(
    flag: &str,
    strict: bool,
    bindings: &BTreeMap<String, CliBinding>,
) -> Result<ParsedArgument, ConfigError> {
    if let Some(binding) = bindings.get(flag)
        && binding.boolean
    {
        return Ok(ParsedArgument::operation(
            Operation::Set {
                path: binding.path.clone(),
                value: Value::Bool(false),
            },
            false,
        ));
    }
    if strict {
        return Err(ConfigError::Argument(format!(
            "unknown boolean configuration flag `--no-{flag}`"
        )));
    }
    Ok(ParsedArgument::ignored())
}

fn parse_named_argument(
    name: &str,
    next: Option<&String>,
    strict: bool,
    schema: &Schema,
    bindings: &BTreeMap<String, CliBinding>,
) -> Result<ParsedArgument, ConfigError> {
    let (name, inline) = name
        .split_once('=')
        .map(|(name, value)| (name, Some(value)))
        .unwrap_or((name, None));
    let Some(binding) = bindings.get(name) else {
        if strict {
            return Err(ConfigError::Argument(format!(
                "unknown configuration flag `--{name}`"
            )));
        }
        return Ok(ParsedArgument::ignored());
    };

    if binding.boolean && inline.is_none() {
        return Ok(ParsedArgument::operation(
            Operation::Set {
                path: binding.path.clone(),
                value: Value::Bool(true),
            },
            false,
        ));
    }

    let (raw, consumed_next) = match inline {
        Some(value) => (value, false),
        None => (
            next.ok_or_else(|| ConfigError::Argument(format!("--{name} requires a value")))?
                .as_str(),
            true,
        ),
    };
    schema
        .get(&binding.path)
        .ok_or_else(|| ConfigError::Argument(format!("unknown path `{}`", binding.path)))?;
    Ok(ParsedArgument::operation(
        Operation::Set {
            path: binding.path.clone(),
            value: parse_scalar(raw),
        },
        consumed_next,
    ))
}

fn unset_operation(path: &str, schema: &Schema) -> Result<Operation, ConfigError> {
    if is_internal_path(path) {
        return Err(ConfigError::Argument(
            "internal configuration paths cannot be unset".into(),
        ));
    }
    schema
        .get(path)
        .ok_or_else(|| ConfigError::Argument(format!("unknown path `{path}`")))?;
    Ok(Operation::Unset { path: path.into() })
}

fn process_arguments() -> Result<Vec<String>, ConfigError> {
    std::env::args_os()
        .skip(1)
        .enumerate()
        .map(|(index, value)| argument_to_string(index + 1, value))
        .collect()
}

fn argument_to_string(position: usize, value: OsString) -> Result<String, ConfigError> {
    value.into_string().map_err(|_| {
        ConfigError::Argument(format!("process argument {position} is not valid Unicode"))
    })
}

fn argument_pair_operation(pair: &str, schema: &Schema) -> Result<Operation, ConfigError> {
    let (path, raw) = pair
        .split_once('=')
        .ok_or_else(|| ConfigError::Argument("--config requires path=value".into()))?;
    if is_internal_path(path) {
        return Err(ConfigError::Argument(
            "internal configuration paths cannot be set".into(),
        ));
    }
    schema
        .get(path)
        .ok_or_else(|| ConfigError::Argument(format!("unknown path `{path}`")))?;
    Ok(Operation::Set {
        path: path.into(),
        value: parse_scalar(raw),
    })
}

fn push_unique_operation(
    input: &mut LogicalInput,
    seen_paths: &mut BTreeSet<String>,
    operation: Operation,
) -> Result<(), ConfigError> {
    let path = operation.path().to_string();
    if !seen_paths.insert(path.clone()) {
        return Err(ConfigError::Argument(format!(
            "configuration path `{path}` was supplied more than once"
        )));
    }
    input.operations.push(operation);
    Ok(())
}

fn valid_cli_flag(flag: &str) -> bool {
    let mut characters = flag.chars();
    characters
        .next()
        .is_some_and(|character| character.is_ascii_alphanumeric())
        && characters
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

fn collect_bindings<T: Config>() -> BTreeMap<String, CliBinding> {
    T::cli_bindings("")
        .into_iter()
        .map(|binding| (binding.flag.clone(), binding))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::argument_to_string;
    use std::ffi::OsString;

    #[cfg(unix)]
    #[test]
    fn non_unicode_process_arguments_are_reported_without_payload() {
        use std::os::unix::ffi::OsStringExt;

        let error = argument_to_string(
            3,
            OsString::from_vec(vec![b's', b'e', b'c', b'r', b'e', b't', 0xff]),
        )
        .unwrap_err();
        let rendered = error.to_string();
        assert!(rendered.contains("argument 3"));
        assert!(rendered.contains("Unicode"));
        assert!(!rendered.contains("secret"));
    }

    #[cfg(windows)]
    #[test]
    fn non_unicode_process_arguments_are_reported_without_payload() {
        use std::os::windows::ffi::OsStringExt;

        let error = argument_to_string(
            3,
            OsString::from_wide(&[0x73, 0x65, 0x63, 0x72, 0x65, 0x74, 0xd800]),
        )
        .unwrap_err();
        let rendered = error.to_string();
        assert!(rendered.contains("argument 3"));
        assert!(rendered.contains("Unicode"));
        assert!(!rendered.contains("secret"));
    }
}
