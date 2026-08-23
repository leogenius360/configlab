//! Reusable typed configuration resolution plan.
//!
//! A plan is the immutable semantic boundary shared by one-shot loading and
//! refreshable configuration. It owns the compiled definition, physical source
//! descriptors, validators, context, layer order, and bounded-work policy.

use crate::source::{
    Source, activate_optional_groups, is_internal_path, materialize_source, strip_internal_markers,
};
use crate::{
    Config, ConfigError, ConfigResolutionReport, Context, Diagnostic, EffectiveConfiguration,
    LiveConfig, LoadedConfig, LogicalInput, ResolutionInformation, ResolutionLimits,
    ResolutionReport, ResolveError, Resolver, Schema, Severity, ValidationIssue,
};

pub(crate) type TypedValidator<T> = Box<dyn Fn(&T) -> Vec<ValidationIssue>>;

struct KernelSuccess {
    effective: EffectiveConfiguration,
    diagnostics: Vec<Diagnostic>,
    information: Vec<ResolutionInformation>,
}

/// An immutable, reusable typed resolution plan.
///
/// `ConfigBuilder::plan` compiles schema and source policy without acquiring
/// physical inputs. Every resolution rereads registered sources, while callers
/// may also reconcile complete source-neutral inputs against the same plan.
pub struct ConfigPlan<T: Config> {
    pub(crate) resolver: Resolver,
    pub(crate) context: Context,
    pub(crate) sources: Vec<Source>,
    pub(crate) direct: Option<LogicalInput>,
    pub(crate) validators: Vec<TypedValidator<T>>,
    pub(crate) optional_groups: Vec<String>,
}

impl<T: Config> ConfigPlan<T> {
    /// Rereads registered sources and resolves their current values.
    pub fn resolve(&self) -> Result<LoadedConfig<T>, ConfigError> {
        self.resolve_report()?.into_result()
    }

    /// Rereads registered sources while preserving all diagnostics.
    pub fn resolve_report(&self) -> Result<ConfigResolutionReport<T>, ConfigError> {
        self.resolve_report_with_inputs(&[])
    }

    /// Rereads registered sources and resolves additional source-neutral inputs.
    ///
    /// The additional inputs do not receive implicit precedence. Their
    /// explicitly declared layers and selectors are evaluated by the same
    /// resolver used by one-shot loading.
    pub fn resolve_with_inputs(
        &self,
        inputs: &[LogicalInput],
    ) -> Result<LoadedConfig<T>, ConfigError> {
        self.resolve_report_with_inputs(inputs)?.into_result()
    }

    /// Resolves additional source-neutral inputs and preserves all diagnostics.
    pub fn resolve_report_with_inputs(
        &self,
        inputs: &[LogicalInput],
    ) -> Result<ConfigResolutionReport<T>, ConfigError> {
        let all_inputs = self.materialize_inputs(inputs)?;
        let kernel_report = self.resolve_kernel_report(&all_inputs)?;
        self.finish_report(kernel_report)
    }

    fn materialize_inputs(
        &self,
        runtime_inputs: &[LogicalInput],
    ) -> Result<Vec<LogicalInput>, ConfigError> {
        let mut all_inputs = self.materialize_physical_inputs()?;
        if let Some(direct) = &self.direct {
            all_inputs.push(direct.clone());
        }
        all_inputs.extend(self.materialize_runtime_inputs(runtime_inputs)?);
        Ok(all_inputs)
    }

    fn materialize_physical_inputs(&self) -> Result<Vec<LogicalInput>, ConfigError> {
        let mut all_inputs = Vec::with_capacity(self.sources.len());
        for (index, source) in self.sources.iter().enumerate() {
            let mut produced =
                materialize_source::<T>(source, index, self.schema(), self.limits())?;
            for input in &mut produced {
                activate_optional_groups(input, &self.optional_groups);
            }
            all_inputs.extend(produced);
        }
        Ok(all_inputs)
    }

    fn materialize_runtime_inputs(
        &self,
        inputs: &[LogicalInput],
    ) -> Result<Vec<LogicalInput>, ConfigError> {
        let mut materialized = Vec::with_capacity(inputs.len());
        for input in inputs {
            if input
                .operations
                .iter()
                .any(|operation| is_internal_path(operation.path()))
            {
                return Err(ConfigError::Schema(
                    "internal configuration paths cannot be supplied through runtime inputs".into(),
                ));
            }
            let mut input = input.clone();
            activate_optional_groups(&mut input, &self.optional_groups);
            materialized.push(input);
        }
        Ok(materialized)
    }

    fn resolve_kernel_report(
        &self,
        inputs: &[LogicalInput],
    ) -> Result<ResolutionReport, ConfigError> {
        self.resolver
            .resolve_report(&self.context, inputs)
            .map_err(|error| redact_resolve_error(error, self.schema()))
    }

    fn finish_report(
        &self,
        kernel_report: ResolutionReport,
    ) -> Result<ConfigResolutionReport<T>, ConfigError> {
        let mut kernel = match kernel_success(kernel_report, self.schema()) {
            Ok(kernel) => kernel,
            Err(report) => return Ok(*report),
        };

        let typed = self.decode_effective(&mut kernel.effective)?;
        let issues = self.validation_issues(&typed);
        if issues.iter().any(|issue| issue.severity == Severity::Error) {
            return Ok(validation_failure_report(kernel, issues));
        }

        Ok(self.successful_report(kernel, typed, issues))
    }

    fn decode_effective(&self, effective: &mut EffectiveConfiguration) -> Result<T, ConfigError> {
        let typed =
            T::decode(effective, "").map_err(|error| redact_decode_error(error, self.schema()))?;
        sanitize_effective(effective, &self.optional_groups);
        Ok(typed)
    }

    fn validation_issues(&self, typed: &T) -> Vec<ValidationIssue> {
        let mut issues = T::validate(typed);
        for validator in &self.validators {
            issues.extend(validator(typed));
        }
        issues
    }

    fn successful_report(
        &self,
        kernel: KernelSuccess,
        typed: T,
        issues: Vec<ValidationIssue>,
    ) -> ConfigResolutionReport<T> {
        let warnings = issues
            .iter()
            .filter(|issue| issue.severity == Severity::Warning)
            .cloned()
            .collect();
        let loaded = LoadedConfig {
            value: typed,
            effective: kernel.effective,
            schema: self.schema().clone(),
            warnings,
            optional_groups: self.optional_groups.clone(),
        };
        ConfigResolutionReport {
            loaded: Some(loaded),
            diagnostics: kernel.diagnostics,
            validation: issues,
            information: kernel.information,
            failure: None,
        }
    }

    /// Returns the exact schema used by every evaluation of this plan.
    pub fn schema(&self) -> &Schema {
        self.resolver.schema()
    }

    /// Returns the stable selector context used by every evaluation.
    pub fn context(&self) -> &Context {
        &self.context
    }

    /// Returns the bounded-work policy applied to physical and logical inputs.
    pub fn limits(&self) -> &ResolutionLimits {
        self.resolver.limits()
    }

    /// Resolves the initial value and creates a refreshable configuration.
    pub fn live(self) -> Result<LiveConfig<T>, ConfigError> {
        LiveConfig::new(self)
    }
}

fn kernel_success<T>(
    kernel_report: ResolutionReport,
    schema: &Schema,
) -> Result<KernelSuccess, Box<ConfigResolutionReport<T>>> {
    let failure = kernel_report
        .first_error()
        .map(|error| redact_resolve_error(error, schema));
    let ResolutionReport {
        effective,
        diagnostics,
        information,
    } = kernel_report;

    match effective {
        Some(effective) => Ok(KernelSuccess {
            effective,
            diagnostics,
            information,
        }),
        None => Err(Box::new(ConfigResolutionReport {
            loaded: None,
            diagnostics,
            validation: Vec::new(),
            information,
            failure,
        })),
    }
}

fn validation_failure_report<T>(
    kernel: KernelSuccess,
    issues: Vec<ValidationIssue>,
) -> ConfigResolutionReport<T> {
    ConfigResolutionReport {
        loaded: None,
        diagnostics: kernel.diagnostics,
        validation: issues,
        information: kernel.information,
        failure: None,
    }
}

fn redact_decode_error(error: ConfigError, schema: &Schema) -> ConfigError {
    match error {
        ConfigError::Decode { path, .. } if schema.is_sensitive_path(&path) => {
            ConfigError::Decode {
                path,
                message: "sensitive value is invalid (details redacted)".into(),
            }
        }
        other => other,
    }
}

fn redact_resolve_error(error: ResolveError, schema: &Schema) -> ConfigError {
    match error {
        ResolveError::MissingRequired(path) if is_internal_path(&path) => {
            ConfigError::Resolve(ResolveError::MissingRequired(public_path(&path)))
        }
        ResolveError::InvalidValue { path, .. } if schema.is_sensitive_path(&path) => {
            ConfigError::Resolve(ResolveError::InvalidValue {
                path: public_path(&path),
                message: "sensitive value is invalid (details redacted)".into(),
            })
        }
        ResolveError::InvalidValue { path, message } if is_internal_path(&path) => {
            ConfigError::Resolve(ResolveError::InvalidValue {
                path: public_path(&path),
                message,
            })
        }
        other => ConfigError::Resolve(other),
    }
}

fn public_path(path: &str) -> String {
    path.strip_suffix(".__config_present")
        .unwrap_or(path)
        .to_string()
}

fn sanitize_effective(effective: &mut EffectiveConfiguration, groups: &[String]) {
    strip_internal_markers(&mut effective.values, groups);
    effective
        .provenance
        .retain(|path, _| !is_internal_path(path));
    effective
        .information
        .retain(|information| match information {
            ResolutionInformation::DefaultUsed { path }
            | ResolutionInformation::DerivedDefaultUsed { path, .. }
            | ResolutionInformation::ValueOverridden { path, .. }
            | ResolutionInformation::OptionalOmitted { path }
            | ResolutionInformation::RemovalNoop { path, .. } => !is_internal_path(path),
            ResolutionInformation::InputIgnored { .. } => true,
        });
}
