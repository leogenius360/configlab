//! Explicit named precedence ordering.

use super::ResolveError;
use std::collections::BTreeMap;

/// Explicit application-declared precedence order.
///
/// Source mechanism never contributes precedence. A file, environment value,
/// or invocation override wins only because the normalized logical input was
/// assigned to a higher-ranked layer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayerOrder {
    names: Vec<String>,
    rank: BTreeMap<String, usize>,
}

impl LayerOrder {
    /// Creates and validates an ordered list of unique layer names.
    pub fn new<I, S>(names: I) -> Result<Self, ResolveError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let names: Vec<String> = names.into_iter().map(Into::into).collect();
        if names.is_empty() {
            return Err(ResolveError::InvalidLayerOrder(
                "at least one layer is required".into(),
            ));
        }
        let mut rank = BTreeMap::new();
        for (index, name) in names.iter().enumerate() {
            if name.is_empty() {
                return Err(ResolveError::InvalidLayerOrder(
                    "layer names cannot be empty".into(),
                ));
            }
            if rank.insert(name.clone(), index).is_some() {
                return Err(ResolveError::InvalidLayerOrder(format!(
                    "duplicate layer `{name}`"
                )));
            }
        }
        Ok(Self { names, rank })
    }

    /// Conventional local-delivery order used by the ergonomic facade.
    ///
    /// Applications may replace this order; it is convenience data, not a
    /// source-type rule in the resolver.
    pub fn conventional() -> Self {
        Self::new([
            "defaults",
            "base",
            "contextual",
            "local",
            "environment",
            "invocation",
            "override",
        ])
        .expect("valid built-in layer order")
    }

    /// Returns the numeric rank of a declared layer.
    pub fn rank(&self, name: &str) -> Result<usize, ResolveError> {
        self.rank
            .get(name)
            .copied()
            .ok_or_else(|| ResolveError::UnknownLayer(name.to_string()))
    }

    /// Returns layer names from lowest to highest precedence.
    pub fn names(&self) -> &[String] {
        &self.names
    }
}
