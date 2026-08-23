//! Metadata shared by all physical source descriptions.

use crate::Selector;

#[derive(Clone, Debug)]
pub(super) struct SourceOptions {
    pub(super) layer: String,
    pub(super) selector: Selector,
    pub(super) order: Option<u32>,
}

impl SourceOptions {
    pub(super) fn new(layer: &str) -> Self {
        Self {
            layer: layer.into(),
            selector: Selector::any(),
            order: None,
        }
    }
}
