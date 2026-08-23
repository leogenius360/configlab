//! Configuration definition model.

mod application;
mod rules;
mod schema;
mod setting;
mod validation;
mod value;

pub use application::{ApplicationDefinition, ComponentComposition, ComponentDefinition};
pub use rules::{Condition, Constraint, DefaultRule, Deprecation, Requirement};
pub use schema::{ComponentMetadata, DefinitionIdentity, DefinitionMetadata, Schema};
pub use setting::{
    Diagnostic, DiagnosticCode, DiagnosticSeverity, SettingSpec, ValidationLevel, ValidationRule,
};
pub use value::{Disclosure, MergePolicy, RemovalPolicy, Representation, ValueShape, ValueType};

pub(crate) use validation::{
    constraint_violation, value_matches_representation, value_matches_shape, value_matches_type,
};
