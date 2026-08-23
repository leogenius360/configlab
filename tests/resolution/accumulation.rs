use super::support::{input, schema};
use configlab::*;
use serde_json::json;

#[test]
fn append_requires_explicit_order_for_equal_specificity_peers() {
    let resolver = Resolver::new(schema(), LayerOrder::new(["project"]).unwrap()).unwrap();
    let a = input(
        "a",
        "project",
        Selector::any(),
        "compiler.includes",
        json!(["a"]),
    );
    let b = input(
        "b",
        "project",
        Selector::any(),
        "compiler.includes",
        json!(["b"]),
    );
    assert!(matches!(
        resolver.resolve(&Context::new(), &[a, b]),
        Err(ResolveError::UndefinedOrder { .. })
    ));
}

#[test]
fn append_with_explicit_order_accumulates_and_preserves_provenance() {
    let resolver = Resolver::new(schema(), LayerOrder::new(["project"]).unwrap()).unwrap();
    let a = input(
        "a",
        "project",
        Selector::any(),
        "compiler.includes",
        json!(["a"]),
    )
    .ordered(1);
    let b = input(
        "b",
        "project",
        Selector::any(),
        "compiler.includes",
        json!(["b"]),
    )
    .ordered(2);
    let pw = input(
        "pw",
        "project",
        Selector::any(),
        "database.password",
        json!("secret://db"),
    );
    let timeout = input(
        "timeout",
        "project",
        Selector::any(),
        "server.timeout",
        json!(30),
    );
    let out = resolver
        .resolve(&Context::new(), &[a, b, pw, timeout])
        .unwrap();
    assert_eq!(out.get("compiler.includes"), Some(&json!(["a", "b"])));
    assert_eq!(out.explain("compiler.includes").len(), 2);
    assert_eq!(out.explain("compiler.includes")[0].item, Some(0));
    assert_eq!(out.explain("compiler.includes")[1].item, Some(1));
}

#[test]
fn prepend_and_remove_keep_current_item_provenance_indices_correct() {
    let mut schema = Schema::new();
    let mut list = SettingSpec::new("items");
    list.shape = ValueShape::OrderedList;
    list.merge = MergePolicy::Prepend;
    schema.insert(list).unwrap();
    let resolver = Resolver::new(schema, LayerOrder::new(["base", "override"]).unwrap()).unwrap();
    let base = input("base", "base", Selector::any(), "items", json!(["b", "c"]));
    let prepend = input(
        "prepend",
        "override",
        Selector::any(),
        "items",
        json!(["a"]),
    );
    let out = resolver.resolve(&Context::new(), &[base, prepend]).unwrap();
    assert_eq!(out.get("items"), Some(&json!(["a", "b", "c"])));
    let trace = out.explain("items");
    assert!(
        trace
            .iter()
            .any(|entry| entry.value == Some(json!("a")) && entry.item == Some(0))
    );
    assert!(
        trace
            .iter()
            .any(|entry| entry.value == Some(json!("b")) && entry.item == Some(1))
    );
    assert!(
        trace
            .iter()
            .any(|entry| entry.value == Some(json!("c")) && entry.item == Some(2))
    );
}

#[test]
fn append_uses_only_maximum_specificity_within_a_layer_before_ordering() {
    let mut schema = Schema::new();
    let mut list = SettingSpec::new("compiler.includes");
    list.shape = ValueShape::OrderedList;
    list.merge = MergePolicy::Append;
    schema.insert(list).unwrap();
    let resolver = Resolver::new(schema, LayerOrder::new(["project"]).unwrap()).unwrap();
    let generic_a = input(
        "generic-a",
        "project",
        Selector::any(),
        "compiler.includes",
        json!(["generic-a"]),
    );
    let generic_b = input(
        "generic-b",
        "project",
        Selector::any(),
        "compiler.includes",
        json!(["generic-b"]),
    );
    let specific = input(
        "specific",
        "project",
        Selector::exact([("region", "west")]),
        "compiler.includes",
        json!(["specific"]),
    );
    let context = Context::from([("region", "west")]);

    let first = resolver
        .resolve(
            &context,
            &[generic_a.clone(), specific.clone(), generic_b.clone()],
        )
        .unwrap();
    let second = resolver
        .resolve(&context, &[generic_b, specific, generic_a])
        .unwrap();

    assert_eq!(first.get("compiler.includes"), Some(&json!(["specific"])));
    assert_eq!(second.get("compiler.includes"), Some(&json!(["specific"])));
    assert_eq!(first.values, second.values);
    assert_eq!(
        first
            .explain("compiler.includes")
            .iter()
            .filter(|entry| entry.action == ProvenanceAction::Shadowed)
            .count(),
        2
    );
}

#[test]
fn prepend_uses_only_maximum_specificity_within_a_layer() {
    let mut schema = Schema::new();
    let mut list = SettingSpec::new("items");
    list.shape = ValueShape::OrderedList;
    list.merge = MergePolicy::Prepend;
    schema.insert(list).unwrap();
    let resolver = Resolver::new(schema, LayerOrder::new(["contextual"]).unwrap()).unwrap();
    let generic = input(
        "generic",
        "contextual",
        Selector::any(),
        "items",
        json!(["generic"]),
    );
    let specific = input(
        "specific",
        "contextual",
        Selector::exact([("environment", "production")]),
        "items",
        json!(["specific"]),
    );

    let effective = resolver
        .resolve(
            &Context::from([("environment", "production")]),
            &[generic, specific],
        )
        .unwrap();
    assert_eq!(effective.get("items"), Some(&json!(["specific"])));
    assert!(
        effective
            .explain("items")
            .iter()
            .any(|entry| entry.action == ProvenanceAction::Shadowed)
    );
}
