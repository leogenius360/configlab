//! Explainability metadata and effective configuration.

use super::contribution::Contribution;
use crate::definition::{MergePolicy, SettingSpec, ValueShape};

use super::{Context, InputTarget, Origin, get_path, set_existing_path};
use crate::definition::{DefinitionIdentity, Diagnostic, Schema};
use crate::report::ComponentPresence;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::{self, Debug, Formatter};

/// Appends one dynamic object/map key to a provenance path without allowing
/// dots or escape characters in the key to collide with structural segments.
pub fn provenance_child_path(parent: &str, key: &str) -> String {
    let escaped = key.replace('\\', "\\\\").replace('.', "\\.");
    format!("{parent}.{escaped}")
}

/// How a provenance entry affected the candidate value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ProvenanceAction {
    /// Input supplied or combined a value.
    Set,
    /// Input removed the complete setting.
    Unset,
    /// Input retained the setting as an empty collection.
    Clear,
    /// Input removed one named-map entry.
    RemoveMapEntry,
    /// Input removed one matching ordered-list item.
    RemoveListItem,
    /// Definition default supplied the value.
    Default,
    /// Applicable same-layer candidate lost to a more-specific candidate.
    Shadowed,
}

/// One explainability record for a setting or accumulated item.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProvenanceEntry {
    /// Explicit precedence layer that supplied the contribution.
    pub layer: String,
    /// Number of selector attributes matched by the contributing input.
    pub specificity: usize,
    /// Stable logical-input identity.
    pub input_id: String,
    /// Scope targeted by the contributing logical input.
    pub target: InputTarget,
    /// Physical/logical source identity for the contribution.
    pub origin: Origin,
    /// How this contribution affected the candidate.
    pub action: ProvenanceAction,
    /// Redacted when the definition marks the setting sensitive.
    pub value: Option<Value>,
    /// Final ordered-list item index when provenance belongs to one item.
    pub item: Option<usize>,
}

/// Normal resolution outcome that is useful to explain but is not a problem.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ResolutionInformation {
    /// A logical input did not match the stable context and was ignored.
    InputIgnored {
        /// Stable logical-input identity.
        input_id: String,
        /// Input target retained for explainability.
        target: InputTarget,
        /// Physical/logical source identity.
        origin: Origin,
        /// Human-readable non-match reason.
        reason: String,
    },
    /// A fixed definition default supplied the candidate value.
    DefaultUsed {
        /// Setting path populated by the default.
        path: String,
    },
    /// A lookup-table default supplied the candidate value.
    DerivedDefaultUsed {
        /// Setting path populated by the derived default.
        path: String,
        /// Already-resolved setting that selected the lookup case.
        source: String,
    },
    /// A lower-layer replace value was superseded by a higher layer.
    ValueOverridden {
        /// Setting whose effective value changed.
        path: String,
        /// Layer that previously supplied the effective value.
        previous_layer: String,
        /// Higher layer that replaced it.
        winning_layer: String,
    },
    /// An optional setting was absent after resolution.
    OptionalOmitted {
        /// Omitted setting path.
        path: String,
    },
    /// A removal targeted no existing map entry or list item.
    RemovalNoop {
        /// Setting path targeted by the no-op removal.
        path: String,
        /// Removal operation kind.
        operation: RemovalKind,
    },
}

/// Structured removal operation kind used by resolution information.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemovalKind {
    /// Removal of one named-map entry.
    MapEntry,
    /// Removal of one matching ordered-list item.
    ListItem,
}

/// Fully resolved, definition-valid configuration.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectiveConfiguration {
    /// Complete resolved values.
    pub values: Value,
    /// Per-setting and per-item provenance.
    pub provenance: BTreeMap<String, Vec<ProvenanceEntry>>,
    /// Stable selector context used for this resolution.
    pub context: Context,
    /// Normal, non-problem resolution information.
    pub information: Vec<ResolutionInformation>,
    /// Non-blocking kernel diagnostics. Successful configurations never carry
    /// error-severity diagnostics.
    pub diagnostics: Vec<Diagnostic>,
    /// Application definition identity when resolution used a compiled
    /// application definition.
    pub definition: Option<DefinitionIdentity>,
    /// Enabled/disabled state for every composed component.
    pub components: BTreeMap<String, ComponentPresence>,
}

impl Debug for EffectiveConfiguration {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EffectiveConfiguration")
            .field("values", &"[RAW CONFIGURATION OMITTED]")
            .field("provenance_paths", &self.provenance.len())
            .field("context", &self.context)
            .field("information", &self.information)
            .field("diagnostics", &self.diagnostics)
            .field("definition", &self.definition)
            .field("components", &self.components)
            .finish()
    }
}

impl EffectiveConfiguration {
    /// Returns one resolved setting by path.
    pub fn get(&self, path: &str) -> Option<&Value> {
        get_path(&self.values, path)
    }

    /// Returns provenance for one setting path.
    pub fn explain(&self, path: &str) -> &[ProvenanceEntry] {
        self.provenance.get(path).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Returns the component's presence/enablement state.
    pub fn component_presence(&self, key: &str) -> ComponentPresence {
        self.components
            .get(key)
            .copied()
            .unwrap_or(ComponentPresence::Absent)
    }

    /// Produces a representation safe for ordinary logs and support output.
    pub fn redacted(&self, schema: &Schema) -> Value {
        let mut value = self.values.clone();
        for (_, setting) in schema.iter().filter(|(_, setting)| setting.is_sensitive()) {
            set_existing_path(
                &mut value,
                &setting.path,
                Value::String("[REDACTED]".into()),
            );
        }
        value
    }
}

// Internal provenance construction helpers used by the resolver merge engine.
pub(super) fn record_default_provenance(
    path: &str,
    setting: &SettingSpec,
    value: &Value,
    provenance: &mut BTreeMap<String, Vec<ProvenanceEntry>>,
) {
    if !setting.is_sensitive()
        && setting.shape == ValueShape::OrderedList
        && matches!(setting.merge, MergePolicy::Append | MergePolicy::Prepend)
    {
        if let Some(items) = value.as_array() {
            let entries = provenance.entry(path.to_string()).or_default();
            for (index, item) in items.iter().enumerate() {
                entries.push(ProvenanceEntry {
                    layer: "defaults".into(),
                    specificity: 0,
                    input_id: "definition-default".into(),
                    target: InputTarget::Root,
                    origin: Origin::new("definition"),
                    action: ProvenanceAction::Default,
                    value: Some(provenance_value(setting, item)),
                    item: Some(index),
                });
            }
        }
        return;
    }

    provenance
        .entry(path.to_string())
        .or_default()
        .push(ProvenanceEntry {
            layer: "defaults".into(),
            specificity: 0,
            input_id: "definition-default".into(),
            target: InputTarget::Root,
            origin: Origin::new("definition"),
            action: ProvenanceAction::Default,
            value: Some(provenance_value(setting, value)),
            item: None,
        });
}
pub(super) fn provenance_entry(
    setting: &SettingSpec,
    contribution: &Contribution<'_>,
    action: ProvenanceAction,
    value: Option<&Value>,
    item: Option<usize>,
) -> ProvenanceEntry {
    ProvenanceEntry {
        layer: contribution.input.layer.clone(),
        specificity: contribution.specificity,
        input_id: contribution.input.id.clone(),
        target: contribution.input.target.clone(),
        origin: contribution.input.origin.clone(),
        action,
        value: value.map(|value| provenance_value(setting, value)),
        item,
    }
}

pub(super) fn current_effective_layer(entries: Option<&[ProvenanceEntry]>) -> Option<&str> {
    for entry in entries?.iter().rev() {
        match entry.action {
            ProvenanceAction::Set | ProvenanceAction::Default => {
                return Some(entry.layer.as_str());
            }
            ProvenanceAction::Unset | ProvenanceAction::Clear => return None,
            ProvenanceAction::Shadowed
            | ProvenanceAction::RemoveMapEntry
            | ProvenanceAction::RemoveListItem => {}
        }
    }
    None
}

pub(super) fn provenance_value(setting: &SettingSpec, value: &Value) -> Value {
    if setting.is_sensitive() {
        Value::String("[REDACTED]".into())
    } else {
        value.clone()
    }
}

/// Records provenance at the direct entry boundary of a named map.
///
/// `CombineByKey` treats each direct map entry as the atomic merge unit. Nested
/// objects inside an entry are therefore intentionally not assigned leaf-level
/// provenance, which would incorrectly imply recursive deep-merge semantics.
pub(super) fn record_named_map_provenance(
    path: &str,
    setting: &SettingSpec,
    value: &Value,
    contribution: &Contribution<'_>,
    provenance: &mut BTreeMap<String, Vec<ProvenanceEntry>>,
) {
    if setting.is_sensitive() {
        return;
    }
    if let Value::Object(map) = value {
        for (key, entry) in map {
            provenance
                .entry(provenance_child_path(path, key))
                .or_default()
                .push(provenance_entry(
                    setting,
                    contribution,
                    ProvenanceAction::Set,
                    Some(entry),
                    None,
                ));
        }
    }
}

pub(super) fn record_nested_provenance(
    path: &str,
    setting: &SettingSpec,
    value: &Value,
    contribution: &Contribution<'_>,
    provenance: &mut BTreeMap<String, Vec<ProvenanceEntry>>,
) {
    if setting.is_sensitive() {
        return;
    }
    if let Value::Object(map) = value {
        for (key, child) in map {
            let child_path = provenance_child_path(path, key);
            if child.is_object() {
                record_nested_provenance(&child_path, setting, child, contribution, provenance);
            } else {
                provenance
                    .entry(child_path)
                    .or_default()
                    .push(provenance_entry(
                        setting,
                        contribution,
                        ProvenanceAction::Set,
                        Some(child),
                        None,
                    ));
            }
        }
    }
}
