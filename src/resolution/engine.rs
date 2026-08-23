//! Deterministic resolution orchestration.

use super::contribution::Contribution;
use super::diagnostics::{collect_diagnostics, evaluate_component_states};
use super::merge::apply_setting;
use super::provenance::record_default_provenance;
use super::{
    Context, EffectiveConfiguration, InputTarget, LayerOrder, LogicalInput, ProvenanceEntry,
    ResolutionInformation, ResolveError, get_path, set_path,
};
use crate::ResolutionLimits;
use crate::definition::{
    ApplicationDefinition, DefaultRule, DiagnosticSeverity, Schema, SettingSpec,
};
use crate::report::ResolutionReport;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// Deterministic, source-agnostic configuration resolver.
pub struct Resolver {
    schema: Schema,
    default_order: Vec<String>,
    layers: LayerOrder,
    limits: ResolutionLimits,
}

struct ResolutionState {
    values: Value,
    provenance: BTreeMap<String, Vec<ProvenanceEntry>>,
}

impl ResolutionState {
    fn new() -> Self {
        Self {
            values: Value::Object(Map::new()),
            provenance: BTreeMap::new(),
        }
    }
}

impl Resolver {
    /// Creates a resolver after validating its schema and layer order.
    pub fn new(schema: Schema, layers: LayerOrder) -> Result<Self, ResolveError> {
        Self::with_limits(schema, layers, ResolutionLimits::default())
    }

    /// Creates a resolver with an explicit bounded-work policy.
    pub fn with_limits(
        schema: Schema,
        layers: LayerOrder,
        limits: ResolutionLimits,
    ) -> Result<Self, ResolveError> {
        schema.validate()?;
        let default_order = schema.default_order()?;
        limits
            .validate()
            .map_err(|message| ResolveError::InvalidSchema(message.into()))?;
        Ok(Self {
            schema,
            default_order,
            layers,
            limits,
        })
    }

    /// Creates a resolver from a composed application definition.
    pub fn for_application(
        application: &ApplicationDefinition,
        layers: LayerOrder,
    ) -> Result<Self, ResolveError> {
        Self::new(application.compile()?, layers)
    }

    /// Returns the validated schema.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Returns the explicit layer order.
    pub fn layers(&self) -> &LayerOrder {
        &self.layers
    }

    /// Returns the bounded-work policy applied to every resolution.
    pub fn limits(&self) -> &ResolutionLimits {
        &self.limits
    }

    /// Resolves configuration using compatibility fail-fast error behavior.
    ///
    /// Use [`Self::resolve_report`] when callers need all definition-validation
    /// diagnostics from the complete candidate.
    pub fn resolve(
        &self,
        context: &Context,
        inputs: &[LogicalInput],
    ) -> Result<EffectiveConfiguration, ResolveError> {
        let report = self.resolve_report(context, inputs)?;
        let first_error = report.first_error();
        match report.effective {
            Some(effective) => Ok(effective),
            None => Err(first_error.unwrap_or_else(|| {
                ResolveError::InvalidSchema(
                    "resolution failed without a blocking diagnostic".into(),
                )
            })),
        }
    }

    /// Resolves configuration and returns structured diagnostics and normal
    /// resolution information separately.
    pub fn resolve_report(
        &self,
        context: &Context,
        inputs: &[LogicalInput],
    ) -> Result<ResolutionReport, ResolveError> {
        validate_resolution_size(context, inputs, &self.limits)?;
        let mut information = Vec::new();
        let mut contributions = self.collect_contributions(context, inputs, &mut information)?;
        let state = self.apply_defaults_and_contributions(&mut contributions, &mut information)?;
        self.record_optional_omissions(&state.values, &mut information);
        Ok(self.build_report(context, state, information))
    }

    fn collect_contributions<'a>(
        &self,
        context: &Context,
        inputs: &'a [LogicalInput],
        information: &mut Vec<ResolutionInformation>,
    ) -> Result<BTreeMap<String, Vec<Contribution<'a>>>, ResolveError> {
        let mut by_path: BTreeMap<String, Vec<Contribution<'a>>> = BTreeMap::new();
        for input in inputs {
            let rank = self.layers.rank(&input.layer)?;
            if !input.selector.matches(context)? {
                information.push(ResolutionInformation::InputIgnored {
                    input_id: input.id.clone(),
                    target: input.target.clone(),
                    origin: input.origin.clone(),
                    reason: "selector did not match stable context".to_string(),
                });
                continue;
            }

            for op in &input.operations {
                let path = targeted_operation_path(&self.schema, &input.target, op.path())?;
                if self.schema.get(&path).is_none() {
                    return Err(ResolveError::UnknownSetting(path));
                }
                by_path.entry(path).or_default().push(Contribution {
                    layer_rank: rank,
                    specificity: input.selector.specificity(),
                    input,
                    op,
                });
            }
        }
        Ok(by_path)
    }

    fn apply_defaults_and_contributions(
        &self,
        by_path: &mut BTreeMap<String, Vec<Contribution<'_>>>,
        information: &mut Vec<ResolutionInformation>,
    ) -> Result<ResolutionState, ResolveError> {
        let mut state = ResolutionState::new();
        for path in &self.default_order {
            let setting = &self.schema.settings[path];
            apply_default(path, setting, &mut state, &mut *information)?;

            if let Some(contributions) = by_path.get_mut(path) {
                contributions.sort_by_key(|contribution| {
                    (
                        contribution.layer_rank,
                        contribution.specificity,
                        contribution.input.order.unwrap_or(0),
                    )
                });
                apply_setting(
                    path,
                    setting,
                    contributions,
                    &mut state.values,
                    &mut state.provenance,
                    &mut *information,
                )?;
            }
        }
        Ok(state)
    }

    fn record_optional_omissions(
        &self,
        values: &Value,
        information: &mut Vec<ResolutionInformation>,
    ) {
        for (path, setting) in self.schema.iter() {
            if get_path(values, path).is_none() && !setting.requirement.is_required(values) {
                information.push(ResolutionInformation::OptionalOmitted {
                    path: path.to_string(),
                });
            }
        }
    }

    fn build_report(
        &self,
        context: &Context,
        state: ResolutionState,
        information: Vec<ResolutionInformation>,
    ) -> ResolutionReport {
        let diagnostics = collect_diagnostics(&self.schema, &state.values);
        let blocking = diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error);

        if blocking {
            return ResolutionReport {
                effective: None,
                diagnostics,
                information,
            };
        }

        let components = evaluate_component_states(&self.schema, &state.values);
        let definition = self
            .schema
            .metadata()
            .map(|metadata| metadata.application.clone());
        let effective = EffectiveConfiguration {
            values: state.values,
            provenance: state.provenance,
            context: context.clone(),
            information: information.clone(),
            diagnostics: diagnostics.clone(),
            definition,
            components,
        };

        ResolutionReport {
            effective: Some(effective),
            diagnostics,
            information,
        }
    }
}

fn apply_default(
    path: &str,
    setting: &SettingSpec,
    state: &mut ResolutionState,
    information: &mut Vec<ResolutionInformation>,
) -> Result<(), ResolveError> {
    let (default, derived_source) = match &setting.default {
        Some(DefaultRule::Fixed(value)) => (Some(value.clone()), None),
        Some(DefaultRule::Lookup { source, cases }) => (
            get_path(&state.values, source)
                .and_then(value_key)
                .and_then(|key| cases.get(&key).cloned()),
            Some(source.as_str()),
        ),
        None => (None, None),
    };
    let Some(value) = default else {
        return Ok(());
    };

    set_path(&mut state.values, path, value.clone())?;
    record_default_provenance(path, setting, &value, &mut state.provenance);
    match derived_source {
        Some(source) => information.push(ResolutionInformation::DerivedDefaultUsed {
            path: path.into(),
            source: source.into(),
        }),
        None => information.push(ResolutionInformation::DefaultUsed { path: path.into() }),
    }
    Ok(())
}

fn validate_resolution_size(
    context: &Context,
    inputs: &[LogicalInput],
    limits: &ResolutionLimits,
) -> Result<(), ResolveError> {
    if inputs.len() > limits.max_inputs {
        return Err(ResolveError::LimitExceeded {
            resource: "input count".into(),
            limit: limits.max_inputs,
        });
    }
    let operation_count = inputs
        .iter()
        .try_fold(0_usize, |count, input| {
            count.checked_add(input.operations.len())
        })
        .unwrap_or(usize::MAX);
    if operation_count > limits.max_operations {
        return Err(ResolveError::LimitExceeded {
            resource: "operation count".into(),
            limit: limits.max_operations,
        });
    }

    let mut logical_bytes = 0_usize;
    for (key, value) in context.iter() {
        count_logical_text(key, &mut logical_bytes, limits)?;
        count_logical_text(value, &mut logical_bytes, limits)?;
    }
    for input in inputs {
        count_logical_text(&input.id, &mut logical_bytes, limits)?;
        count_logical_text(&input.layer, &mut logical_bytes, limits)?;
        if let InputTarget::Component(component) = &input.target {
            count_logical_text(component, &mut logical_bytes, limits)?;
        }
        count_logical_text(&input.origin.source, &mut logical_bytes, limits)?;
        if let Some(detail) = &input.origin.detail {
            count_logical_text(detail, &mut logical_bytes, limits)?;
        }
        for (key, value) in input.selector.iter() {
            count_logical_text(key, &mut logical_bytes, limits)?;
            count_logical_text(value, &mut logical_bytes, limits)?;
        }
        for operation in &input.operations {
            count_logical_text(operation.path(), &mut logical_bytes, limits)?;
            if let super::Operation::RemoveMapEntry { key, .. } = operation {
                count_logical_text(key, &mut logical_bytes, limits)?;
            }
        }
    }

    let mut nodes = 0_usize;
    for value in inputs.iter().flat_map(|input| {
        input
            .operations
            .iter()
            .filter_map(|operation| match operation {
                super::Operation::Set { value, .. }
                | super::Operation::RemoveListItem { value, .. } => Some(value),
                _ => None,
            })
    }) {
        let mut stack = vec![(value, 1_usize)];
        while let Some((value, depth)) = stack.pop() {
            nodes = nodes.saturating_add(1);
            if nodes > limits.max_value_nodes {
                return Err(ResolveError::LimitExceeded {
                    resource: "value node count".into(),
                    limit: limits.max_value_nodes,
                });
            }
            if depth > limits.max_value_depth {
                return Err(ResolveError::LimitExceeded {
                    resource: "value nesting depth".into(),
                    limit: limits.max_value_depth,
                });
            }
            match value {
                Value::String(value) => {
                    count_logical_text(value, &mut logical_bytes, limits)?;
                }
                Value::Array(values) => {
                    stack.extend(values.iter().map(|value| (value, depth + 1)));
                }
                Value::Object(values) => {
                    for key in values.keys() {
                        count_logical_text(key, &mut logical_bytes, limits)?;
                    }
                    stack.extend(values.values().map(|value| (value, depth + 1)));
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn count_logical_text(
    value: &str,
    total: &mut usize,
    limits: &ResolutionLimits,
) -> Result<(), ResolveError> {
    if value.len() > limits.max_scalar_bytes {
        return Err(ResolveError::LimitExceeded {
            resource: "logical scalar bytes".into(),
            limit: limits.max_scalar_bytes,
        });
    }
    *total = total.saturating_add(value.len());
    if *total > limits.max_logical_bytes {
        return Err(ResolveError::LimitExceeded {
            resource: "aggregate logical bytes".into(),
            limit: limits.max_logical_bytes,
        });
    }
    Ok(())
}

fn targeted_operation_path(
    schema: &Schema,
    target: &InputTarget,
    operation_path: &str,
) -> Result<String, ResolveError> {
    match target {
        InputTarget::Root => Ok(operation_path.to_string()),
        InputTarget::Component(key) => {
            let component = schema
                .metadata()
                .and_then(|metadata| metadata.components.get(key))
                .ok_or_else(|| ResolveError::UnknownTarget(key.clone()))?;
            if operation_path.is_empty() {
                Ok(component.path.clone())
            } else {
                Ok(format!("{}.{}", component.path, operation_path))
            }
        }
    }
}
fn value_key(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}
