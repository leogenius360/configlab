//! Resolution request, stable context, and exact selectors.

use super::ResolveError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Raw facts available before selector context and logical inputs are stable.
///
/// A resolution request is deliberately not configuration and is deliberately
/// not selector context. Discovery may promote selected facts into [`Context`]
/// before ordinary selector evaluation begins.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResolutionRequest(BTreeMap<String, String>);

impl ResolutionRequest {
    /// Creates an empty request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a request from arbitrary named facts.
    pub fn from_facts<I, K, V>(facts: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        Self(
            facts
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        )
    }

    /// Inserts or replaces one request fact.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.0.insert(key.into(), value.into());
    }

    /// Returns one request fact.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(String::as_str)
    }

    /// Iterates request facts in deterministic key order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }
}

/// Stable selector context for one resolution.
///
/// Every attribute referenced by a selector must be established before
/// [`crate::Resolver`] evaluates logical inputs. Configuration values being resolved
/// cannot decide whether their own input applies.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Context(BTreeMap<String, String>);

impl Context {
    /// Creates an empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts or replaces one stable context attribute.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.0.insert(key.into(), value.into());
    }

    /// Returns one context attribute.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(String::as_str)
    }

    /// Iterates context attributes in deterministic key order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }
}

impl<K, V, const N: usize> From<[(K, V); N]> for Context
where
    K: Into<String>,
    V: Into<String>,
{
    fn from(value: [(K, V); N]) -> Self {
        let mut context = Self::new();
        for (key, value) in value {
            context.insert(key, value);
        }
        context
    }
}

/// Exact-match predicate deciding whether a logical input applies.
///
/// Specificity is the number of required context attributes. It is only a
/// tie-breaker among inputs in the same layer; it never outranks a higher
/// layer.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Selector(BTreeMap<String, String>);

impl Selector {
    /// Matches every stable context.
    pub fn any() -> Self {
        Self::default()
    }

    /// Creates an exact selector from attribute/value pairs.
    pub fn exact<I, K, V>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        Self(
            pairs
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        )
    }

    /// Returns the selector's within-layer specificity.
    pub fn specificity(&self) -> usize {
        self.0.len()
    }

    /// Iterates selector requirements in deterministic key order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }

    /// Evaluates this selector against stable context.
    ///
    /// Referencing an unavailable context attribute is an error rather than a
    /// non-match, preventing circular self-selection.
    pub fn matches(&self, context: &Context) -> Result<bool, ResolveError> {
        for key in self.0.keys() {
            if context.get(key).is_none() {
                return Err(ResolveError::UnavailableContext {
                    attribute: key.clone(),
                });
            }
        }
        Ok(self
            .0
            .iter()
            .all(|(key, expected)| context.get(key) == Some(expected.as_str())))
    }
}
