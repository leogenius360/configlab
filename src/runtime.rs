//! Refreshable typed configuration.
//!
//! [`LiveConfig`] retains source descriptors, rereads physical sources on each
//! refresh, and replaces its current value only after complete resolution and
//! validation succeed. Remote transports remain outside this crate and can
//! submit complete source-neutral inputs through [`LiveConfig::reconcile`].

use crate::source::is_internal_path;
use crate::{Config, ConfigError, ConfigPlan, LoadedConfig, LogicalInput, ProvenanceEntry};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Kind of one setting-level refresh change.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    /// The setting was absent and is now present.
    Added,
    /// The setting was present and is now absent.
    Removed,
    /// The effective setting value changed.
    Modified,
    /// The value is equal but its public provenance changed.
    ProvenanceChanged,
}

/// One redaction-aware setting-level refresh change.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConfigurationChange {
    path: String,
    component: Option<String>,
    kind: ChangeKind,
    sensitive: bool,
    previous: Option<Value>,
    current: Option<Value>,
    previous_provenance: Vec<ProvenanceEntry>,
    current_provenance: Vec<ProvenanceEntry>,
}

impl ConfigurationChange {
    /// Returns the public setting path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the composed component that owns the setting, when applicable.
    pub fn component(&self) -> Option<&str> {
        self.component.as_deref()
    }

    /// Returns how the setting changed.
    pub fn kind(&self) -> ChangeKind {
        self.kind
    }

    /// Returns whether values and provenance values are intentionally withheld.
    pub fn is_sensitive(&self) -> bool {
        self.sensitive
    }

    /// Returns the previous public value when disclosure is allowed.
    pub fn previous(&self) -> Option<&Value> {
        self.previous.as_ref()
    }

    /// Returns the current public value when disclosure is allowed.
    pub fn current(&self) -> Option<&Value> {
        self.current.as_ref()
    }

    /// Returns redaction-aware provenance before the refresh.
    pub fn previous_provenance(&self) -> &[ProvenanceEntry] {
        &self.previous_provenance
    }

    /// Returns redaction-aware provenance after the refresh.
    pub fn current_provenance(&self) -> &[ProvenanceEntry] {
        &self.current_provenance
    }
}

/// Ordered redaction-aware changes produced by one successful refresh.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ChangeSet {
    changes: Vec<ConfigurationChange>,
}

impl ChangeSet {
    /// Returns all changed schema settings in lexical path order.
    pub fn changes(&self) -> &[ConfigurationChange] {
        &self.changes
    }

    /// Returns whether no effective value or public provenance changed.
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    /// Returns the number of changed settings.
    pub fn len(&self) -> usize {
        self.changes.len()
    }

    /// Iterates changes owned by one composed component.
    pub fn for_component<'a>(
        &'a self,
        component: &'a str,
    ) -> impl Iterator<Item = &'a ConfigurationChange> {
        self.changes
            .iter()
            .filter(move |change| change.component() == Some(component))
    }
}

/// Result of one successful refresh or reconciliation.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RefreshReport {
    changes: ChangeSet,
}

impl RefreshReport {
    /// Returns setting-level value and provenance changes.
    pub fn changes(&self) -> &ChangeSet {
        &self.changes
    }

    /// Returns whether the effective value or provenance changed.
    pub fn changed(&self) -> bool {
        !self.changes.is_empty()
    }
}

/// A typed configuration value that can reread and reconcile its sources.
///
/// A refresh is transactional: ConfigLab acquires every registered source,
/// resolves and validates the complete configuration, computes a redaction-aware
/// change report, and only then replaces the current value. Failure leaves the
/// current value and external logical inputs unchanged.
pub struct LiveConfig<T: Config> {
    plan: ConfigPlan<T>,
    current: LoadedConfig<T>,
    external_inputs: Vec<LogicalInput>,
}

impl<T: Config> LiveConfig<T> {
    /// Resolves the initial value from a reusable plan.
    pub fn new(plan: ConfigPlan<T>) -> Result<Self, ConfigError> {
        let current = plan.resolve()?;
        Ok(Self {
            plan,
            current,
            external_inputs: Vec::new(),
        })
    }

    /// Borrows the current typed configuration.
    pub fn value(&self) -> &T {
        self.current.value()
    }

    /// Borrows the current typed value and its explainability metadata.
    pub fn loaded(&self) -> &LoadedConfig<T> {
        &self.current
    }

    /// Borrows the reusable source and resolution plan.
    pub fn plan(&self) -> &ConfigPlan<T> {
        &self.plan
    }

    /// Returns the number of complete external logical inputs currently reconciled.
    pub fn external_input_count(&self) -> usize {
        self.external_inputs.len()
    }

    /// Rereads all registered physical sources and reconciles retained external inputs.
    pub fn refresh(&mut self) -> Result<RefreshReport, ConfigError> {
        self.replace_with(self.external_inputs.clone())
    }

    /// Replaces the complete external contribution while rereading physical sources.
    ///
    /// Callers acquire remote data through their own transport and pass its
    /// current source-neutral representation here. Inputs are retained for later
    /// [`refresh`](Self::refresh) calls only after this reconciliation succeeds.
    pub fn reconcile<I>(&mut self, inputs: I) -> Result<RefreshReport, ConfigError>
    where
        I: IntoIterator<Item = LogicalInput>,
    {
        self.replace_with(inputs.into_iter().collect())
    }

    /// Alias for [`reconcile`](Self::reconcile) emphasizing one combined refresh.
    pub fn refresh_with<I>(&mut self, inputs: I) -> Result<RefreshReport, ConfigError>
    where
        I: IntoIterator<Item = LogicalInput>,
    {
        self.reconcile(inputs)
    }

    /// Removes all external contributions and rereads registered physical sources.
    pub fn clear_external(&mut self) -> Result<RefreshReport, ConfigError> {
        self.replace_with(Vec::new())
    }

    fn replace_with(
        &mut self,
        external_inputs: Vec<LogicalInput>,
    ) -> Result<RefreshReport, ConfigError> {
        let next = self.plan.resolve_with_inputs(&external_inputs)?;
        let changes = changes_between(&self.current, &next);
        self.current = next;
        self.external_inputs = external_inputs;
        Ok(RefreshReport { changes })
    }
}

impl<T: Config> std::ops::Deref for LiveConfig<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value()
    }
}

fn changes_between<T>(before: &LoadedConfig<T>, after: &LoadedConfig<T>) -> ChangeSet {
    let mut changes = Vec::new();
    for (path, setting) in after.schema.iter() {
        if is_internal_path(path) {
            continue;
        }
        let previous = before.effective.get(path);
        let current = after.effective.get(path);
        let kind = match (previous, current) {
            (None, None) => {
                if setting_provenance_equal(&before.effective, &after.effective, path) {
                    continue;
                }
                ChangeKind::ProvenanceChanged
            }
            (None, Some(_)) => ChangeKind::Added,
            (Some(_), None) => ChangeKind::Removed,
            (Some(previous), Some(current)) if previous != current => ChangeKind::Modified,
            (Some(_), Some(_)) => {
                if setting_provenance_equal(&before.effective, &after.effective, path) {
                    continue;
                }
                ChangeKind::ProvenanceChanged
            }
        };
        let sensitive = setting.is_sensitive();
        changes.push(ConfigurationChange {
            path: path.to_string(),
            component: component_for_path(after.schema.metadata(), path),
            kind,
            sensitive,
            previous: (!sensitive).then(|| previous.cloned()).flatten(),
            current: (!sensitive).then(|| current.cloned()).flatten(),
            previous_provenance: public_provenance(&before.effective, path, sensitive),
            current_provenance: public_provenance(&after.effective, path, sensitive),
        });
    }
    ChangeSet { changes }
}

fn component_for_path(metadata: Option<&crate::DefinitionMetadata>, path: &str) -> Option<String> {
    metadata?.components.values().find_map(|component| {
        (path == component.path || path.starts_with(&format!("{}.", component.path)))
            .then(|| component.key.clone())
    })
}

fn public_provenance(
    effective: &crate::EffectiveConfiguration,
    path: &str,
    sensitive: bool,
) -> Vec<ProvenanceEntry> {
    let child_prefix = format!("{path}.");
    let mut entries = effective
        .provenance
        .range(path.to_string()..)
        .take_while(|(candidate, _)| {
            candidate.as_str() == path || candidate.starts_with(&child_prefix)
        })
        .flat_map(|(_, entries)| entries.iter().cloned())
        .collect::<Vec<_>>();
    if sensitive {
        for entry in &mut entries {
            if entry.value.is_some() {
                entry.value = Some(Value::String("[REDACTED]".into()));
            }
        }
    }
    entries
}

fn setting_provenance_equal(
    before: &crate::EffectiveConfiguration,
    after: &crate::EffectiveConfiguration,
    path: &str,
) -> bool {
    let child_prefix = format!("{path}.");
    let lower_bound = path.to_string();
    let before = before
        .provenance
        .range(lower_bound.clone()..)
        .take_while(|(candidate, _)| {
            candidate.as_str() == path || candidate.starts_with(&child_prefix)
        });
    let after = after
        .provenance
        .range(lower_bound..)
        .take_while(|(candidate, _)| {
            candidate.as_str() == path || candidate.starts_with(&child_prefix)
        });
    before.eq(after)
}
