//! Definition diagnostics and component-state evaluation for a resolved candidate.

use crate::definition::{
    self, Diagnostic, DiagnosticCode, DiagnosticSeverity, Schema, SettingSpec, ValidationLevel,
};
use crate::report::ComponentPresence;
use crate::resolution::get_path;
use serde_json::Value;
use std::collections::BTreeMap;

pub(super) fn collect_diagnostics(schema: &Schema, values: &Value) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    collect_setting_diagnostics(schema, values, &mut diagnostics);

    // Section/component/application rules are meaningful only after the value
    // layer is definition-valid. This preserves the documented validation
    // order and avoids cascading relationship errors from malformed inputs.
    if !has_errors(&diagnostics) {
        collect_rule_diagnostics(schema, values, &mut diagnostics);
    }
    diagnostics
}

fn collect_setting_diagnostics(schema: &Schema, values: &Value, diagnostics: &mut Vec<Diagnostic>) {
    for (path, setting) in schema.iter() {
        collect_setting_diagnostic(path, setting, values, diagnostics);
    }
}

fn collect_setting_diagnostic(
    path: &str,
    setting: &SettingSpec,
    values: &Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(value) = get_path(values, path) else {
        if setting.requirement.is_required(values) {
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::MissingRequired,
                Some(path.to_string()),
                vec![path.to_string()],
                "required setting is missing",
            ));
        }
        return;
    };

    if value.is_null() {
        if !setting.nullable {
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::InvalidType,
                Some(path.to_string()),
                vec![path.to_string()],
                "null is not permitted",
            ));
        } else {
            collect_deprecation_diagnostic(path, setting, diagnostics);
        }
        return;
    }

    if !definition::value_matches_shape(setting.shape, value)
        || !definition::value_matches_type(setting, value)
    {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::InvalidType,
            Some(path.to_string()),
            vec![path.to_string()],
            format!(
                "value must match shape {:?} and type {:?}",
                setting.shape, setting.value_type
            ),
        ));
        return;
    }

    if !definition::value_matches_representation(setting, value) {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::InvalidRepresentation,
            Some(path.to_string()),
            vec![path.to_string()],
            format!("value must use representation {:?}", setting.representation),
        ));
        return;
    }

    collect_constraint_diagnostics(path, setting, value, diagnostics);
    collect_deprecation_diagnostic(path, setting, diagnostics);
}

fn has_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
}

fn collect_rule_diagnostics(schema: &Schema, values: &Value, diagnostics: &mut Vec<Diagnostic>) {
    for level in [
        ValidationLevel::Section,
        ValidationLevel::Component,
        ValidationLevel::Application,
    ] {
        for rule in schema.rules().iter().filter(|rule| rule.level == level) {
            if !rule.condition.evaluate(values) {
                let diagnostic = match rule.severity {
                    DiagnosticSeverity::Error => Diagnostic::error(
                        DiagnosticCode::RuleViolation,
                        rule.subjects.first().cloned(),
                        rule.subjects.clone(),
                        rule.message.clone(),
                    ),
                    DiagnosticSeverity::Warning => Diagnostic::warning(
                        DiagnosticCode::RuleViolation,
                        rule.subjects.first().cloned(),
                        rule.subjects.clone(),
                        rule.message.clone(),
                    ),
                };
                diagnostics.push(diagnostic);
            }
        }
    }
}

fn collect_deprecation_diagnostic(
    path: &str,
    setting: &SettingSpec,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(deprecation) = &setting.deprecation else {
        return;
    };
    let message = match &deprecation.replacement {
        Some(replacement) => format!("{}; use `{replacement}` instead", deprecation.message),
        None => deprecation.message.clone(),
    };
    diagnostics.push(Diagnostic::warning(
        DiagnosticCode::DeprecatedSetting,
        Some(path.to_string()),
        vec![path.to_string()],
        message,
    ));
}

fn collect_constraint_diagnostics(
    path: &str,
    setting: &SettingSpec,
    value: &Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for constraint in &setting.constraints {
        if let Some(message) = definition::constraint_violation(constraint, value) {
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::ConstraintViolation,
                Some(path.to_string()),
                vec![path.to_string()],
                message,
            ));
        }
    }
}

pub(super) fn evaluate_component_states(
    schema: &Schema,
    values: &Value,
) -> BTreeMap<String, ComponentPresence> {
    let Some(metadata) = schema.metadata() else {
        return BTreeMap::new();
    };

    metadata
        .components
        .iter()
        .map(|(key, component)| {
            let enabled = component
                .enabled_when
                .as_ref()
                .is_none_or(|condition| condition.evaluate(values));
            (
                key.clone(),
                if enabled {
                    ComponentPresence::Enabled
                } else {
                    ComponentPresence::Disabled
                },
            )
        })
        .collect()
}
