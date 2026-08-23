use super::support::input;
use configlab::*;
use serde_json::Value;
use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn unset_is_distinct_from_null_and_suppresses_default() {
    let mut schema = Schema::new();
    let mut port = SettingSpec::new("port");
    port.default = Some(DefaultRule::Fixed(json!(8080)));
    schema.insert(port).unwrap();
    let resolver = Resolver::new(schema, LayerOrder::new(["override"]).unwrap()).unwrap();
    let unset = LogicalInput::new("unset", "override", Origin::new("cli")).push(Operation::Unset {
        path: "port".into(),
    });
    let out = resolver.resolve(&Context::new(), &[unset]).unwrap();
    assert_eq!(out.get("port"), None);

    let null = input(
        "null",
        "override",
        Selector::any(),
        "port",
        serde_json::Value::Null,
    );
    assert!(matches!(
        resolver.resolve(&Context::new(), &[null]),
        Err(ResolveError::InvalidValue { .. })
    ));
}

#[test]
fn derived_defaults_are_lookup_based_and_cycles_are_rejected() {
    let mut schema = Schema::new();
    let mut engine = SettingSpec::new("db.engine");
    engine.default = Some(DefaultRule::Fixed(json!("postgres")));
    schema.insert(engine).unwrap();
    let mut cases = BTreeMap::new();
    cases.insert("postgres".into(), json!(5432));
    cases.insert("mysql".into(), json!(3306));
    let mut port = SettingSpec::new("db.port");
    port.default = Some(DefaultRule::Lookup {
        source: "db.engine".into(),
        cases,
    });
    schema.insert(port).unwrap();
    let resolver = Resolver::new(schema, LayerOrder::new(["base"]).unwrap()).unwrap();
    assert_eq!(
        resolver
            .resolve(&Context::new(), &[])
            .unwrap()
            .get("db.port"),
        Some(&json!(5432))
    );

    let mut cyclic = Schema::new();
    let mut a = SettingSpec::new("a");
    a.default = Some(DefaultRule::Lookup {
        source: "b".into(),
        cases: BTreeMap::new(),
    });
    cyclic.insert(a).unwrap();
    let mut b = SettingSpec::new("b");
    b.default = Some(DefaultRule::Lookup {
        source: "a".into(),
        cases: BTreeMap::new(),
    });
    cyclic.insert(b).unwrap();
    assert!(Resolver::new(cyclic, LayerOrder::new(["x"]).unwrap()).is_err());
}

#[test]
fn nullable_setting_accepts_null_but_omission_and_unset_remain_distinct() {
    let mut schema = Schema::new();
    let mut setting = SettingSpec::new("value");
    setting.nullable = true;
    setting.default = Some(DefaultRule::Fixed(json!(7)));
    schema.insert(setting).unwrap();
    let resolver = Resolver::new(schema, LayerOrder::new(["base", "override"]).unwrap()).unwrap();

    assert_eq!(
        resolver.resolve(&Context::new(), &[]).unwrap().get("value"),
        Some(&json!(7))
    );
    let null = input("null", "base", Selector::any(), "value", Value::Null);
    assert_eq!(
        resolver
            .resolve(&Context::new(), &[null])
            .unwrap()
            .get("value"),
        Some(&Value::Null)
    );
    let unset = LogicalInput::new("unset", "override", Origin::new("cli")).push(Operation::Unset {
        path: "value".into(),
    });
    assert_eq!(
        resolver
            .resolve(&Context::new(), &[unset])
            .unwrap()
            .get("value"),
        None
    );
}

#[test]
fn malformed_fixed_and_lookup_defaults_are_schema_errors() {
    let mut fixed_schema = Schema::new();
    let mut list = SettingSpec::new("items");
    list.shape = ValueShape::OrderedList;
    list.default = Some(DefaultRule::Fixed(json!("not-a-list")));
    fixed_schema.insert(list).unwrap();
    assert!(matches!(
        Resolver::new(fixed_schema, LayerOrder::new(["base"]).unwrap()),
        Err(ResolveError::InvalidSchema(_))
    ));

    let mut lookup_schema = Schema::new();
    let mut engine = SettingSpec::new("engine");
    engine.default = Some(DefaultRule::Fixed(json!("postgres")));
    lookup_schema.insert(engine).unwrap();
    let mut cases = BTreeMap::new();
    cases.insert("postgres".to_string(), json!("not-a-list"));
    let mut items = SettingSpec::new("items");
    items.shape = ValueShape::OrderedList;
    items.default = Some(DefaultRule::Lookup {
        source: "engine".into(),
        cases,
    });
    lookup_schema.insert(items).unwrap();
    assert!(matches!(
        Resolver::new(lookup_schema, LayerOrder::new(["base"]).unwrap()),
        Err(ResolveError::InvalidSchema(_))
    ));
}

#[test]
fn item_removal_applies_to_definition_default_collections() {
    let mut map_spec = SettingSpec::new("labels");
    map_spec.shape = ValueShape::NamedMap;
    map_spec.merge = MergePolicy::CombineByKey;
    map_spec.removal = RemovalPolicy::unset_only().with_map_entry_removal();
    map_spec.default = Some(DefaultRule::Fixed(json!({"keep":"yes","drop":"no"})));
    let mut list_spec = SettingSpec::new("paths");
    list_spec.shape = ValueShape::OrderedList;
    list_spec.merge = MergePolicy::Append;
    list_spec.removal = RemovalPolicy::unset_only().with_list_item_removal();
    list_spec.default = Some(DefaultRule::Fixed(json!(["a", "b"])));
    let mut schema = Schema::new();
    schema.insert(map_spec).unwrap();
    schema.insert(list_spec).unwrap();
    let resolver = Resolver::new(schema, LayerOrder::conventional()).unwrap();
    let input = LogicalInput::new("remove-default-items", "override", Origin::new("test"))
        .push(Operation::RemoveMapEntry {
            path: "labels".into(),
            key: "drop".into(),
        })
        .push(Operation::RemoveListItem {
            path: "paths".into(),
            value: json!("a"),
        });
    let result = resolver.resolve(&Context::new(), &[input]).unwrap();
    assert_eq!(result.get("labels"), Some(&json!({"keep":"yes"})));
    assert_eq!(result.get("paths"), Some(&json!(["b"])));
}
