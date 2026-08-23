//! Source-neutral logical inputs and operations.

use super::Selector;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Explicit value or removal operation contributed by one logical input.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Operation {
    /// Supplies or combines a value according to the setting's merge policy.
    Set { path: String, value: Value },
    /// Removes the whole setting when permitted by its removal policy.
    Unset { path: String },
    /// Retains the setting as an empty collection.
    Clear { path: String },
    /// Removes one named-map entry.
    RemoveMapEntry { path: String, key: String },
    /// Removes one matching item from an ordered list.
    ///
    /// The operation removes one occurrence. Equal-precedence duplicate peer
    /// removals of the same value are therefore ambiguous when multiplicity
    /// could be observable.
    RemoveListItem { path: String, value: Value },
}

impl Operation {
    /// Returns the setting path targeted by this operation.
    pub fn path(&self) -> &str {
        match self {
            Self::Set { path, .. }
            | Self::Unset { path }
            | Self::Clear { path }
            | Self::RemoveMapEntry { path, .. }
            | Self::RemoveListItem { path, .. } => path,
        }
    }
}

/// Physical/logical origin retained independently of resolution semantics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Origin {
    /// Human-readable source identity, such as a file path or environment.
    pub source: String,
    /// Optional section/location/include-chain detail.
    pub detail: Option<String>,
}

impl Origin {
    /// Creates an origin without additional location detail.
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            detail: None,
        }
    }
}

/// Definition scope targeted by a logical input.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum InputTarget {
    /// Operation paths are absolute in the resolver schema.
    Root,
    /// Operation paths are relative to one application-local component root.
    Component(String),
}

/// Source-neutral normalized input consumed by the resolver.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogicalInput {
    /// Stable identity used in provenance and ambiguity diagnostics.
    pub id: String,
    /// Explicit precedence layer.
    pub layer: String,
    /// Application/component scope for operation paths.
    pub target: InputTarget,
    /// Exact context selector.
    pub selector: Selector,
    /// Explicit peer order required when accumulation order is semantically relevant.
    pub order: Option<u32>,
    /// Physical/logical origin chain.
    pub origin: Origin,
    /// Ordered operations contributed by this input.
    pub operations: Vec<Operation>,
}

impl LogicalInput {
    /// Creates an unconditional logical input with no operations.
    pub fn new(id: impl Into<String>, layer: impl Into<String>, origin: Origin) -> Self {
        Self {
            id: id.into(),
            layer: layer.into(),
            target: InputTarget::Root,
            selector: Selector::any(),
            order: None,
            origin,
            operations: Vec::new(),
        }
    }

    /// Targets operation paths at one composed component.
    ///
    /// Component-targeted paths are relative to the component's application
    /// root and therefore cannot escape into another component namespace.
    #[must_use]
    pub fn target_component(mut self, key: impl Into<String>) -> Self {
        self.target = InputTarget::Component(key.into());
        self
    }

    /// Applies an exact context selector.
    #[must_use]
    pub fn when(mut self, selector: Selector) -> Self {
        self.selector = selector;
        self
    }

    /// Declares deterministic order among equal-layer/equal-specificity peers.
    #[must_use]
    pub fn ordered(mut self, order: u32) -> Self {
        self.order = Some(order);
        self
    }

    /// Appends one operation.
    #[must_use]
    pub fn push(mut self, operation: Operation) -> Self {
        self.operations.push(operation);
        self
    }
}
