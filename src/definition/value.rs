//! Value shape, semantic type, representation, disclosure, and removal policy.

use serde::{Deserialize, Serialize};

/// Structural shape of a setting value.
///
/// Shape controls how values are structurally combined.  It is intentionally
/// separate from [`ValueType`], which describes the semantic type of scalar
/// values or collection elements.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ValueShape {
    /// A single non-collection value.
    Scalar,
    /// A structured object whose fields may be recursively combined.
    Object,
    /// An ordered sequence.
    OrderedList,
    /// A string-keyed map whose entries may be combined by key.
    NamedMap,
    /// Escape hatch for application-defined JSON values.
    Any,
}

/// Cross-input combination behavior for a setting.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MergePolicy {
    /// A higher-precedence applicable value replaces the lower value.
    Replace,
    /// Objects recursively combine by field.
    Deep,
    /// Lists accumulate lower-precedence items before higher-precedence items.
    Append,
    /// Lists accumulate higher-precedence items before lower-precedence items.
    Prepend,
    /// Named maps combine at the direct key boundary.
    ///
    /// Disjoint keys coexist. A higher-precedence value for an existing key
    /// replaces that entry as one unit; nested objects inside the entry are
    /// not recursively deep-merged.
    CombineByKey,
}

/// Semantic type of a scalar value or collection element.
///
/// `ValueType` does not encode collection shape.  For example, a
/// `Vec<String>` is represented by `ValueShape::OrderedList` together with
/// `ValueType::Text`.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum ValueType {
    /// Application-defined value whose JSON representation is not constrained
    /// by the kernel.
    #[default]
    Any,
    /// Boolean value.
    Boolean,
    /// Signed integral value.
    Integer,
    /// Unsigned integral value.
    UnsignedInteger,
    /// Floating-point or integral numeric value.
    Float,
    /// UTF-8 text.
    Text,
    /// Normalized duration represented as text at the kernel boundary.
    Duration,
    /// Normalized byte-size value represented as text at the kernel boundary.
    ByteSize,
    /// Filesystem path represented as text.
    Path,
    /// Network address or endpoint represented as text.
    Address,
    /// Text constrained to one of the declared variants.
    Enumeration(Vec<String>),
}

/// Representation of a configuration value, independent of disclosure.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Representation {
    /// Ordinary configuration data.
    Ordinary,
    /// Reference to externally managed non-secret data.
    ExternalReference,
    /// Reference to protected secret material.  The referenced payload is not
    /// part of the effective configuration.
    SecretReference,
}

/// Whether a value may appear in ordinary diagnostics and support output.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Disclosure {
    /// The value may be shown normally.
    Public,
    /// The value is structurally redacted from public diagnostic output.
    Sensitive,
}

/// Removal operations a setting definition permits.
///
/// Whole-setting unset is enabled by default for ergonomic compatibility.
/// Collection-specific mutation must be opted into by the definition.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RemovalPolicy {
    /// Permit removing the entire setting.
    pub unset: bool,
    /// Permit clearing a collection while retaining an empty collection.
    pub clear: bool,
    /// Permit removing one named-map entry.
    pub remove_map_entry: bool,
    /// Permit removing one matching item from an ordered collection.
    pub remove_list_item: bool,
}

impl RemovalPolicy {
    /// No removal operations are allowed.
    pub const fn none() -> Self {
        Self {
            unset: false,
            clear: false,
            remove_map_entry: false,
            remove_list_item: false,
        }
    }

    /// Only whole-setting unset is allowed.
    pub const fn unset_only() -> Self {
        Self {
            unset: true,
            ..Self::none()
        }
    }

    /// Enables collection clear.
    #[must_use]
    pub const fn with_clear(mut self) -> Self {
        self.clear = true;
        self
    }

    /// Enables removal of named-map entries.
    #[must_use]
    pub const fn with_map_entry_removal(mut self) -> Self {
        self.remove_map_entry = true;
        self
    }

    /// Enables removal of matching ordered-list items.
    #[must_use]
    pub const fn with_list_item_removal(mut self) -> Self {
        self.remove_list_item = true;
        self
    }
}

impl Default for RemovalPolicy {
    fn default() -> Self {
        Self::unset_only()
    }
}
