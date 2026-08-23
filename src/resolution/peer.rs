//! Equal-layer/equal-specificity peer compatibility rules.

use super::{
    Operation, ResolveError,
    contribution::Contribution,
    merge::{CollisionBehavior, merge_structured_value},
};
use crate::definition::{MergePolicy, SettingSpec};
use serde_json::{Map, Value};

/// Returns whether equal-precedence peers disagree for this setting.
///
/// A `true` result means the resolver must fail with ambiguity rather than let
/// incidental input order decide the effective value.
pub(super) fn conflicting_peers(
    setting: &SettingSpec,
    peers: &[&Contribution<'_>],
) -> Result<bool, ResolveError> {
    let sets: Vec<&Value> = peers
        .iter()
        .filter_map(|contribution| match contribution.op {
            Operation::Set { value, .. } => Some(value),
            _ => None,
        })
        .collect();

    if sets.len() != peers.len() {
        if !sets.is_empty() {
            return Ok(true);
        }
        return Ok(non_set_peers_conflict(peers));
    }
    if sets.len() <= 1 {
        return Ok(false);
    }

    match setting.merge {
        MergePolicy::Deep | MergePolicy::CombineByKey => {
            let mut merged = Value::Object(Map::new());
            for value in sets {
                if merge_structured_value(
                    &setting.path,
                    setting.merge,
                    &mut merged,
                    value,
                    CollisionBehavior::RejectUnequal,
                )? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        _ => Ok(sets.windows(2).any(|window| window[0] != window[1])),
    }
}

/// Returns whether equal-precedence non-set peers have incompatible effects.
///
/// The resolver admits only operations whose effects are order-independent at
/// this stage. Accumulating list policies are handled earlier and retain their
/// explicit-order requirement.
fn non_set_peers_conflict(peers: &[&Contribution<'_>]) -> bool {
    let Some(first) = peers.first() else {
        return false;
    };

    match first.op {
        Operation::Unset { .. } => peers
            .iter()
            .any(|peer| !matches!(peer.op, Operation::Unset { .. })),
        Operation::Clear { .. } => peers
            .iter()
            .any(|peer| !matches!(peer.op, Operation::Clear { .. })),
        Operation::RemoveMapEntry { .. } => peers
            .iter()
            .any(|peer| !matches!(peer.op, Operation::RemoveMapEntry { .. })),
        Operation::RemoveListItem { .. } => {
            let mut seen: Vec<&Value> = Vec::new();
            for peer in peers {
                let Operation::RemoveListItem { value, .. } = peer.op else {
                    return true;
                };
                if seen.contains(&value) {
                    // Removing one matching item is multiplicity-sensitive. Two equal
                    // peer removals could consume two equal occurrences, so callers must
                    // disambiguate rather than relying on incidental peer iteration.
                    return true;
                }
                seen.push(value);
            }
            false
        }
        Operation::Set { .. } => false,
    }
}
