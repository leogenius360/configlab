use super::support::{input, schema};
use configlab::*;
use serde_json::json;

#[test]
fn redacted_view_hides_sensitive_setting() {
    let schema = schema();
    let resolver = Resolver::new(schema.clone(), LayerOrder::new(["base"]).unwrap()).unwrap();
    let inputs = vec![
        input("t", "base", Selector::any(), "server.timeout", json!(20)),
        input(
            "p",
            "base",
            Selector::any(),
            "database.password",
            json!("secret://db"),
        ),
    ];
    let out = resolver.resolve(&Context::new(), &inputs).unwrap();
    assert_eq!(
        out.redacted(&schema)["database"]["password"],
        json!("[REDACTED]")
    );
}

#[test]
fn less_specific_same_layer_candidate_is_preserved_as_shadowed_provenance() {
    let resolver =
        Resolver::new(schema(), LayerOrder::new(["base", "contextual"]).unwrap()).unwrap();
    let context = Context::from([("environment", "production"), ("region", "west-africa")]);
    let inputs = vec![
        input(
            "prod",
            "contextual",
            Selector::exact([("environment", "production")]),
            "server.timeout",
            json!(20),
        ),
        input(
            "regional",
            "contextual",
            Selector::exact([("environment", "production"), ("region", "west-africa")]),
            "server.timeout",
            json!(25),
        ),
        input(
            "pw",
            "base",
            Selector::any(),
            "database.password",
            json!("secret://db"),
        ),
    ];
    let out = resolver.resolve(&context, &inputs).unwrap();
    let trace = out.explain("server.timeout");
    assert!(
        trace
            .iter()
            .any(|entry| entry.input_id == "prod" && entry.action == ProvenanceAction::Shadowed)
    );
    assert!(
        trace
            .iter()
            .any(|entry| entry.input_id == "regional" && entry.action == ProvenanceAction::Set)
    );
}

#[test]
fn combine_by_key_preserves_entry_level_provenance() {
    let mut schema = Schema::new();
    let mut outputs = SettingSpec::new("logging.outputs");
    outputs.shape = ValueShape::NamedMap;
    outputs.merge = MergePolicy::CombineByKey;
    schema.insert(outputs).unwrap();
    let resolver = Resolver::new(schema, LayerOrder::new(["base", "contextual"]).unwrap()).unwrap();
    let base = input(
        "base",
        "base",
        Selector::any(),
        "logging.outputs",
        json!({"console": {"enabled": true}}),
    );
    let contextual = input(
        "contextual",
        "contextual",
        Selector::any(),
        "logging.outputs",
        json!({"audit": {"enabled": true}}),
    );
    let out = resolver
        .resolve(&Context::new(), &[base, contextual])
        .unwrap();
    assert_eq!(
        out.get("logging.outputs.console.enabled"),
        Some(&json!(true))
    );
    assert_eq!(out.get("logging.outputs.audit.enabled"), Some(&json!(true)));
    assert_eq!(out.explain("logging.outputs.console")[0].input_id, "base");
    assert_eq!(
        out.explain("logging.outputs.audit")[0].input_id,
        "contextual"
    );
    assert!(out.explain("logging.outputs.console.enabled").is_empty());
}

#[test]
fn dynamic_provenance_segments_escape_dots_and_backslashes() {
    let mut schema = Schema::new();
    let mut object = SettingSpec::new("payload");
    object.shape = ValueShape::Object;
    object.merge = MergePolicy::Deep;
    schema.insert(object).unwrap();
    let resolver = Resolver::new(schema, LayerOrder::new(["base"]).unwrap()).unwrap();
    let contribution = input(
        "base",
        "base",
        Selector::any(),
        "payload",
        json!({"a.b": 1, "a": {"b": 2}, "c\\d": 3}),
    );
    let out = resolver.resolve(&Context::new(), &[contribution]).unwrap();

    assert_eq!(out.explain("payload.a\\.b")[0].value, Some(json!(1)));
    assert_eq!(out.explain("payload.a.b")[0].value, Some(json!(2)));
    assert_eq!(out.explain("payload.c\\\\d")[0].value, Some(json!(3)));
    assert_eq!(provenance_child_path("payload", "a.b"), "payload.a\\.b");
}
