//! Structured resolution report types.

use crate::{
    ConfigError, Diagnostic, DiagnosticCode, DiagnosticSeverity, EffectiveConfiguration,
    LoadedConfig, ResolutionInformation, ResolveError, Severity, ValidationIssue,
};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug, Formatter};

/// Resolved presence/enablement state of a component key.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComponentPresence {
    /// The application definition does not compose this component key.
    Absent,
    /// The component is composed but its enablement condition is false.
    Disabled,
    /// The component is composed and enabled.
    Enabled,
}

/// Full structured result of one resolution attempt.
///
/// Blocking diagnostics result in `effective == None`. Warnings preserve a
/// successful effective configuration.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolutionReport {
    /// Effective configuration when no blocking diagnostics were produced.
    pub effective: Option<EffectiveConfiguration>,
    /// Definition-validation diagnostics produced for the complete candidate.
    pub diagnostics: Vec<Diagnostic>,
    /// Non-problem resolution information such as defaults and selector misses.
    pub information: Vec<ResolutionInformation>,
}

impl Debug for ResolutionReport {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResolutionReport")
            .field("effective", &self.effective)
            .field("diagnostics", &self.diagnostics)
            .field("information", &self.information)
            .finish()
    }
}

impl ResolutionReport {
    /// Returns true when the report contains no blocking diagnostics.
    pub fn is_success(&self) -> bool {
        self.effective.is_some()
            && !self
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
    }

    /// Iterates non-blocking diagnostics.
    pub fn warnings(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Warning)
    }

    /// Converts the first blocking diagnostic into the compatibility
    /// [`ResolveError`] used by [`crate::Resolver::resolve`].
    pub(crate) fn first_error(&self) -> Option<ResolveError> {
        let diagnostic = self
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)?;

        match diagnostic.code {
            DiagnosticCode::MissingRequired => diagnostic
                .path
                .as_ref()
                .map(|path| ResolveError::MissingRequired(path.clone())),
            DiagnosticCode::InvalidType
            | DiagnosticCode::ConstraintViolation
            | DiagnosticCode::InvalidRepresentation
            | DiagnosticCode::RuleViolation => Some(ResolveError::InvalidValue {
                path: diagnostic
                    .path
                    .clone()
                    .unwrap_or_else(|| "<configuration>".into()),
                message: diagnostic.message.clone(),
            }),
            DiagnosticCode::DeprecatedSetting => None,
        }
    }
}

/// Complete typed-facade resolution result.
///
/// Source acquisition, schema construction, and operational resolver failures
/// remain `Err(ConfigError)`. Definition and typed-validation failures are
/// retained here so callers can present every actionable diagnostic at once.
pub struct ConfigResolutionReport<T> {
    pub(crate) loaded: Option<LoadedConfig<T>>,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) validation: Vec<ValidationIssue>,
    pub(crate) information: Vec<ResolutionInformation>,
    pub(crate) failure: Option<ConfigError>,
}

impl<T> ConfigResolutionReport<T> {
    /// Returns true when a validated typed configuration is available.
    pub fn is_success(&self) -> bool {
        self.loaded.is_some()
    }

    /// Borrows the successfully loaded typed configuration, when available.
    pub fn loaded(&self) -> Option<&LoadedConfig<T>> {
        self.loaded.as_ref()
    }

    /// Returns all kernel definition diagnostics.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Returns every typed validation issue, including warnings and errors.
    pub fn validation(&self) -> &[ValidationIssue] {
        &self.validation
    }

    /// Returns normal resolution outcomes collected before validation.
    pub fn information(&self) -> &[ResolutionInformation] {
        &self.information
    }

    /// Consumes a successful report and returns the loaded configuration.
    pub fn into_loaded(self) -> Option<LoadedConfig<T>> {
        self.loaded
    }

    pub(crate) fn into_result(self) -> Result<LoadedConfig<T>, ConfigError> {
        if let Some(loaded) = self.loaded {
            return Ok(loaded);
        }

        let errors = self
            .validation
            .into_iter()
            .filter(|issue| issue.severity == Severity::Error)
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            return Err(ConfigError::Validation(errors));
        }

        Err(self.failure.unwrap_or_else(|| {
            ConfigError::Resolve(ResolveError::InvalidSchema(
                "typed resolution failed without a blocking diagnostic".into(),
            ))
        }))
    }
}
