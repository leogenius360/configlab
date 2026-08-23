//! Resolved typed configuration plus explainability metadata.

use crate::source::strip_internal_markers;
use crate::{
    ComponentPresence, Context, Diagnostic, EffectiveConfiguration, ProvenanceEntry,
    ResolutionInformation, Schema, ValidationIssue,
};
use serde_json::Value;

/// Typed resolved configuration together with redaction-safe explainability metadata.
pub struct LoadedConfig<T> {
    pub(crate) value: T,
    pub(crate) effective: EffectiveConfiguration,
    pub(crate) schema: Schema,
    pub(crate) warnings: Vec<ValidationIssue>,
    pub(crate) optional_groups: Vec<String>,
}

impl<T> LoadedConfig<T> {
    /// Borrows the typed application configuration.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Returns non-blocking application-level validation warnings.
    pub fn warnings(&self) -> &[ValidationIssue] {
        &self.warnings
    }

    /// Returns normal kernel resolution outcomes such as defaults and overrides.
    pub fn information(&self) -> &[ResolutionInformation] {
        &self.effective.information
    }

    /// Returns non-blocking diagnostics produced by the resolver kernel.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.effective.diagnostics
    }

    /// Returns whether a composed component is absent, disabled, or enabled.
    pub fn component_presence(&self, key: &str) -> ComponentPresence {
        self.effective.component_presence(key)
    }

    /// Consumes the wrapper and returns only the typed configuration.
    pub fn into_inner(self) -> T {
        self.value
    }

    /// Returns redaction-safe provenance for one setting or accumulated item path.
    pub fn explain(&self, path: &str) -> Vec<ProvenanceEntry> {
        let sensitive = self.schema.is_sensitive_path(path);
        self.effective
            .explain(path)
            .iter()
            .cloned()
            .map(|mut entry| {
                if sensitive && entry.value.is_some() {
                    entry.value = Some(Value::String("[REDACTED]".into()));
                }
                entry
            })
            .collect()
    }

    /// Returns the effective configuration with sensitive values and internal markers removed.
    pub fn redacted_value(&self) -> Value {
        let mut value = self.effective.redacted(&self.schema);
        strip_internal_markers(&mut value, &self.optional_groups);
        value
    }

    /// Returns the stable selector context used during resolution.
    pub fn context(&self) -> &Context {
        &self.effective.context
    }

    /// Returns the exact validated kernel schema generated for this configuration type.
    ///
    /// Optional nested sections use internal `__config_present` marker settings
    /// to gate child defaults and requirements. Those markers are removed from
    /// public resolution output but intentionally remain visible in this schema
    /// because they are part of the resolver definition graph.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }
}

impl<T> std::ops::Deref for LoadedConfig<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}
