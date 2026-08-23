//! Structured-document normalization and optional-section bookkeeping.

use crate::codec::join;
use crate::{Config, ConfigError, LogicalInput, Operation, Origin, Schema, Selector, get_path};
use serde_json::Value;
use std::collections::BTreeSet;

pub(crate) fn activate_optional_groups(input: &mut LogicalInput, groups: &[String]) {
    let mut markers = BTreeSet::new();
    for operation in &input.operations {
        if matches!(operation, Operation::Set { .. }) {
            for group in groups {
                if operation.path().starts_with(&format!("{group}."))
                    && !operation.path().ends_with(".__config_present")
                {
                    markers.insert(format!("{group}.__config_present"));
                }
            }
        }
    }

    for marker in markers {
        if !input
            .operations
            .iter()
            .any(|operation| operation.path() == marker)
        {
            input.operations.insert(
                0,
                Operation::Set {
                    path: marker,
                    value: Value::Bool(true),
                },
            );
        }
    }
}

pub(super) fn document_input<T: Config>(
    id: String,
    layer: &str,
    selector: Selector,
    origin: Origin,
    order: Option<u32>,
    value: Value,
    schema: &Schema,
) -> Result<LogicalInput, ConfigError> {
    validate_document_root(&value, &origin.source)?;

    let mut input = LogicalInput::new(id, layer.to_string(), origin).when(selector);
    input.order = order;
    let groups = T::optional_groups("");
    add_optional_group_operations(&mut input, &value, schema, &groups);
    let known = add_known_setting_operations(&mut input, &value, schema, &groups);
    validate_document_paths(&value, schema, &groups, &known, &input.origin.source)?;

    Ok(input)
}

fn validate_document_root(value: &Value, source: &str) -> Result<(), ConfigError> {
    if value.is_object() {
        return Ok(());
    }
    Err(ConfigError::Parse {
        source: source.into(),
        message: "configuration document root must be an object".into(),
    })
}

fn add_optional_group_operations(
    input: &mut LogicalInput,
    value: &Value,
    schema: &Schema,
    groups: &[String],
) {
    for group in groups {
        if let Some(group_value) = get_path(value, group) {
            let marker = format!("{group}.__config_present");
            if group_value.is_null() {
                for (path, _) in schema.iter().filter(|(path, _)| is_descendant(path, group)) {
                    input.operations.push(Operation::Unset {
                        path: path.to_string(),
                    });
                }
            } else if group_value.is_object() {
                input.operations.push(Operation::Set {
                    path: marker,
                    value: Value::Bool(true),
                });
            }
        }
    }
}

fn add_known_setting_operations(
    input: &mut LogicalInput,
    value: &Value,
    schema: &Schema,
    groups: &[String],
) -> BTreeSet<String> {
    let marker_paths = groups
        .iter()
        .map(|group| format!("{group}.__config_present"))
        .collect::<BTreeSet<_>>();
    let mut known = BTreeSet::new();
    for (path, _) in schema.iter() {
        if marker_paths.contains(path) {
            continue;
        }
        if groups
            .iter()
            .any(|group| group_is_null(value, group) && is_descendant(path, group))
        {
            known.insert(path.to_string());
            continue;
        }
        if let Some(field_value) = get_path(value, path) {
            input.operations.push(Operation::Set {
                path: path.into(),
                value: field_value.clone(),
            });
            known.insert(path.to_string());
        }
    }
    known
}

fn validate_document_paths(
    value: &Value,
    schema: &Schema,
    groups: &[String],
    known: &BTreeSet<String>,
    source: &str,
) -> Result<(), ConfigError> {
    for leaf in leaf_paths(value, "") {
        if groups
            .iter()
            .any(|group| leaf == *group && group_is_null(value, group))
        {
            continue;
        }
        let known_leaf = known.iter().any(|path| paths_overlap(&leaf, path));
        let known_empty_section = empty_section_is_known(value, schema, &leaf);
        if !known_leaf && !known_empty_section {
            return Err(ConfigError::Parse {
                source: source.into(),
                message: format!("unknown configuration path `{leaf}`"),
            });
        }
    }
    Ok(())
}

fn group_is_null(value: &Value, group: &str) -> bool {
    get_path(value, group).is_some_and(Value::is_null)
}

fn paths_overlap(left: &str, right: &str) -> bool {
    left == right || is_descendant(left, right) || is_descendant(right, left)
}

fn is_descendant(path: &str, parent: &str) -> bool {
    path.strip_prefix(parent)
        .is_some_and(|suffix| suffix.starts_with('.'))
}

fn empty_section_is_known(value: &Value, schema: &Schema, path: &str) -> bool {
    get_path(value, path).is_some_and(|candidate| {
        candidate
            .as_object()
            .is_some_and(|object| object.is_empty())
    }) && schema
        .iter()
        .any(|(setting_path, _)| is_descendant(setting_path, path))
}

fn leaf_paths(value: &Value, prefix: &str) -> Vec<String> {
    match value {
        Value::Object(map) if map.is_empty() => {
            if prefix.is_empty() {
                vec![]
            } else {
                vec![prefix.into()]
            }
        }
        Value::Object(map) => map
            .iter()
            .flat_map(|(key, value)| {
                let path = join(prefix, key);
                leaf_paths(value, &path)
            })
            .collect(),
        _ => vec![prefix.into()],
    }
}

pub(crate) fn strip_internal_markers(value: &mut Value, groups: &[String]) {
    for group in groups {
        crate::remove_path(value, &format!("{group}.__config_present"));
    }
}

pub(crate) fn is_internal_path(path: &str) -> bool {
    path.split('.').any(|segment| segment == "__config_present")
}
