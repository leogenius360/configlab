//! Per-setting merge, accumulation, and removal mechanics.

use super::provenance::{
    current_effective_layer, provenance_child_path, provenance_entry, provenance_value,
    record_named_map_provenance, record_nested_provenance,
};
use super::{
    Operation, ProvenanceAction, ProvenanceEntry, ResolutionInformation, ResolveError,
    contribution::Contribution, get_path, get_path_mut, remove_path, set_path,
};
use crate::definition::{MergePolicy, SettingSpec, ValueShape};
use serde_json::{Map, Value};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use super::peer::conflicting_peers;

pub(super) fn apply_setting(
    path: &str,
    setting: &SettingSpec,
    contributions: &[Contribution<'_>],
    values: &mut Value,
    provenance: &mut BTreeMap<String, Vec<ProvenanceEntry>>,
    information: &mut Vec<ResolutionInformation>,
) -> Result<(), ResolveError> {
    let mut layers: BTreeMap<usize, Vec<&Contribution<'_>>> = BTreeMap::new();
    for contribution in contributions {
        layers
            .entry(contribution.layer_rank)
            .or_default()
            .push(contribution);
    }

    for (_, group) in layers {
        let max_specificity = group
            .iter()
            .map(|contribution| contribution.specificity)
            .max()
            .unwrap_or(0);
        record_shadowed_candidates(path, setting, &group, max_specificity, provenance);

        let mut peers: Vec<_> = group
            .into_iter()
            .filter(|contribution| contribution.specificity == max_specificity)
            .collect();
        prepare_peers(path, setting, &mut peers, max_specificity)?;

        for contribution in peers {
            apply_op(path, setting, contribution, values, provenance, information)?;
        }
    }
    Ok(())
}

fn record_shadowed_candidates(
    path: &str,
    setting: &SettingSpec,
    group: &[&Contribution<'_>],
    max_specificity: usize,
    provenance: &mut BTreeMap<String, Vec<ProvenanceEntry>>,
) {
    for contribution in group
        .iter()
        .filter(|contribution| contribution.specificity < max_specificity)
    {
        let value = match contribution.op {
            Operation::Set { value, .. } => Some(provenance_value(setting, value)),
            _ => None,
        };
        provenance
            .entry(path.into())
            .or_default()
            .push(ProvenanceEntry {
                layer: contribution.input.layer.clone(),
                specificity: contribution.specificity,
                input_id: contribution.input.id.clone(),
                target: contribution.input.target.clone(),
                origin: contribution.input.origin.clone(),
                action: ProvenanceAction::Shadowed,
                value,
                item: None,
            });
    }
}

fn prepare_peers(
    path: &str,
    setting: &SettingSpec,
    peers: &mut [&Contribution<'_>],
    specificity: usize,
) -> Result<(), ResolveError> {
    match setting.merge {
        MergePolicy::Append | MergePolicy::Prepend if peers.len() > 1 => {
            if peers
                .iter()
                .any(|contribution| contribution.input.order.is_none())
                || has_duplicate_orders(peers)
            {
                return Err(ResolveError::UndefinedOrder {
                    path: path.into(),
                    inputs: sorted_input_ids(peers),
                });
            }
            peers.sort_by_key(|contribution| contribution.input.order);
        }
        MergePolicy::Append | MergePolicy::Prepend => {}
        _ if peers.len() > 1 && conflicting_peers(setting, peers)? => {
            return Err(ResolveError::Ambiguous {
                path: path.into(),
                layer: peers[0].input.layer.clone(),
                specificity,
                inputs: sorted_input_ids(peers),
            });
        }
        _ => sort_compatible_peers(peers),
    }
    Ok(())
}

/// Canonicalizes commutative peers so provenance does not inherit caller order.
///
/// Compatibility checking has already established that these operations have
/// an order-independent effect. The full logical identity is used after the
/// stable input id so even accidentally repeated ids have deterministic
/// presentation order.
fn sort_compatible_peers(peers: &mut [&Contribution<'_>]) {
    peers.sort_by(|left, right| {
        left.input
            .id
            .cmp(&right.input.id)
            .then_with(|| {
                input_target_key(&left.input.target).cmp(&input_target_key(&right.input.target))
            })
            .then_with(|| left.input.selector.iter().cmp(right.input.selector.iter()))
            .then_with(|| left.input.order.cmp(&right.input.order))
            .then_with(|| left.input.origin.source.cmp(&right.input.origin.source))
            .then_with(|| left.input.origin.detail.cmp(&right.input.origin.detail))
            .then_with(|| compare_operations(left.op, right.op))
    });
}

fn input_target_key(target: &super::InputTarget) -> (u8, &str) {
    match target {
        super::InputTarget::Root => (0, ""),
        super::InputTarget::Component(key) => (1, key),
    }
}

fn compare_operations(left: &Operation, right: &Operation) -> Ordering {
    operation_rank(left)
        .cmp(&operation_rank(right))
        .then_with(|| left.path().cmp(right.path()))
        .then_with(|| match (left, right) {
            (Operation::Set { value: left, .. }, Operation::Set { value: right, .. })
            | (
                Operation::RemoveListItem { value: left, .. },
                Operation::RemoveListItem { value: right, .. },
            ) => compare_values(left, right),
            (
                Operation::RemoveMapEntry { key: left, .. },
                Operation::RemoveMapEntry { key: right, .. },
            ) => left.cmp(right),
            _ => Ordering::Equal,
        })
}

fn operation_rank(operation: &Operation) -> u8 {
    match operation {
        Operation::Set { .. } => 0,
        Operation::Unset { .. } => 1,
        Operation::Clear { .. } => 2,
        Operation::RemoveMapEntry { .. } => 3,
        Operation::RemoveListItem { .. } => 4,
    }
}

fn compare_values(left: &Value, right: &Value) -> Ordering {
    value_rank(left)
        .cmp(&value_rank(right))
        .then_with(|| match (left, right) {
            (Value::Null, Value::Null) => Ordering::Equal,
            (Value::Bool(left), Value::Bool(right)) => left.cmp(right),
            (Value::Number(left), Value::Number(right)) => left.to_string().cmp(&right.to_string()),
            (Value::String(left), Value::String(right)) => left.cmp(right),
            (Value::Array(left), Value::Array(right)) => compare_value_slices(left, right),
            (Value::Object(left), Value::Object(right)) => compare_value_maps(left, right),
            _ => Ordering::Equal,
        })
}

fn value_rank(value: &Value) -> u8 {
    match value {
        Value::Null => 0,
        Value::Bool(_) => 1,
        Value::Number(_) => 2,
        Value::String(_) => 3,
        Value::Array(_) => 4,
        Value::Object(_) => 5,
    }
}

fn compare_value_slices(left: &[Value], right: &[Value]) -> Ordering {
    for (left, right) in left.iter().zip(right) {
        let ordering = compare_values(left, right);
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    left.len().cmp(&right.len())
}

fn compare_value_maps(left: &Map<String, Value>, right: &Map<String, Value>) -> Ordering {
    let mut left_keys = left.keys().collect::<Vec<_>>();
    let mut right_keys = right.keys().collect::<Vec<_>>();
    left_keys.sort_unstable();
    right_keys.sort_unstable();

    for (left_key, right_key) in left_keys.iter().zip(&right_keys) {
        let ordering = left_key.cmp(right_key).then_with(|| {
            compare_values(
                left.get(*left_key).expect("key came from this map"),
                right.get(*right_key).expect("key came from this map"),
            )
        });
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    left_keys.len().cmp(&right_keys.len())
}

fn sorted_input_ids(peers: &[&Contribution<'_>]) -> Vec<String> {
    let mut inputs = peers
        .iter()
        .map(|contribution| contribution.input.id.clone())
        .collect::<Vec<_>>();
    inputs.sort();
    inputs
}

fn has_duplicate_orders(peers: &[&Contribution<'_>]) -> bool {
    let mut seen = BTreeSet::new();
    peers
        .iter()
        .filter_map(|contribution| contribution.input.order)
        .any(|order| !seen.insert(order))
}

struct ApplyOpContext<'a, 'input> {
    path: &'a str,
    setting: &'a SettingSpec,
    contribution: &'a Contribution<'input>,
    values: &'a mut Value,
    provenance: &'a mut BTreeMap<String, Vec<ProvenanceEntry>>,
    information: &'a mut Vec<ResolutionInformation>,
}

fn apply_op(
    path: &str,
    setting: &SettingSpec,
    contribution: &Contribution<'_>,
    values: &mut Value,
    provenance: &mut BTreeMap<String, Vec<ProvenanceEntry>>,
    information: &mut Vec<ResolutionInformation>,
) -> Result<(), ResolveError> {
    let mut context = ApplyOpContext {
        path,
        setting,
        contribution,
        values,
        provenance,
        information,
    };
    let entries = match contribution.op {
        Operation::Set { value, .. } => apply_set(&mut context, value)?,
        Operation::Unset { .. } => apply_unset(&mut context)?,
        Operation::Clear { .. } => apply_clear(&mut context)?,
        Operation::RemoveMapEntry { key, .. } => apply_remove_map_entry(&mut context, key)?,
        Operation::RemoveListItem { value, .. } => apply_remove_list_item(&mut context, value)?,
    };

    context
        .provenance
        .entry(context.path.into())
        .or_default()
        .extend(entries);
    Ok(())
}

fn apply_set(
    context: &mut ApplyOpContext<'_, '_>,
    value: &Value,
) -> Result<Vec<ProvenanceEntry>, ResolveError> {
    validate_merge_input(context.setting, value)?;
    let current = get_path(context.values, context.path).cloned();
    record_replacement_override(context, current.is_some());

    let (next, mut entries) = match context.setting.merge {
        MergePolicy::Replace => (value.clone(), Vec::new()),
        MergePolicy::Deep | MergePolicy::CombineByKey => {
            let mut base = current.unwrap_or(Value::Object(Map::new()));
            merge_structured_value(
                context.path,
                context.setting.merge,
                &mut base,
                value,
                CollisionBehavior::Overwrite,
            )?;
            (base, Vec::new())
        }
        MergePolicy::Append => append_value(context, current, value)?,
        MergePolicy::Prepend => prepend_value(context, current, value)?,
    };

    set_path(context.values, context.path, next)?;
    record_set_provenance(context, value, &mut entries);
    Ok(entries)
}

fn record_replacement_override(context: &mut ApplyOpContext<'_, '_>, current_exists: bool) {
    if current_exists
        && context.setting.merge == MergePolicy::Replace
        && let Some(previous_layer) =
            current_effective_layer(context.provenance.get(context.path).map(Vec::as_slice))
        && previous_layer != context.contribution.input.layer
    {
        context
            .information
            .push(ResolutionInformation::ValueOverridden {
                path: context.path.to_string(),
                previous_layer: previous_layer.to_string(),
                winning_layer: context.contribution.input.layer.clone(),
            });
    }
}

fn append_value(
    context: &ApplyOpContext<'_, '_>,
    current: Option<Value>,
    value: &Value,
) -> Result<(Value, Vec<ProvenanceEntry>), ResolveError> {
    let mut list = current
        .and_then(|candidate| candidate.as_array().cloned())
        .unwrap_or_default();
    let start = list.len();
    let incoming = value.as_array().ok_or_else(|| ResolveError::InvalidValue {
        path: context.path.into(),
        message: "append requires a list".into(),
    })?;
    let entries = list_item_provenance(context, incoming, start);
    list.extend(incoming.iter().cloned());
    Ok((Value::Array(list), entries))
}

fn prepend_value(
    context: &mut ApplyOpContext<'_, '_>,
    current: Option<Value>,
    value: &Value,
) -> Result<(Value, Vec<ProvenanceEntry>), ResolveError> {
    let incoming = value
        .as_array()
        .cloned()
        .ok_or_else(|| ResolveError::InvalidValue {
            path: context.path.into(),
            message: "prepend requires a list".into(),
        })?;
    shift_existing_item_provenance(context, incoming.len());
    let entries = list_item_provenance(context, &incoming, 0);

    let mut next = incoming;
    let mut old = current
        .and_then(|candidate| candidate.as_array().cloned())
        .unwrap_or_default();
    next.append(&mut old);
    Ok((Value::Array(next), entries))
}

fn list_item_provenance(
    context: &ApplyOpContext<'_, '_>,
    items: &[Value],
    start: usize,
) -> Vec<ProvenanceEntry> {
    if context.setting.is_sensitive() {
        return Vec::new();
    }

    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            provenance_entry(
                context.setting,
                context.contribution,
                ProvenanceAction::Set,
                Some(item),
                Some(start + index),
            )
        })
        .collect()
}

fn shift_existing_item_provenance(context: &mut ApplyOpContext<'_, '_>, shift: usize) {
    if !context.setting.is_sensitive()
        && let Some(existing) = context.provenance.get_mut(context.path)
    {
        for entry in existing.iter_mut() {
            if let Some(index) = entry.item {
                entry.item = Some(index + shift);
            }
        }
    }
}

fn record_set_provenance(
    context: &mut ApplyOpContext<'_, '_>,
    value: &Value,
    entries: &mut Vec<ProvenanceEntry>,
) {
    if !entries.is_empty() {
        return;
    }

    entries.push(provenance_entry(
        context.setting,
        context.contribution,
        ProvenanceAction::Set,
        Some(value),
        None,
    ));
    match context.setting.merge {
        MergePolicy::Deep => record_nested_provenance(
            context.path,
            context.setting,
            value,
            context.contribution,
            context.provenance,
        ),
        MergePolicy::CombineByKey => record_named_map_provenance(
            context.path,
            context.setting,
            value,
            context.contribution,
            context.provenance,
        ),
        _ => {}
    }
}

fn apply_unset(context: &mut ApplyOpContext<'_, '_>) -> Result<Vec<ProvenanceEntry>, ResolveError> {
    ensure_removal_allowed(context.path, "unset", context.setting.removal.unset)?;
    remove_path(context.values, context.path);
    Ok(vec![provenance_entry(
        context.setting,
        context.contribution,
        ProvenanceAction::Unset,
        None,
        None,
    )])
}

fn apply_clear(context: &mut ApplyOpContext<'_, '_>) -> Result<Vec<ProvenanceEntry>, ResolveError> {
    ensure_removal_allowed(context.path, "clear", context.setting.removal.clear)?;
    let empty = match context.setting.shape {
        ValueShape::OrderedList => Value::Array(Vec::new()),
        ValueShape::Object | ValueShape::NamedMap => Value::Object(Map::new()),
        _ => {
            return Err(ResolveError::InvalidValue {
                path: context.path.into(),
                message: "clear requires a collection setting".into(),
            });
        }
    };
    set_path(context.values, context.path, empty)?;
    if let Some(existing) = context.provenance.get_mut(context.path) {
        for entry in existing.iter_mut() {
            entry.item = None;
        }
    }
    Ok(vec![provenance_entry(
        context.setting,
        context.contribution,
        ProvenanceAction::Clear,
        None,
        None,
    )])
}

fn apply_remove_map_entry(
    context: &mut ApplyOpContext<'_, '_>,
    key: &str,
) -> Result<Vec<ProvenanceEntry>, ResolveError> {
    ensure_removal_allowed(
        context.path,
        "remove-map-entry",
        context.setting.removal.remove_map_entry,
    )?;
    if context.setting.shape != ValueShape::NamedMap {
        return Err(ResolveError::InvalidValue {
            path: context.path.into(),
            message: "remove-map-entry requires a named-map setting".into(),
        });
    }

    let removed = get_path_mut(context.values, context.path)
        .and_then(Value::as_object_mut)
        .and_then(|map| map.remove(key));
    if removed.is_none() {
        context
            .information
            .push(ResolutionInformation::RemovalNoop {
                path: context.path.into(),
                operation: super::RemovalKind::MapEntry,
            });
    }
    if !context.setting.is_sensitive()
        && let Some(removed) = removed.as_ref()
    {
        context
            .provenance
            .entry(provenance_child_path(context.path, key))
            .or_default()
            .push(provenance_entry(
                context.setting,
                context.contribution,
                ProvenanceAction::RemoveMapEntry,
                Some(removed),
                None,
            ));
    }
    let key_value = Value::String(key.to_owned());
    Ok(vec![provenance_entry(
        context.setting,
        context.contribution,
        ProvenanceAction::RemoveMapEntry,
        Some(&key_value),
        None,
    )])
}

fn apply_remove_list_item(
    context: &mut ApplyOpContext<'_, '_>,
    value: &Value,
) -> Result<Vec<ProvenanceEntry>, ResolveError> {
    ensure_removal_allowed(
        context.path,
        "remove-list-item",
        context.setting.removal.remove_list_item,
    )?;
    if context.setting.shape != ValueShape::OrderedList {
        return Err(ResolveError::InvalidValue {
            path: context.path.into(),
            message: "remove-list-item requires an ordered-list setting".into(),
        });
    }

    let removed_index = get_path_mut(context.values, context.path)
        .and_then(Value::as_array_mut)
        .and_then(|list| {
            let index = list.iter().position(|candidate| candidate == value)?;
            list.remove(index);
            Some(index)
        });

    if let (Some(index), Some(existing)) = (removed_index, context.provenance.get_mut(context.path))
    {
        for entry in existing.iter_mut() {
            match entry.item {
                Some(item) if item == index => entry.item = None,
                Some(item) if item > index => entry.item = Some(item - 1),
                _ => {}
            }
        }
    }
    if removed_index.is_none() {
        context
            .information
            .push(ResolutionInformation::RemovalNoop {
                path: context.path.into(),
                operation: super::RemovalKind::ListItem,
            });
    }

    Ok(vec![provenance_entry(
        context.setting,
        context.contribution,
        ProvenanceAction::RemoveListItem,
        Some(value),
        None,
    )])
}

fn ensure_removal_allowed(path: &str, operation: &str, allowed: bool) -> Result<(), ResolveError> {
    if allowed {
        Ok(())
    } else {
        Err(ResolveError::RemovalNotAllowed {
            path: path.into(),
            operation: operation.into(),
        })
    }
}

fn validate_merge_input(setting: &SettingSpec, value: &Value) -> Result<(), ResolveError> {
    if value.is_null() {
        return if matches!(setting.merge, MergePolicy::Replace) {
            Ok(())
        } else {
            Err(ResolveError::InvalidValue {
                path: setting.path.clone(),
                message: format!("null cannot be used with merge policy {:?}", setting.merge),
            })
        };
    }

    let structurally_mergeable = match setting.merge {
        MergePolicy::Replace => true,
        MergePolicy::Deep | MergePolicy::CombineByKey => value.is_object(),
        MergePolicy::Append | MergePolicy::Prepend => value.is_array(),
    };

    if structurally_mergeable {
        Ok(())
    } else {
        Err(ResolveError::InvalidValue {
            path: setting.path.clone(),
            message: format!(
                "value cannot be combined with merge policy {:?}",
                setting.merge
            ),
        })
    }
}

/// Determines how an existing merge unit handles a colliding incoming value.
///
/// Application overwrites the unit, while equal-precedence peer validation
/// admits only equal collisions. Both modes intentionally share the same
/// structural traversal so their interpretation of merge units cannot drift.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CollisionBehavior {
    Overwrite,
    RejectUnequal,
}

/// Applies a structured merge policy and reports an incompatible collision.
///
/// `Deep` treats non-object leaves as merge units at every depth;
/// `CombineByKey` treats each direct map entry as one atomic merge unit.
pub(super) fn merge_structured_value(
    path: &str,
    policy: MergePolicy,
    target: &mut Value,
    incoming: &Value,
    collision: CollisionBehavior,
) -> Result<bool, ResolveError> {
    match policy {
        MergePolicy::Deep => Ok(merge_deep_value(target, incoming, collision)),
        MergePolicy::CombineByKey => merge_by_key_value(path, target, incoming, collision),
        _ => unreachable!("structured merge requires Deep or CombineByKey"),
    }
}

fn merge_by_key_value(
    path: &str,
    target: &mut Value,
    incoming: &Value,
    collision: CollisionBehavior,
) -> Result<bool, ResolveError> {
    let target_map = target
        .as_object_mut()
        .ok_or_else(|| ResolveError::InvalidValue {
            path: path.into(),
            message: "combine-by-key requires an object".into(),
        })?;
    let incoming_map = incoming
        .as_object()
        .ok_or_else(|| ResolveError::InvalidValue {
            path: path.into(),
            message: "combine-by-key requires an object".into(),
        })?;
    for (key, value) in incoming_map {
        match target_map.get_mut(key) {
            Some(current) => {
                if merge_unit_conflicts(current, value, collision) {
                    return Ok(true);
                }
            }
            None => {
                target_map.insert(key.clone(), value.clone());
            }
        }
    }
    Ok(false)
}

fn merge_deep_value(target: &mut Value, incoming: &Value, collision: CollisionBehavior) -> bool {
    match (target, incoming) {
        (Value::Object(existing), Value::Object(incoming)) => {
            for (key, value) in incoming {
                match existing.get_mut(key) {
                    Some(current) => {
                        if merge_deep_value(current, value, collision) {
                            return true;
                        }
                    }
                    None => {
                        existing.insert(key.clone(), value.clone());
                    }
                }
            }
            false
        }
        (target, incoming) => merge_unit_conflicts(target, incoming, collision),
    }
}

fn merge_unit_conflicts(
    target: &mut Value,
    incoming: &Value,
    collision: CollisionBehavior,
) -> bool {
    match collision {
        CollisionBehavior::Overwrite => {
            target.clone_from(incoming);
            false
        }
        CollisionBehavior::RejectUnequal => target != incoming,
    }
}
