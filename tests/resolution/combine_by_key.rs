use super::support::input;
use configlab::*;
use serde_json::json;

fn resolver() -> Resolver {
    let mut schema = Schema::new();
    let mut outputs = SettingSpec::new("logging.outputs");
    outputs.shape = ValueShape::NamedMap;
    outputs.merge = MergePolicy::CombineByKey;
    schema.insert(outputs).unwrap();
    Resolver::new(
        schema,
        LayerOrder::new(["base", "contextual", "override"]).unwrap(),
    )
    .unwrap()
}

#[test]
fn combine_by_key_combines_disjoint_entries() {
    let resolver = resolver();
    let left = input(
        "left",
        "contextual",
        Selector::any(),
        "logging.outputs",
        json!({"console": {"enabled": true}}),
    );
    let right = input(
        "right",
        "contextual",
        Selector::any(),
        "logging.outputs",
        json!({"audit": {"enabled": true}}),
    );

    let out = resolver.resolve(&Context::new(), &[left, right]).unwrap();
    assert_eq!(
        out.get("logging.outputs"),
        Some(&json!({
            "console": {"enabled": true},
            "audit": {"enabled": true}
        }))
    );
}

#[test]
fn combine_by_key_allows_identical_equal_peer_entries() {
    let resolver = resolver();
    let a = input(
        "a",
        "contextual",
        Selector::any(),
        "logging.outputs",
        json!({"console": {"enabled": true}}),
    );
    let b = input(
        "b",
        "contextual",
        Selector::any(),
        "logging.outputs",
        json!({"console": {"enabled": true}}),
    );

    let out = resolver.resolve(&Context::new(), &[a, b]).unwrap();
    assert_eq!(
        out.get("logging.outputs.console"),
        Some(&json!({"enabled": true}))
    );
}

#[test]
fn combine_by_key_rejects_different_equal_peer_values_for_the_same_entry() {
    let resolver = resolver();
    let a = input(
        "a",
        "contextual",
        Selector::any(),
        "logging.outputs",
        json!({"console": {"enabled": true}}),
    );
    let b = input(
        "b",
        "contextual",
        Selector::any(),
        "logging.outputs",
        json!({"console": {"enabled": false}}),
    );

    assert!(matches!(
        resolver.resolve(&Context::new(), &[a, b]),
        Err(ResolveError::Ambiguous { .. })
    ));
}

#[test]
fn combine_by_key_higher_layer_replaces_an_entry_as_a_unit() {
    let resolver = resolver();
    let base = input(
        "base",
        "base",
        Selector::any(),
        "logging.outputs",
        json!({
            "console": {
                "enabled": true,
                "format": "text"
            },
            "audit": {"enabled": true}
        }),
    );
    let contextual = input(
        "contextual",
        "contextual",
        Selector::any(),
        "logging.outputs",
        json!({"console": {"format": "json"}}),
    );

    let out = resolver
        .resolve(&Context::new(), &[base, contextual])
        .unwrap();
    assert_eq!(
        out.get("logging.outputs"),
        Some(&json!({
            "console": {"format": "json"},
            "audit": {"enabled": true}
        }))
    );
    assert_eq!(out.get("logging.outputs.console.enabled"), None);
}

#[test]
fn combine_by_key_nested_peer_agreement_is_input_order_independent() {
    let resolver = resolver();
    let base = input(
        "base",
        "base",
        Selector::any(),
        "logging.outputs",
        json!({
            "base_only": {"enabled": true},
            "console": {"enabled": false, "legacy": true}
        }),
    );
    let a = input(
        "a",
        "contextual",
        Selector::any(),
        "logging.outputs",
        json!({
            "audit": {"enabled": true},
            "console": {"enabled": true, "format": {"kind": "json"}}
        }),
    );
    let b = input(
        "b",
        "contextual",
        Selector::any(),
        "logging.outputs",
        json!({
            "console": {"enabled": true, "format": {"kind": "json"}},
            "metrics": {"enabled": true}
        }),
    );
    let expected = json!({
        "audit": {"enabled": true},
        "base_only": {"enabled": true},
        "console": {"enabled": true, "format": {"kind": "json"}},
        "metrics": {"enabled": true}
    });

    let outputs = [
        vec![base.clone(), a.clone(), b.clone()],
        vec![base.clone(), b, a],
    ]
    .map(|inputs| resolver.resolve(&Context::new(), &inputs).unwrap());
    for out in &outputs {
        assert_eq!(out.get("logging.outputs"), Some(&expected));
    }
    assert_eq!(outputs[0], outputs[1]);
}

#[test]
fn combine_by_key_rejects_nested_partial_peer_entries_in_every_input_order() {
    let resolver = resolver();
    let a = input(
        "a",
        "contextual",
        Selector::any(),
        "logging.outputs",
        json!({"console": {"enabled": true}}),
    );
    let b = input(
        "b",
        "contextual",
        Selector::any(),
        "logging.outputs",
        json!({"console": {"format": "json"}}),
    );

    for inputs in [vec![a.clone(), b.clone()], vec![b, a]] {
        assert!(matches!(
            resolver.resolve(&Context::new(), &inputs),
            Err(ResolveError::Ambiguous { inputs, .. }) if inputs == ["a", "b"]
        ));
    }
}
