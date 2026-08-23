//! Schema registration, metadata, validation, and default dependency ordering.

use super::rules::{Condition, DefaultRule};
use super::setting::{SettingSpec, ValidationRule};
use super::validation::{
    prefix_path, valid_definition_path, validate_condition, validate_constraint_definitions,
    validate_definition_value, validate_removal_policy, validate_representation_type,
    validate_requirement, validate_shape_policy, validate_value_type_shape,
};
use crate::ResolveError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Logical identity of a component or application definition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DefinitionIdentity {
    /// Stable logical name.
    pub name: String,
    /// Definition version.  This is intentionally opaque to the kernel.
    pub version: String,
}

impl DefinitionIdentity {
    /// Creates one definition identity.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }
}

/// Metadata for one component composed into an application definition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComponentMetadata {
    /// Application-local component key.
    pub key: String,
    /// Root path owned by the component.
    pub path: String,
    /// Component definition identity.
    pub identity: DefinitionIdentity,
    /// Optional condition that determines enabled versus disabled state.
    pub enabled_when: Option<Condition>,
}

/// Definition metadata embedded in a compiled application schema.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DefinitionMetadata {
    /// Application definition identity.
    pub application: DefinitionIdentity,
    /// Composed components keyed by application-local identity.
    pub components: BTreeMap<String, ComponentMetadata>,
}

/// Complete resolver schema.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Schema {
    pub(crate) settings: BTreeMap<String, SettingSpec>,
    pub(crate) rules: Vec<ValidationRule>,
    pub(crate) metadata: Option<DefinitionMetadata>,
}

impl Schema {
    /// Creates an empty schema.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one setting definition.
    pub fn insert(&mut self, setting: SettingSpec) -> Result<(), ResolveError> {
        if !valid_definition_path(&setting.path) {
            return Err(ResolveError::InvalidSchema(format!(
                "setting path `{}` must contain non-empty dot-separated segments",
                setting.path
            )));
        }
        if let Some(existing) = self
            .settings
            .keys()
            .find(|existing| paths_overlap_by_ancestry(existing, &setting.path))
        {
            let (first, second) = if existing < &setting.path {
                (existing.as_str(), setting.path.as_str())
            } else {
                (setting.path.as_str(), existing.as_str())
            };
            return Err(ResolveError::InvalidSchema(format!(
                "setting paths `{first}` and `{second}` overlap by ancestry"
            )));
        }
        match self.settings.entry(setting.path.clone()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(setting);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(entry) => Err(
                ResolveError::InvalidSchema(format!("duplicate setting `{}`", entry.key())),
            ),
        }
    }

    /// Replaces an existing setting definition without disturbing rules or metadata.
    ///
    /// This is intended for facade-level definition customization such as a
    /// declarative lookup default. The replacement must target an already
    /// declared path; adding new settings should use [`Self::insert`].
    pub fn replace(&mut self, setting: SettingSpec) -> Result<(), ResolveError> {
        if !self.settings.contains_key(&setting.path) {
            return Err(ResolveError::InvalidSchema(format!(
                "cannot replace unknown setting `{}`",
                setting.path
            )));
        }
        self.settings.insert(setting.path.clone(), setting);
        Ok(())
    }

    /// Adds a declarative validation rule.
    pub fn add_rule(&mut self, rule: ValidationRule) {
        self.rules.push(rule);
    }

    /// Returns one setting definition by path.
    pub fn get(&self, path: &str) -> Option<&SettingSpec> {
        self.settings.get(path)
    }

    /// Iterates setting definitions in deterministic path order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &SettingSpec)> {
        self.settings
            .iter()
            .map(|(path, setting)| (path.as_str(), setting))
    }

    pub(crate) fn is_sensitive_path(&self, path: &str) -> bool {
        self.iter().any(|(setting_path, setting)| {
            setting.is_sensitive()
                && (path == setting_path
                    || path
                        .strip_prefix(setting_path)
                        .is_some_and(|suffix| suffix.starts_with('.')))
        })
    }

    /// Returns declarative validation rules.
    pub fn rules(&self) -> &[ValidationRule] {
        &self.rules
    }

    /// Returns application/component identity metadata when this schema was
    /// compiled from an [`crate::ApplicationDefinition`].
    pub fn metadata(&self) -> Option<&DefinitionMetadata> {
        self.metadata.as_ref()
    }

    /// Validates the definition before value resolution begins.
    pub fn validate(&self) -> Result<(), ResolveError> {
        self.validate_structure()?;
        self.validate_settings()?;
        self.validate_rules()?;
        self.validate_metadata()?;
        self.default_order()?;
        Ok(())
    }

    fn validate_settings(&self) -> Result<(), ResolveError> {
        for setting in self.settings.values() {
            self.validate_setting(setting)?;
        }
        Ok(())
    }

    fn validate_setting(&self, setting: &SettingSpec) -> Result<(), ResolveError> {
        validate_shape_policy(setting)?;
        validate_value_type_shape(setting)?;
        validate_representation_type(setting)?;
        validate_constraint_definitions(setting)?;
        validate_removal_policy(setting)?;
        validate_requirement(self, &setting.requirement, &setting.path)?;
        self.validate_default(setting)?;
        self.validate_deprecation(setting)
    }

    fn validate_default(&self, setting: &SettingSpec) -> Result<(), ResolveError> {
        match &setting.default {
            Some(DefaultRule::Fixed(value)) => {
                validate_definition_value(setting, value).map_err(|message| {
                    ResolveError::InvalidSchema(format!(
                        "default for `{}` is invalid: {message}",
                        setting.path
                    ))
                })?;
            }
            Some(DefaultRule::Lookup { source, cases }) => {
                if !self.settings.contains_key(source) {
                    return Err(ResolveError::InvalidSchema(format!(
                        "derived default `{}` references unknown `{source}`",
                        setting.path
                    )));
                }
                for value in cases.values() {
                    validate_definition_value(setting, value).map_err(|message| {
                        ResolveError::InvalidSchema(format!(
                            "derived-default case for `{}` is invalid: {message}",
                            setting.path
                        ))
                    })?;
                }
            }
            None => {}
        }
        Ok(())
    }

    fn validate_deprecation(&self, setting: &SettingSpec) -> Result<(), ResolveError> {
        let Some(deprecation) = &setting.deprecation else {
            return Ok(());
        };
        if deprecation.message.trim().is_empty() {
            return Err(ResolveError::InvalidSchema(format!(
                "deprecated setting `{}` must include a non-empty message",
                setting.path
            )));
        }
        if let Some(replacement) = &deprecation.replacement {
            if replacement == &setting.path {
                return Err(ResolveError::InvalidSchema(format!(
                    "deprecated setting `{}` cannot replace itself",
                    setting.path
                )));
            }
            if !self.settings.contains_key(replacement) {
                return Err(ResolveError::InvalidSchema(format!(
                    "deprecated setting `{}` references unknown replacement `{replacement}`",
                    setting.path
                )));
            }
        }
        Ok(())
    }

    fn validate_rules(&self) -> Result<(), ResolveError> {
        for rule in &self.rules {
            if rule.message.trim().is_empty() {
                return Err(ResolveError::InvalidSchema(
                    "validation rule messages must be non-empty".into(),
                ));
            }
            if rule.subjects.is_empty() {
                return Err(ResolveError::InvalidSchema(
                    "validation rules must identify at least one subject path".into(),
                ));
            }
            validate_condition(self, &rule.condition, "validation rule")?;
            for subject in &rule.subjects {
                if !self.settings.contains_key(subject) {
                    return Err(ResolveError::InvalidSchema(format!(
                        "validation rule references unknown subject `{subject}`"
                    )));
                }
            }
        }
        Ok(())
    }

    fn validate_metadata(&self) -> Result<(), ResolveError> {
        let Some(metadata) = &self.metadata else {
            return Ok(());
        };
        if metadata.application.name.is_empty() || metadata.application.version.is_empty() {
            return Err(ResolveError::InvalidSchema(
                "application definition identity must have non-empty name and version".into(),
            ));
        }
        for component in metadata.components.values() {
            if component.key.is_empty()
                || component.path.is_empty()
                || component.identity.name.is_empty()
                || component.identity.version.is_empty()
            {
                return Err(ResolveError::InvalidSchema(
                    "component metadata must have non-empty key, path, name, and version".into(),
                ));
            }
            if let Some(condition) = &component.enabled_when {
                validate_condition(self, condition, "component enablement")?;
            }
        }
        Ok(())
    }

    /// Returns a copy of this schema rooted beneath `prefix`.
    pub fn prefixed(&self, prefix: &str) -> Result<Self, ResolveError> {
        if !valid_definition_path(prefix) {
            return Err(ResolveError::InvalidSchema(format!(
                "schema prefix `{prefix}` must contain non-empty dot-separated segments"
            )));
        }
        let mut settings = BTreeMap::new();
        for setting in self.settings.values() {
            let mut setting = setting.clone();
            setting.path = prefix_path(prefix, &setting.path);
            setting.requirement = setting.requirement.prefixed(prefix);
            setting.default = setting.default.as_ref().map(|rule| rule.prefixed(prefix));
            if let Some(deprecation) = &mut setting.deprecation
                && let Some(replacement) = &deprecation.replacement
            {
                deprecation.replacement = Some(prefix_path(prefix, replacement));
            }
            settings.insert(setting.path.clone(), setting);
        }

        let prefixed = Self {
            settings,
            rules: self
                .rules
                .iter()
                .map(|rule| rule.prefixed(prefix))
                .collect(),
            metadata: None,
        };
        prefixed.validate()?;
        Ok(prefixed)
    }

    fn validate_structure(&self) -> Result<(), ResolveError> {
        let mut previous_path: Option<&str> = None;
        for (key, setting) in &self.settings {
            if key != &setting.path {
                return Err(ResolveError::InvalidSchema(format!(
                    "schema key `{key}` does not match setting path `{}`",
                    setting.path
                )));
            }
            if !valid_definition_path(&setting.path) {
                return Err(ResolveError::InvalidSchema(format!(
                    "setting path `{}` must contain non-empty dot-separated segments",
                    setting.path
                )));
            }
            if let Some(previous) = previous_path
                && paths_overlap_by_ancestry(previous, &setting.path)
            {
                return Err(ResolveError::InvalidSchema(format!(
                    "setting paths `{previous}` and `{}` overlap by ancestry",
                    setting.path
                )));
            }
            previous_path = Some(&setting.path);
        }

        if let Some(metadata) = &self.metadata {
            let mut component_paths: Vec<&str> = Vec::new();
            for (key, component) in &metadata.components {
                if key != &component.key {
                    return Err(ResolveError::InvalidSchema(format!(
                        "component metadata key `{key}` does not match component key `{}`",
                        component.key
                    )));
                }
                if !valid_definition_path(&component.path) {
                    return Err(ResolveError::InvalidSchema(format!(
                        "component path `{}` must contain non-empty dot-separated segments",
                        component.path
                    )));
                }
                if let Some(other) = component_paths.iter().find(|other| {
                    **other == component.path || paths_overlap_by_ancestry(other, &component.path)
                }) {
                    return Err(ResolveError::InvalidSchema(format!(
                        "component path `{}` overlaps component path `{other}`",
                        component.path
                    )));
                }
                component_paths.push(component.path.as_str());
            }
        }
        Ok(())
    }

    pub(crate) fn default_order(&self) -> Result<Vec<String>, ResolveError> {
        let mut remaining: BTreeSet<String> = self.settings.keys().cloned().collect();
        let mut resolved = BTreeSet::new();
        let mut order = Vec::new();

        while !remaining.is_empty() {
            let ready: Vec<String> = remaining
                .iter()
                .filter(|path| {
                    match self
                        .settings
                        .get(path.as_str())
                        .and_then(|setting| setting.default.as_ref())
                    {
                        Some(DefaultRule::Lookup { source, .. }) => resolved.contains(source),
                        _ => true,
                    }
                })
                .cloned()
                .collect();

            if ready.is_empty() {
                return Err(ResolveError::InvalidSchema(
                    "derived-default dependency cycle".into(),
                ));
            }

            for path in ready {
                remaining.remove(&path);
                resolved.insert(path.clone());
                order.push(path);
            }
        }

        Ok(order)
    }

    pub(super) fn merge_from(&mut self, other: Schema) -> Result<(), ResolveError> {
        for setting in other.settings.into_values() {
            self.insert(setting)?;
        }
        self.rules.extend(other.rules);
        Ok(())
    }
}

fn paths_overlap_by_ancestry(left: &str, right: &str) -> bool {
    left != right && (is_path_ancestor(left, right) || is_path_ancestor(right, left))
}

fn is_path_ancestor(ancestor: &str, descendant: &str) -> bool {
    descendant
        .strip_prefix(ancestor)
        .is_some_and(|remainder| remainder.starts_with('.'))
}
