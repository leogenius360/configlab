//! Resolved metadata for one setting operation after layer/selector ranking.

use super::{LogicalInput, Operation};

/// One applicable operation together with the precedence facts computed by the
/// resolver before per-setting merge semantics are applied.
#[derive(Clone, Debug)]
pub(super) struct Contribution<'a> {
    pub(super) layer_rank: usize,
    pub(super) specificity: usize,
    pub(super) input: &'a LogicalInput,
    pub(super) op: &'a Operation,
}
