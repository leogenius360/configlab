//! Dot-path lookup and mutation helpers used by the resolver.

use super::ResolveError;
use serde_json::{Map, Value};

pub(crate) fn segments(path: &str) -> Result<Vec<&str>, ResolveError> {
    let parts: Vec<_> = path.split('.').collect();
    if parts.is_empty() || parts.iter().any(|part| part.is_empty()) {
        Err(ResolveError::InvalidPath(path.into()))
    } else {
        Ok(parts)
    }
}

/// Reads one dot-separated path from a canonical configuration value tree.
///
/// Returns `None` when an intermediate value is not an object or the path is
/// absent. Paths are schema paths, not JSON Pointer expressions.
pub fn get_path<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = root;
    for segment in path.split('.') {
        current = current.as_object()?.get(segment)?;
    }
    Some(current)
}

pub(crate) fn get_path_mut<'a>(root: &'a mut Value, path: &str) -> Option<&'a mut Value> {
    let parts = path.split('.').collect::<Vec<_>>();
    let mut current = root;
    for segment in &parts[..parts.len().saturating_sub(1)] {
        current = current.as_object_mut()?.get_mut(*segment)?;
    }
    current.as_object_mut()?.get_mut(*parts.last()?)
}

/// Writes one dot-separated path into a canonical configuration value tree.
///
/// Missing intermediate objects are created deterministically. An invalid path
/// containing an empty segment returns [`ResolveError::InvalidPath`].
pub fn set_path(root: &mut Value, path: &str, value: Value) -> Result<(), ResolveError> {
    let parts = segments(path)?;
    let mut current = root;
    for segment in &parts[..parts.len() - 1] {
        if !current.is_object() {
            *current = Value::Object(Map::new());
        }
        current = current
            .as_object_mut()
            .expect("value was converted to an object")
            .entry((*segment).to_string())
            .or_insert_with(|| Value::Object(Map::new()));
    }
    if !current.is_object() {
        *current = Value::Object(Map::new());
    }
    current
        .as_object_mut()
        .expect("value was converted to an object")
        .insert(parts.last().expect("path is non-empty").to_string(), value);
    Ok(())
}

pub(crate) fn set_existing_path(root: &mut Value, path: &str, value: Value) {
    if let Some(slot) = get_path_mut(root, path) {
        *slot = value;
    }
}

pub(crate) fn remove_path(root: &mut Value, path: &str) {
    let parts: Vec<_> = path.split('.').collect();
    if parts.is_empty() {
        return;
    }

    let mut current = root;
    for segment in &parts[..parts.len() - 1] {
        let Some(next) = current
            .as_object_mut()
            .and_then(|map| map.get_mut(*segment))
        else {
            return;
        };
        current = next;
    }

    if let Some(map) = current.as_object_mut() {
        map.remove(*parts.last().expect("path is non-empty"));
    }
}
