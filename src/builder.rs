//! Typed configuration builder and orchestration of source normalization.
//!
//! This module does not implement resolution semantics. It validates facade
//! metadata, normalizes physical sources into logical inputs, and delegates to
//! [`crate::Resolver`].

use crate::codec::encode;
use crate::source::{
    ArgumentsSource, EnvironmentSource, FileSource, InlineSource, Source, activate_optional_groups,
    is_internal_path, validate_bindings,
};
use crate::{
    Config, ConfigError, ConfigPlan, ConfigResolutionReport, Context, DefaultRule, LayerOrder,
    LiveConfig, LoadedConfig, LogicalInput, Operation, Origin, ResolutionLimits, ResolutionRequest,
    Resolver, Schema, SettingSpec, ValidationIssue,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::path::Path;

use crate::plan::TypedValidator;

type DerivedDefault = (String, String, BTreeMap<String, Value>);

/// Builds one typed configuration resolution.
///
/// Physical sources are described by values such as [`crate::FileSource`],
/// [`crate::EnvironmentSource`], and [`crate::ArgumentsSource`]. The builder
/// validates their declared layers, materializes them into logical inputs, and
/// delegates all precedence/selector/merge behavior to the single resolver.
pub struct ConfigBuilder<T: Config> {
    layers: LayerOrder,
    request: ResolutionRequest,
    context: Context,
    sources: Vec<Source>,
    direct: Vec<Operation>,
    derived: Vec<DerivedDefault>,
    validators: Vec<TypedValidator<T>>,
    limits: ResolutionLimits,
    _marker: PhantomData<T>,
}

impl<T: Config> Default for ConfigBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Config> ConfigBuilder<T> {
    /// Creates a builder using the conventional explicit local layer order.
    #[must_use]
    pub fn new() -> Self {
        Self {
            layers: LayerOrder::conventional(),
            request: ResolutionRequest::new(),
            context: Context::new(),
            sources: Vec::new(),
            direct: Vec::new(),
            derived: Vec::new(),
            validators: Vec::new(),
            limits: ResolutionLimits::default(),
            _marker: PhantomData,
        }
    }

    /// Replaces the application's explicit low-to-high precedence order.
    ///
    /// This is an advanced customization point. Ordinary applications can use
    /// the conventional order and override individual source placement through
    /// `.layer(...)` on source descriptions.
    pub fn layer_order<I, S>(mut self, names: I) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.layers = LayerOrder::new(names)?;
        Ok(self)
    }

    /// Adds one stable context attribute used by exact selectors.
    #[must_use]
    pub fn context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key, value);
        self
    }

    /// Supplies raw pre-discovery request facts.
    #[must_use]
    pub fn resolution_request(mut self, request: ResolutionRequest) -> Self {
        self.request = request;
        self
    }

    /// Adds one raw fact to the pre-discovery request.
    #[must_use]
    pub fn request_fact(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.request.insert(key, value);
        self
    }

    /// Replaces the bounded-work policy for source loading and resolution.
    pub fn limits(mut self, limits: ResolutionLimits) -> Result<Self, ConfigError> {
        limits
            .validate()
            .map_err(|message| ConfigError::Schema(message.into()))?;
        self.limits = limits;
        Ok(self)
    }

    /// Promotes an already-known request fact into stable selector context.
    pub fn context_from_request(
        mut self,
        context_key: impl Into<String>,
        fact_key: &str,
    ) -> Result<Self, ConfigError> {
        let value = self
            .request
            .get(fact_key)
            .ok_or_else(|| {
                ConfigError::Schema(format!(
                    "resolution request fact `{fact_key}` is unavailable"
                ))
            })?
            .to_string();
        self.context.insert(context_key, value);
        Ok(self)
    }

    /// Registers a file source.
    ///
    /// A bare string/path is shorthand for a required file in the conventional
    /// `base` layer. Use [`crate::path`] when the source needs modifiers:
    ///
    /// ```no_run
    /// # use configlab::{path, selector, Config};
    /// # #[derive(Config)] struct App { #[config(default = 1)] value: u16 }
    /// let builder = App::builder().file(
    ///     path("production.toml")
    ///         .layer("contextual")
    ///         .optional()
    ///         .when(selector!(environment = "production")),
    /// );
    /// # let _ = builder;
    /// ```
    #[must_use]
    pub fn file(mut self, source: impl Into<FileSource>) -> Self {
        self.sources.push(Source::File(source.into()));
        self
    }

    /// Registers an in-memory TOML/JSON/YAML document description.
    #[must_use]
    pub fn inline(mut self, source: InlineSource) -> Self {
        self.sources.push(Source::Inline(source));
        self
    }

    /// Registers an environment-variable source description.
    #[must_use]
    pub fn environment(mut self, source: EnvironmentSource) -> Self {
        self.sources.push(Source::Environment(source));
        self
    }

    /// Registers a command-line argument source description.
    #[must_use]
    pub fn arguments(mut self, source: ArgumentsSource) -> Self {
        self.sources.push(Source::Arguments(source));
        self
    }

    /// Adds a fully normalized logical input for advanced integrations.
    #[must_use]
    pub fn input(mut self, input: LogicalInput) -> Self {
        self.sources.push(Source::Input(input));
        self
    }

    /// Adds a direct replacement/merge operation in the conventional `override` layer.
    pub fn set<V: Serialize>(
        mut self,
        path: impl Into<String>,
        value: V,
    ) -> Result<Self, ConfigError> {
        let path = path.into();
        if is_internal_path(&path) {
            return Err(ConfigError::Schema(
                "internal configuration paths cannot be overridden directly".into(),
            ));
        }
        let value = encode(&value, &path)?;
        self.direct.push(Operation::Set { path, value });
        Ok(self)
    }

    /// Removes a whole setting in the conventional `override` layer.
    #[must_use]
    pub fn unset(mut self, path: impl Into<String>) -> Self {
        self.direct.push(Operation::Unset { path: path.into() });
        self
    }

    /// Replaces one generated fixed default with a declarative lookup-table default.
    pub fn derived_default<I, K, V>(
        mut self,
        path: impl Into<String>,
        source: impl Into<String>,
        cases: I,
    ) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Serialize,
    {
        let path = path.into();
        let source = source.into();
        if is_internal_path(&path) || is_internal_path(&source) {
            return Err(ConfigError::Schema(
                "derived defaults cannot reference internal configuration paths".into(),
            ));
        }
        if self
            .derived
            .iter()
            .any(|(existing_path, _, _)| existing_path == &path)
        {
            return Err(ConfigError::Schema(format!(
                "derived default for `{path}` is configured more than once"
            )));
        }
        let mut encoded_cases = BTreeMap::new();
        for (key, value) in cases {
            let key = key.into();
            if encoded_cases.contains_key(&key) {
                return Err(ConfigError::Schema(format!(
                    "derived default for `{path}` contains duplicate lookup cases"
                )));
            }
            encoded_cases.insert(key, encode(&value, &path)?);
        }
        self.derived.push((path, source, encoded_cases));
        Ok(self)
    }

    /// Discovers an ancestor from a request fact such as `working_directory`.
    pub fn discover_ancestor_from_request(
        self,
        fact_key: &str,
        marker: &str,
        config_file: &str,
        layer: &str,
    ) -> Result<Self, ConfigError> {
        let start = self
            .request
            .get(fact_key)
            .ok_or_else(|| {
                ConfigError::Schema(format!(
                    "resolution request fact `{fact_key}` is unavailable"
                ))
            })?
            .to_string();
        self.discover_ancestor(start, marker, config_file, layer)
    }

    /// Walks ancestors, establishes `project_root`, and registers its required config file.
    pub fn discover_ancestor(
        mut self,
        start: impl AsRef<Path>,
        marker: &str,
        config_file: &str,
        layer: &str,
    ) -> Result<Self, ConfigError> {
        let start = start.as_ref();
        let mut current = start.canonicalize().map_err(|error| ConfigError::Io {
            path: start.to_path_buf(),
            message: error.to_string(),
        })?;

        loop {
            if current.join(marker).exists() {
                self.context.insert("project_root", context_path(&current));
                self.sources.push(Source::File(
                    FileSource::new(current.join(config_file)).layer(layer),
                ));
                return Ok(self);
            }
            if !current.pop() {
                return Err(ConfigError::Io {
                    path: start.to_path_buf(),
                    message: format!("could not discover ancestor containing `{marker}`"),
                });
            }
        }
    }

    /// Adds Rust/application-specific post-decode validation.
    ///
    /// Prefer declarative [`crate::ValidationRule`] data when the invariant can be
    /// represented in the language-agnostic definition model.
    #[must_use]
    pub fn validate_with<F>(mut self, validator: F) -> Self
    where
        F: Fn(&T) -> Vec<ValidationIssue> + 'static,
    {
        self.validators.push(Box::new(validator));
        self
    }

    /// Resolves and returns only the typed application configuration.
    pub fn load(self) -> Result<T, ConfigError> {
        self.resolve().map(LoadedConfig::into_inner)
    }

    /// Resolves and returns the typed value plus explainability metadata.
    pub fn resolve(self) -> Result<LoadedConfig<T>, ConfigError> {
        self.resolve_report()?.into_result()
    }

    /// Resolves configuration while preserving all blocking kernel diagnostics.
    ///
    /// Failures to acquire or parse sources, construct the schema, or execute
    /// deterministic resolution remain `Err`. Definition and typed-validation
    /// failures return a report with no loaded configuration so applications can
    /// present every actionable issue in one pass.
    pub fn resolve_report(self) -> Result<ConfigResolutionReport<T>, ConfigError> {
        self.plan()?.resolve_report()
    }

    /// Creates a refreshable configuration that retains all source descriptors.
    ///
    /// The initial value is resolved immediately. Each subsequent
    /// [`LiveConfig::refresh`] call rereads every registered physical source and
    /// replaces the current typed value after successful validation.
    pub fn live(self) -> Result<LiveConfig<T>, ConfigError> {
        self.plan()?.live()
    }

    /// Compiles this builder into a reusable resolution plan.
    ///
    /// The resulting plan retains physical source descriptors and rereads them
    /// for every resolution. Additional source-neutral inputs use the same
    /// precedence and validation implementation.
    pub fn plan(mut self) -> Result<ConfigPlan<T>, ConfigError> {
        let mut schema = T::schema("")?;
        validate_bindings::<T>(&self.sources)?;
        for source in &self.sources {
            self.layers.rank(source.layer())?;
        }

        for (path, source, cases) in self.derived.drain(..) {
            let mut setting = schema.get(&path).cloned().ok_or_else(|| {
                ConfigError::Schema(format!("unknown derived-default path `{path}`"))
            })?;
            setting.default = Some(DefaultRule::Lookup { source, cases });
            replace_setting(&mut schema, setting)?;
        }

        let groups = T::optional_groups("");
        let direct = materialize_direct_input(&schema, &groups, self.direct)?;

        let resolver = Resolver::with_limits(schema, self.layers, self.limits)?;

        Ok(ConfigPlan {
            resolver,
            context: self.context,
            sources: self.sources,
            direct,
            validators: self.validators,
            optional_groups: groups,
        })
    }
}

fn materialize_direct_input(
    schema: &Schema,
    groups: &[String],
    operations: Vec<Operation>,
) -> Result<Option<LogicalInput>, ConfigError> {
    if operations.is_empty() {
        return Ok(None);
    }

    let mut input = LogicalInput::new(
        "direct-overrides",
        "override",
        Origin::new("builder override"),
    );

    for operation in operations {
        if is_internal_path(operation.path()) {
            return Err(ConfigError::Schema(
                "internal configuration paths cannot be overridden directly".into(),
            ));
        }

        match operation {
            Operation::Unset { path } if schema.get(&path).is_none() => {
                let group = groups.iter().any(|group| group == &path);
                let descendants = if group {
                    schema
                        .iter()
                        .map(|(path, _)| path.to_string())
                        .filter(|candidate| candidate.starts_with(&format!("{path}.")))
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };

                if descendants.is_empty() {
                    input.operations.push(Operation::Unset { path });
                } else {
                    input.operations.extend(
                        descendants
                            .into_iter()
                            .map(|path| Operation::Unset { path }),
                    );
                }
            }
            other => input.operations.push(other),
        }
    }

    activate_optional_groups(&mut input, groups);
    Ok(Some(input))
}

fn context_path(path: &Path) -> String {
    let rendered = path.to_string_lossy();
    #[cfg(windows)]
    {
        if let Some(rest) = rendered.strip_prefix("\\\\?\\UNC\\") {
            return format!("\\\\{rest}");
        }
        if let Some(rest) = rendered.strip_prefix("\\\\?\\") {
            return rest.to_string();
        }
    }
    rendered.into_owned()
}

fn replace_setting(schema: &mut Schema, setting: SettingSpec) -> Result<(), ConfigError> {
    schema.replace(setting)?;
    Ok(())
}
