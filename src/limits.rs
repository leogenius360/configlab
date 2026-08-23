//! Resource limits applied before and during configuration resolution.

/// Bounded-work policy for physical sources and logical resolution.
///
/// Defaults are intentionally generous for application configuration while
/// preventing accidental unbounded allocation or recursion. Applications with
/// larger generated configurations can supply reviewed limits through
/// [`crate::ConfigBuilder::limits`] or [`crate::Resolver::with_limits`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolutionLimits {
    /// Maximum bytes in one file or inline structured document.
    pub max_document_bytes: usize,
    /// Maximum bytes in one physical or logical scalar/metadata string.
    pub max_scalar_bytes: usize,
    /// Maximum aggregate bytes in logical-input metadata, keys, and strings.
    pub max_logical_bytes: usize,
    /// Maximum arguments examined by one argument source.
    pub max_arguments: usize,
    /// Maximum logical inputs in one resolution.
    pub max_inputs: usize,
    /// Maximum operations across all logical inputs.
    pub max_operations: usize,
    /// Maximum nesting depth of one contributed JSON value.
    pub max_value_depth: usize,
    /// Maximum nodes across all contributed JSON values.
    pub max_value_nodes: usize,
}

impl Default for ResolutionLimits {
    fn default() -> Self {
        Self {
            max_document_bytes: 16 * 1024 * 1024,
            max_scalar_bytes: 1024 * 1024,
            max_logical_bytes: 64 * 1024 * 1024,
            max_arguments: 4_096,
            max_inputs: 4_096,
            max_operations: 100_000,
            max_value_depth: 128,
            max_value_nodes: 1_000_000,
        }
    }
}

impl ResolutionLimits {
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        if self.max_document_bytes == 0
            || self.max_scalar_bytes == 0
            || self.max_logical_bytes == 0
            || self.max_arguments == 0
            || self.max_inputs == 0
            || self.max_operations == 0
            || self.max_value_depth == 0
            || self.max_value_nodes == 0
        {
            Err("all resolution limits must be greater than zero")
        } else {
            Ok(())
        }
    }
}
