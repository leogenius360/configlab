//! Declarative conditions, requirements, constraints, defaults, and deprecation metadata.

use super::validation::{compare_paths, prefix_path};
use crate::get_path;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// Declarative predicate over a fully resolved candidate configuration.
///
/// Conditions are intentionally small and non-executable.  They support the
/// conditional-required and validation cases established by the reference
/// applications without becoming a general-purpose scripting language.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Condition {
    /// A path equals a literal value.
    Equals { path: String, value: Value },
    /// A path is present, including explicit `null`.
    Present { path: String },
    /// An array or object contains the supplied value/key.
    Contains { path: String, value: Value },
    /// Numeric value at `left` is strictly less than numeric value at `right`.
    LessThan { left: String, right: String },
    /// Numeric value at `left` is less than or equal to numeric value at `right`.
    LessOrEqual { left: String, right: String },
    /// All child conditions must match.
    All(Vec<Condition>),
    /// At least one child condition must match.
    Any(Vec<Condition>),
    /// Logical negation.
    Not(Box<Condition>),
}

impl Condition {
    /// Convenience condition for a boolean flag being `true`.
    pub fn boolean(path: impl Into<String>) -> Self {
        Self::Equals {
            path: path.into(),
            value: Value::Bool(true),
        }
    }

    /// Evaluates this condition against one complete resolved candidate.
    pub fn evaluate(&self, values: &Value) -> bool {
        match self {
            Self::Equals { path, value } => get_path(values, path) == Some(value),
            Self::Present { path } => get_path(values, path).is_some(),
            Self::Contains { path, value } => match get_path(values, path) {
                Some(Value::Array(items)) => items.iter().any(|item| item == value),
                Some(Value::Object(map)) => value.as_str().is_some_and(|key| map.contains_key(key)),
                _ => false,
            },
            Self::LessThan { left, right } => compare_paths(values, left, right)
                .is_some_and(|ordering| ordering == Ordering::Less),
            Self::LessOrEqual { left, right } => compare_paths(values, left, right)
                .is_some_and(|ordering| ordering != Ordering::Greater),
            Self::All(conditions) => conditions
                .iter()
                .all(|condition| condition.evaluate(values)),
            Self::Any(conditions) => conditions
                .iter()
                .any(|condition| condition.evaluate(values)),
            Self::Not(condition) => !condition.evaluate(values),
        }
    }

    /// Returns all configuration paths referenced by this condition.
    pub fn referenced_paths(&self) -> Vec<&str> {
        let mut paths = Vec::new();
        self.collect_paths(&mut paths);
        paths
    }

    /// Returns a condition with every referenced path rooted beneath `prefix`.
    #[must_use]
    pub fn prefixed(&self, prefix: &str) -> Self {
        match self {
            Self::Equals { path, value } => Self::Equals {
                path: prefix_path(prefix, path),
                value: value.clone(),
            },
            Self::Present { path } => Self::Present {
                path: prefix_path(prefix, path),
            },
            Self::Contains { path, value } => Self::Contains {
                path: prefix_path(prefix, path),
                value: value.clone(),
            },
            Self::LessThan { left, right } => Self::LessThan {
                left: prefix_path(prefix, left),
                right: prefix_path(prefix, right),
            },
            Self::LessOrEqual { left, right } => Self::LessOrEqual {
                left: prefix_path(prefix, left),
                right: prefix_path(prefix, right),
            },
            Self::All(conditions) => Self::All(
                conditions
                    .iter()
                    .map(|condition| condition.prefixed(prefix))
                    .collect(),
            ),
            Self::Any(conditions) => Self::Any(
                conditions
                    .iter()
                    .map(|condition| condition.prefixed(prefix))
                    .collect(),
            ),
            Self::Not(condition) => Self::Not(Box::new(condition.prefixed(prefix))),
        }
    }

    fn collect_paths<'a>(&'a self, output: &mut Vec<&'a str>) {
        match self {
            Self::Equals { path, .. } | Self::Present { path } | Self::Contains { path, .. } => {
                output.push(path)
            }
            Self::LessThan { left, right } | Self::LessOrEqual { left, right } => {
                output.push(left);
                output.push(right);
            }
            Self::All(conditions) | Self::Any(conditions) => {
                for condition in conditions {
                    condition.collect_paths(output);
                }
            }
            Self::Not(condition) => condition.collect_paths(output),
        }
    }
}

/// Whether a setting must exist in the effective configuration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Requirement {
    /// The setting may be absent.
    Optional,
    /// The setting must always be present.
    Required,
    /// The setting is required when the declarative condition evaluates true.
    RequiredWhen(Condition),
}

impl Requirement {
    /// Returns whether this requirement is active for `values`.
    pub fn is_required(&self, values: &Value) -> bool {
        match self {
            Self::Optional => false,
            Self::Required => true,
            Self::RequiredWhen(condition) => condition.evaluate(values),
        }
    }

    /// Returns a requirement whose condition paths are rooted beneath `prefix`.
    #[must_use]
    pub fn prefixed(&self, prefix: &str) -> Self {
        match self {
            Self::Optional => Self::Optional,
            Self::Required => Self::Required,
            Self::RequiredWhen(condition) => Self::RequiredWhen(condition.prefixed(prefix)),
        }
    }

    /// Gates an existing requirement behind another condition.
    ///
    /// This is primarily used by optional nested sections: a child that is
    /// normally required becomes required only when its parent section is
    /// present, while an already-conditional child requires both conditions.
    #[must_use]
    pub fn gated_by(self, gate: Condition) -> Self {
        match self {
            Self::Optional => Self::Optional,
            Self::Required => Self::RequiredWhen(gate),
            Self::RequiredWhen(condition) => {
                Self::RequiredWhen(Condition::All(vec![gate, condition]))
            }
        }
    }
}

/// Built-in value constraints applied after type validation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Constraint {
    /// Minimum signed integer value.
    IntegerMin(i128),
    /// Maximum signed integer value.
    IntegerMax(i128),
    /// Minimum unsigned integer value.
    UnsignedMin(u128),
    /// Maximum unsigned integer value.
    UnsignedMax(u128),
    /// Minimum floating-point value.
    FloatMin(f64),
    /// Maximum floating-point value.
    FloatMax(f64),
    /// String must not be empty.
    NonEmpty,
    /// String must match one of the listed values.
    OneOf(Vec<String>),
}

/// Definition-owned default rule.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DefaultRule {
    /// Fixed literal default.
    Fixed(Value),
    /// Declarative lookup default keyed by an already-resolved setting.
    Lookup {
        /// Source setting path.
        source: String,
        /// Stringified source-value to default-value mapping.
        cases: BTreeMap<String, Value>,
    },
}

impl DefaultRule {
    pub(super) fn prefixed(&self, prefix: &str) -> Self {
        match self {
            Self::Fixed(value) => Self::Fixed(value.clone()),
            Self::Lookup { source, cases } => Self::Lookup {
                source: prefix_path(prefix, source),
                cases: cases.clone(),
            },
        }
    }
}

/// Optional deprecation metadata for a setting.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Deprecation {
    /// Human-readable migration guidance.
    pub message: String,
    /// Optional replacement setting path.
    pub replacement: Option<String>,
}
