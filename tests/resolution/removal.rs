use super::support::input;
use configlab::*;
use serde_json::{Value, json};

fn resolver() -> Resolver {
    let mut schema = Schema::new();

    let mut map = SettingSpec::new("map");
    map.shape = ValueShape::NamedMap;
    map.merge = MergePolicy::CombineByKey;
    map.removal = RemovalPolicy::unset_only()
        .with_clear()
        .with_map_entry_removal();
    schema.insert(map).unwrap();

    let mut list = SettingSpec::new("list");
    list.shape = ValueShape::OrderedList;
    list.merge = MergePolicy::Replace;
    list.removal = RemovalPolicy::unset_only()
        .with_clear()
        .with_list_item_removal();
    schema.insert(list).unwrap();

    Resolver::new(schema, LayerOrder::new(["base", "override"]).unwrap()).unwrap()
}

#[test]
fn map_and_list_removals_are_explicit_operations() {
    let mut schema = Schema::new();
    let mut map = SettingSpec::new("logging.outputs");
    map.shape = ValueShape::NamedMap;
    map.merge = MergePolicy::CombineByKey;
    map.removal = RemovalPolicy::unset_only().with_map_entry_removal();
    schema.insert(map).unwrap();
    let mut list = SettingSpec::new("compiler.includes");
    list.shape = ValueShape::OrderedList;
    list.merge = MergePolicy::Append;
    list.removal = RemovalPolicy::unset_only().with_list_item_removal();
    schema.insert(list).unwrap();
    let resolver = Resolver::new(schema, LayerOrder::new(["base", "override"]).unwrap()).unwrap();

    let base_map = input(
        "map-base",
        "base",
        Selector::any(),
        "logging.outputs",
        json!({"console": true, "file": true}),
    );
    let base_list = input(
        "list-base",
        "base",
        Selector::any(),
        "compiler.includes",
        json!(["a", "b"]),
    );
    let remove_map = LogicalInput::new("map-remove", "override", Origin::new("cli")).push(
        Operation::RemoveMapEntry {
            path: "logging.outputs".into(),
            key: "file".into(),
        },
    );
    let remove_list = LogicalInput::new("list-remove", "override", Origin::new("cli")).push(
        Operation::RemoveListItem {
            path: "compiler.includes".into(),
            value: json!("a"),
        },
    );
    let out = resolver
        .resolve(
            &Context::new(),
            &[base_map, base_list, remove_map, remove_list],
        )
        .unwrap();
    assert_eq!(out.get("logging.outputs"), Some(&json!({"console": true})));
    assert_eq!(out.get("compiler.includes"), Some(&json!(["b"])));
    let removed = out.explain("logging.outputs.file");
    assert!(removed.iter().any(|entry| {
        entry.action == ProvenanceAction::RemoveMapEntry && entry.value == Some(json!(true))
    }));
}

#[test]
fn sensitive_map_removal_keeps_keys_and_values_out_of_diagnostics_and_dynamic_paths() {
    let mut schema = Schema::new();
    let mut map = SettingSpec::new("credentials");
    map.shape = ValueShape::NamedMap;
    map.merge = MergePolicy::CombineByKey;
    map.disclosure = Disclosure::Sensitive;
    map.removal = RemovalPolicy::unset_only().with_map_entry_removal();
    schema.insert(map).unwrap();
    let resolver = Resolver::new(schema, LayerOrder::new(["base", "override"]).unwrap()).unwrap();

    let base = input(
        "base",
        "base",
        Selector::any(),
        "credentials",
        json!({"customer-secret-key": "customer-secret-value"}),
    );
    let remove = LogicalInput::new("remove", "override", Origin::new("test")).push(
        Operation::RemoveMapEntry {
            path: "credentials".into(),
            key: "customer-secret-key".into(),
        },
    );
    let noop = LogicalInput::new("noop", "override", Origin::new("test")).push(
        Operation::RemoveMapEntry {
            path: "credentials".into(),
            key: "another-secret-key".into(),
        },
    );
    let effective = resolver
        .resolve(&Context::new(), &[base, remove, noop])
        .unwrap();

    assert!(
        effective
            .explain("credentials.customer-secret-key")
            .is_empty()
    );
    assert!(
        effective
            .explain("credentials.another-secret-key")
            .is_empty()
    );
    assert!(effective.explain("credentials").iter().all(|entry| {
        entry
            .value
            .as_ref()
            .is_none_or(|value| value == &json!("[REDACTED]"))
    }));
    assert!(effective.information.iter().any(|information| matches!(
        information,
        ResolutionInformation::RemovalNoop {
            path,
            operation: RemovalKind::MapEntry,
        } if path == "credentials"
    )));
    let rendered = format!("{effective:?}");
    assert!(!rendered.contains("customer-secret-key"));
    assert!(!rendered.contains("customer-secret-value"));
    assert!(!rendered.contains("another-secret-key"));
}

#[test]
fn identical_equal_peer_unsets_and_clears_are_compatible() {
    let resolver = resolver();
    let base = input(
        "base",
        "base",
        Selector::any(),
        "map",
        json!({"keep": true}),
    );
    let unset_a = LogicalInput::new("unset-a", "override", Origin::new("test"))
        .push(Operation::Unset { path: "map".into() });
    let unset_b = LogicalInput::new("unset-b", "override", Origin::new("test"))
        .push(Operation::Unset { path: "map".into() });
    let unset = resolver
        .resolve(&Context::new(), &[base.clone(), unset_a, unset_b])
        .unwrap();
    assert_eq!(unset.get("map"), None);

    let clear_a = LogicalInput::new("clear-a", "override", Origin::new("test"))
        .push(Operation::Clear { path: "map".into() });
    let clear_b = LogicalInput::new("clear-b", "override", Origin::new("test"))
        .push(Operation::Clear { path: "map".into() });
    let cleared = resolver
        .resolve(&Context::new(), &[base, clear_a, clear_b])
        .unwrap();
    assert_eq!(cleared.get("map"), Some(&json!({})));
}

#[test]
fn equal_peer_map_entry_removals_are_compatible_for_same_or_disjoint_keys() {
    let resolver = resolver();
    let base = input(
        "base",
        "base",
        Selector::any(),
        "map",
        json!({"a": 1, "b": 2, "c": 3}),
    );
    let remove = |id: &str, key: &str| {
        LogicalInput::new(id, "override", Origin::new("test")).push(Operation::RemoveMapEntry {
            path: "map".into(),
            key: key.into(),
        })
    };

    let disjoint = resolver
        .resolve(
            &Context::new(),
            &[base.clone(), remove("a", "a"), remove("b", "b")],
        )
        .unwrap();
    assert_eq!(disjoint.get("map"), Some(&json!({"c": 3})));

    let duplicate = resolver
        .resolve(
            &Context::new(),
            &[base, remove("a1", "a"), remove("a2", "a")],
        )
        .unwrap();
    assert_eq!(duplicate.get("map"), Some(&json!({"b": 2, "c": 3})));
}

#[test]
fn different_equal_peer_removal_kinds_conflict() {
    let resolver = resolver();
    let unset = LogicalInput::new("unset", "override", Origin::new("test"))
        .push(Operation::Unset { path: "map".into() });
    let clear = LogicalInput::new("clear", "override", Origin::new("test"))
        .push(Operation::Clear { path: "map".into() });

    assert!(matches!(
        resolver.resolve(&Context::new(), &[unset, clear]),
        Err(ResolveError::Ambiguous { .. })
    ));
}

#[test]
fn equal_peer_set_and_removal_conflict() {
    let resolver = resolver();
    let set = input("set", "override", Selector::any(), "map", json!({"a": 1}));
    let unset = LogicalInput::new("unset", "override", Origin::new("test"))
        .push(Operation::Unset { path: "map".into() });

    assert!(matches!(
        resolver.resolve(&Context::new(), &[set, unset]),
        Err(ResolveError::Ambiguous { .. })
    ));
}

#[test]
fn equal_peer_distinct_list_item_removals_commute_but_duplicates_are_ambiguous() {
    let resolver = resolver();
    let base = input(
        "base",
        "base",
        Selector::any(),
        "list",
        json!(["a", "b", "a", "c"]),
    );
    let remove = |id: &str, value: Value| {
        LogicalInput::new(id, "override", Origin::new("test")).push(Operation::RemoveListItem {
            path: "list".into(),
            value,
        })
    };

    let distinct = resolver
        .resolve(
            &Context::new(),
            &[
                base.clone(),
                remove("a", json!("a")),
                remove("b", json!("b")),
            ],
        )
        .unwrap();
    assert_eq!(distinct.get("list"), Some(&json!(["a", "c"])));

    assert!(matches!(
        resolver.resolve(
            &Context::new(),
            &[base, remove("a1", json!("a")), remove("a2", json!("a")),],
        ),
        Err(ResolveError::Ambiguous { .. })
    ));
}
