//! Component definitions and application composition.

use super::rules::Condition;
use super::schema::{ComponentMetadata, DefinitionIdentity, DefinitionMetadata, Schema};
use super::setting::{ValidationLevel, ValidationRule};
use super::validation::{valid_definition_path, validate_condition};
use crate::ResolveError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Independently configurable software capability.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComponentDefinition {
    /// Stable component identity.
    pub identity: DefinitionIdentity,
    /// Component-owned schema using paths relative to the component root.
    pub schema: Schema,
    /// Optional relative enablement condition.  No condition means always enabled.
    pub enabled_when: Option<Condition>,
}

impl ComponentDefinition {
    /// Creates a component definition.
    pub fn new(name: impl Into<String>, version: impl Into<String>, schema: Schema) -> Self {
        Self {
            identity: DefinitionIdentity::new(name, version),
            schema,
            enabled_when: None,
        }
    }

    /// Validates this component definition independently of an application.
    ///
    /// Component schemas may own section- and component-level rules, but they
    /// cannot contain compiled application metadata or application-level rules.
    pub fn validate(&self) -> Result<(), ResolveError> {
        if self.identity.name.trim().is_empty() || self.identity.version.trim().is_empty() {
            return Err(ResolveError::InvalidSchema(
                "component definition identity must have non-empty name and version".into(),
            ));
        }
        if self.schema.metadata().is_some() {
            return Err(ResolveError::InvalidSchema(
                "component schema cannot contain compiled application metadata".into(),
            ));
        }
        if self
            .schema
            .rules()
            .iter()
            .any(|rule| rule.level == ValidationLevel::Application)
        {
            return Err(ResolveError::InvalidSchema(
                "component schema cannot own application-level validation rules".into(),
            ));
        }
        self.schema.validate()?;
        if let Some(condition) = &self.enabled_when {
            validate_condition(&self.schema, condition, "component enablement")?;
        }
        Ok(())
    }

    /// Declares the condition under which the component is enabled.
    #[must_use]
    pub fn enabled_when(mut self, condition: Condition) -> Self {
        self.enabled_when = Some(condition);
        self
    }
}

/// One component composition inside an application definition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComponentComposition {
    /// Application-local component key.
    pub key: String,
    /// Root path under which the component owns configuration.
    pub path: String,
    /// Reusable component definition.
    pub definition: ComponentDefinition,
}

/// Application definition that composes independently owned components.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApplicationDefinition {
    /// Stable application definition identity.
    pub identity: DefinitionIdentity,
    /// Application-owned settings not belonging to a child component.
    pub settings: Schema,
    /// Composed component definitions.
    pub components: Vec<ComponentComposition>,
    /// Cross-component application validation rules.
    pub rules: Vec<ValidationRule>,
}

impl ApplicationDefinition {
    /// Creates an empty application definition.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            identity: DefinitionIdentity::new(name, version),
            settings: Schema::new(),
            components: Vec::new(),
            rules: Vec::new(),
        }
    }

    /// Replaces the application-owned root settings.
    #[must_use]
    pub fn with_settings(mut self, settings: Schema) -> Self {
        self.settings = settings;
        self
    }

    /// Composes a component under an application-local key and root path.
    pub fn compose(
        &mut self,
        key: impl Into<String>,
        path: impl Into<String>,
        definition: ComponentDefinition,
    ) -> Result<(), ResolveError> {
        let key = key.into();
        let path = path.into();
        if key.trim().is_empty() || !valid_definition_path(&path) {
            return Err(ResolveError::InvalidSchema(
                "component key must be non-empty and path must contain non-empty dot-separated segments".into(),
            ));
        }
        if self.components.iter().any(|component| component.key == key) {
            return Err(ResolveError::InvalidSchema(format!(
                "duplicate component key `{key}`"
            )));
        }
        if self.components.iter().any(|component| {
            component.path == path
                || component.path.starts_with(&format!("{path}."))
                || path.starts_with(&format!("{}.", component.path))
        }) {
            return Err(ResolveError::InvalidSchema(format!(
                "component path `{path}` overlaps an existing component path"
            )));
        }
        self.components.push(ComponentComposition {
            key,
            path,
            definition,
        });
        Ok(())
    }

    /// Adds an application-level validation rule.
    pub fn add_rule(&mut self, mut rule: ValidationRule) {
        rule.level = ValidationLevel::Application;
        self.rules.push(rule);
    }

    /// Compiles component-local schemas into one resolver schema.
    ///
    /// Compilation revalidates composition invariants even when callers built
    /// the public data structs directly instead of using [`Self::compose`].
    pub fn compile(&self) -> Result<Schema, ResolveError> {
        if self.identity.name.trim().is_empty() || self.identity.version.trim().is_empty() {
            return Err(ResolveError::InvalidSchema(
                "application definition identity must have non-empty name and version".into(),
            ));
        }
        if self.settings.metadata().is_some() {
            return Err(ResolveError::InvalidSchema(
                "application-owned settings cannot contain compiled application metadata".into(),
            ));
        }
        if self
            .rules
            .iter()
            .any(|rule| rule.level != ValidationLevel::Application)
        {
            return Err(ResolveError::InvalidSchema(
                "application-owned cross-component rules must use ValidationLevel::Application"
                    .into(),
            ));
        }

        self.settings.validate()?;
        validate_component_compositions(&self.components)?;

        let mut schema = self.settings.clone();
        let mut metadata = DefinitionMetadata {
            application: self.identity.clone(),
            components: BTreeMap::new(),
        };

        for composition in &self.components {
            if self.settings.iter().any(|(path, _)| {
                path == composition.path.as_str()
                    || path.starts_with(&format!("{}.", composition.path))
                    || composition.path.starts_with(&format!("{path}."))
            }) {
                return Err(ResolveError::InvalidSchema(format!(
                    "application-owned settings overlap component path `{}`",
                    composition.path
                )));
            }

            composition.definition.validate()?;
            let prefixed = composition.definition.schema.prefixed(&composition.path)?;
            schema.merge_from(prefixed)?;

            let enabled_when = composition
                .definition
                .enabled_when
                .as_ref()
                .map(|condition| condition.prefixed(&composition.path));

            metadata.components.insert(
                composition.key.clone(),
                ComponentMetadata {
                    key: composition.key.clone(),
                    path: composition.path.clone(),
                    identity: composition.definition.identity.clone(),
                    enabled_when,
                },
            );
        }

        schema.rules.extend(self.rules.clone());
        schema.metadata = Some(metadata);
        schema.validate()?;
        Ok(schema)
    }
}

fn validate_component_compositions(
    components: &[ComponentComposition],
) -> Result<(), ResolveError> {
    let mut keys = BTreeSet::new();
    for (index, component) in components.iter().enumerate() {
        if component.key.trim().is_empty() || !valid_definition_path(&component.path) {
            return Err(ResolveError::InvalidSchema(
                "component key must be non-empty and path must contain non-empty dot-separated segments".into(),
            ));
        }
        if !keys.insert(component.key.as_str()) {
            return Err(ResolveError::InvalidSchema(format!(
                "duplicate component key `{}`",
                component.key
            )));
        }
        for other in components.iter().take(index) {
            if other.path == component.path
                || other.path.starts_with(&format!("{}.", component.path))
                || component.path.starts_with(&format!("{}.", other.path))
            {
                return Err(ResolveError::InvalidSchema(format!(
                    "component path `{}` overlaps component path `{}`",
                    component.path, other.path
                )));
            }
        }
    }
    Ok(())
}
