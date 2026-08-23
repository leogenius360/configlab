use super::support::input;
use configlab::*;
use serde_json::json;

#[test]
fn deep_merge_allows_disjoint_equal_specificity_peers_and_rejects_conflicting_leaf() {
    let mut schema = Schema::new();
    let mut object = SettingSpec::new("service.metadata");
    object.shape = ValueShape::Object;
    object.merge = MergePolicy::Deep;
    schema.insert(object).unwrap();
    let resolver = Resolver::new(schema, LayerOrder::new(["contextual"]).unwrap()).unwrap();

    let a = input(
        "a",
        "contextual",
        Selector::any(),
        "service.metadata",
        json!({"left": 1}),
    );
    let b = input(
        "b",
        "contextual",
        Selector::any(),
        "service.metadata",
        json!({"right": 2}),
    );
    let out = resolver.resolve(&Context::new(), &[a, b]).unwrap();
    assert_eq!(
        out.get("service.metadata"),
        Some(&json!({"left": 1, "right": 2}))
    );

    let a = input(
        "a",
        "contextual",
        Selector::any(),
        "service.metadata",
        json!({"shared": 1}),
    );
    let b = input(
        "b",
        "contextual",
        Selector::any(),
        "service.metadata",
        json!({"shared": 2}),
    );
    assert!(matches!(
        resolver.resolve(&Context::new(), &[a, b]),
        Err(ResolveError::Ambiguous { .. })
    ));
}

#[test]
fn deep_merge_nested_peer_agreement_is_input_order_independent() {
    let mut schema = Schema::new();
    let mut object = SettingSpec::new("service.metadata");
    object.shape = ValueShape::Object;
    object.merge = MergePolicy::Deep;
    schema.insert(object).unwrap();
    let resolver = Resolver::new(schema, LayerOrder::new(["base", "contextual"]).unwrap()).unwrap();

    let base = input(
        "base",
        "base",
        Selector::any(),
        "service.metadata",
        json!({
            "base_only": true,
            "service": {
                "endpoint": "old",
                "headers": {"base": "yes"}
            }
        }),
    );
    let a = input(
        "a",
        "contextual",
        Selector::any(),
        "service.metadata",
        json!({
            "service": {
                "endpoint": "new",
                "headers": {"a": "1", "shared": "same"}
            }
        }),
    );
    let b = input(
        "b",
        "contextual",
        Selector::any(),
        "service.metadata",
        json!({
            "service": {
                "endpoint": "new",
                "headers": {"b": "2", "shared": "same"}
            }
        }),
    );
    let expected = json!({
        "base_only": true,
        "service": {
            "endpoint": "new",
            "headers": {
                "a": "1",
                "b": "2",
                "base": "yes",
                "shared": "same"
            }
        }
    });

    let outputs = [
        vec![base.clone(), a.clone(), b.clone()],
        vec![base.clone(), b, a],
    ]
    .map(|inputs| resolver.resolve(&Context::new(), &inputs).unwrap());
    for out in &outputs {
        assert_eq!(out.get("service.metadata"), Some(&expected));
    }
    assert_eq!(outputs[0], outputs[1]);
}

#[test]
fn deep_merge_rejects_nested_peer_disagreement_in_every_input_order() {
    let mut schema = Schema::new();
    let mut object = SettingSpec::new("service.metadata");
    object.shape = ValueShape::Object;
    object.merge = MergePolicy::Deep;
    schema.insert(object).unwrap();
    let resolver = Resolver::new(schema, LayerOrder::new(["contextual"]).unwrap()).unwrap();

    let a = input(
        "a",
        "contextual",
        Selector::any(),
        "service.metadata",
        json!({"outer": {"middle": {"leaf": 1}}}),
    );
    let b = input(
        "b",
        "contextual",
        Selector::any(),
        "service.metadata",
        json!({"outer": {"middle": {"leaf": 2}}}),
    );

    for inputs in [vec![a.clone(), b.clone()], vec![b, a]] {
        assert!(matches!(
            resolver.resolve(&Context::new(), &inputs),
            Err(ResolveError::Ambiguous { inputs, .. }) if inputs == ["a", "b"]
        ));
    }
}

#[test]
fn shape_and_merge_policy_are_validated_separately() {
    let mut schema = Schema::new();
    let mut invalid = SettingSpec::new("not_a_list");
    invalid.shape = ValueShape::Scalar;
    invalid.merge = MergePolicy::Append;
    schema.insert(invalid).unwrap();
    assert!(matches!(
        Resolver::new(schema, LayerOrder::new(["base"]).unwrap()),
        Err(ResolveError::InvalidSchema(_))
    ));
}
