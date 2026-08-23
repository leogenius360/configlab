//! Setting definitions and structured validation diagnostics.

use super::rules::{Condition, Constraint, DefaultRule, Deprecation, Requirement};
use super::validation::prefix_path;
use super::value::{Disclosure, MergePolicy, RemovalPolicy, Representation, ValueShape, ValueType};
use serde::{Deserialize, Serialize};

/// Complete definition of one effective configuration setting.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SettingSpec {
    /// Stable dot-separated setting path.
    pub path: String,
    /// Structural value shape.
    pub shape: ValueShape,
    /// Scalar/element semantic type.
    pub value_type: ValueType,
    /// Cross-input combination policy.
    pub merge: MergePolicy,
    /// Requiredness rule.
    pub requirement: Requirement,
    /// Whether explicit JSON null is permitted.
    pub nullable: bool,
    /// Whether the value is ordinary data or a reference.
    pub representation: Representation,
    /// Disclosure classification used for diagnostics and provenance.
    pub disclosure: Disclosure,
    /// Definition-approved removal operations.
    pub removal: RemovalPolicy,
    /// Value constraints applied after type validation.
    pub constraints: Vec<Constraint>,
    /// Optional fixed or lookup default.
    pub default: Option<DefaultRule>,
    /// Optional human-readable documentation.
    pub description: Option<String>,
    /// Optional deprecation metadata.
    pub deprecation: Option<Deprecation>,
}

impl SettingSpec {
    /// Creates a setting with conservative defaults.
    ///
    /// `ValueType::Any` is retained as an escape hatch for hand-written
    /// schemas.  Typed facades should set a concrete type whenever possible.
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            shape: ValueShape::Scalar,
            value_type: ValueType::Any,
            merge: MergePolicy::Replace,
            requirement: Requirement::Optional,
            nullable: false,
            representation: Representation::Ordinary,
            disclosure: Disclosure::Public,
            removal: RemovalPolicy::default(),
            constraints: Vec::new(),
            default: None,
            description: None,
            deprecation: None,
        }
    }

    /// Creates a setting with an explicit semantic type.
    pub fn typed(path: impl Into<String>, value_type: ValueType) -> Self {
        let mut setting = Self::new(path);
        setting.value_type = value_type;
        setting
    }

    /// Returns true when diagnostic values for this setting must be redacted.
    pub fn is_sensitive(&self) -> bool {
        self.disclosure == Disclosure::Sensitive
            || self.representation == Representation::SecretReference
    }
}

/// Validation phase associated with a declarative rule.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ValidationLevel {
    /// Relationship within a logical section.
    Section,
    /// Relationship across one component definition.
    Component,
    /// Relationship across composed application components.
    Application,
}

/// Diagnostic severity produced by kernel validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    /// Blocks production of a successful effective configuration.
    Error,
    /// Allows a successful effective configuration while surfacing a problem.
    Warning,
}

/// Stable category of a kernel validation diagnostic.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DiagnosticCode {
    /// Required value is absent.
    MissingRequired,
    /// Value has the wrong semantic type.
    InvalidType,
    /// Value violates a declared constraint.
    ConstraintViolation,
    /// Value violates representation semantics.
    InvalidRepresentation,
    /// Declarative section/component/application rule failed.
    RuleViolation,
    /// Deprecated setting is present.
    DeprecatedSetting,
}

impl DiagnosticCode {
    /// Stable descriptive identifier suitable for serialized support tooling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingRequired => "missing_required",
            Self::InvalidType => "invalid_type",
            Self::ConstraintViolation => "constraint_violation",
            Self::InvalidRepresentation => "invalid_representation",
            Self::RuleViolation => "rule_violation",
            Self::DeprecatedSetting => "deprecated_setting",
        }
    }
}

/// Structured validation diagnostic.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Blocking/error versus non-blocking/warning severity.
    pub severity: DiagnosticSeverity,
    /// Machine-readable diagnostic category.
    pub code: DiagnosticCode,
    /// Optional primary setting path.
    pub path: Option<String>,
    /// All settings participating in the problem.
    pub subjects: Vec<String>,
    /// Human-readable explanation that must not contain sensitive values.
    pub message: String,
}

impl Diagnostic {
    /// Constructs a blocking diagnostic.
    pub fn error(
        code: DiagnosticCode,
        path: Option<String>,
        subjects: Vec<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: DiagnosticSeverity::Error,
            code,
            path,
            subjects,
            message: message.into(),
        }
    }

    /// Constructs a non-blocking diagnostic.
    pub fn warning(
        code: DiagnosticCode,
        path: Option<String>,
        subjects: Vec<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: DiagnosticSeverity::Warning,
            code,
            path,
            subjects,
            message: message.into(),
        }
    }
}

/// Declarative validation rule evaluated against the complete candidate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ValidationRule {
    /// Validation phase for diagnostics and documentation.
    pub level: ValidationLevel,
    /// Predicate that must evaluate true.
    pub condition: Condition,
    /// Diagnostic severity when the predicate fails.
    pub severity: DiagnosticSeverity,
    /// Human-readable failure explanation.
    pub message: String,
    /// Paths involved in the rule.
    pub subjects: Vec<String>,
}

impl ValidationRule {
    /// Creates a blocking validation rule.
    pub fn error<I, S>(
        level: ValidationLevel,
        condition: Condition,
        subjects: I,
        message: impl Into<String>,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            level,
            condition,
            severity: DiagnosticSeverity::Error,
            message: message.into(),
            subjects: subjects.into_iter().map(Into::into).collect(),
        }
    }

    /// Creates a warning validation rule.
    pub fn warning<I, S>(
        level: ValidationLevel,
        condition: Condition,
        subjects: I,
        message: impl Into<String>,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            level,
            condition,
            severity: DiagnosticSeverity::Warning,
            message: message.into(),
            subjects: subjects.into_iter().map(Into::into).collect(),
        }
    }

    pub(super) fn prefixed(&self, prefix: &str) -> Self {
        Self {
            level: self.level,
            condition: self.condition.prefixed(prefix),
            severity: self.severity,
            message: self.message.clone(),
            subjects: self
                .subjects
                .iter()
                .map(|subject| prefix_path(prefix, subject))
                .collect(),
        }
    }

    /// Gates this validity rule behind a presence or enablement condition.
    ///
    /// A validation rule describes a condition that must be true.  When the
    /// gate is false, the rule is not applicable and therefore succeeds.
    #[must_use]
    pub fn gated_by(mut self, gate: Condition) -> Self {
        self.condition = Condition::Any(vec![Condition::Not(Box::new(gate)), self.condition]);
        self
    }
}
