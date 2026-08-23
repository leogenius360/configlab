use super::support::{input, schema};
use configlab::*;
use serde_json::json;

#[test]
fn more_specific_selector_wins_inside_one_layer() {
    let resolver = Resolver::new(
        schema(),
        LayerOrder::new(["base", "contextual", "invocation"]).unwrap(),
    )
    .unwrap();
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
            "wa",
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
    let effective = resolver.resolve(&context, &inputs).unwrap();
    assert_eq!(effective.get("server.timeout"), Some(&json!(25)));
}

#[test]
fn equal_specificity_conflict_fails_loudly() {
    let resolver =
        Resolver::new(schema(), LayerOrder::new(["contextual", "base"]).unwrap()).unwrap();
    let context = Context::from([("environment", "production")]);
    let inputs = vec![
        input(
            "a",
            "contextual",
            Selector::exact([("environment", "production")]),
            "server.timeout",
            json!(20),
        ),
        input(
            "b",
            "contextual",
            Selector::exact([("environment", "production")]),
            "server.timeout",
            json!(22),
        ),
        input(
            "pw",
            "base",
            Selector::any(),
            "database.password",
            json!("secret://db"),
        ),
    ];
    assert!(matches!(
        resolver.resolve(&context, &inputs),
        Err(ResolveError::Ambiguous { .. })
    ));
}

#[test]
fn higher_layer_wins_even_when_lower_layer_is_more_specific() {
    let resolver = Resolver::new(
        schema(),
        LayerOrder::new(["contextual", "invocation"]).unwrap(),
    )
    .unwrap();
    let context = Context::from([("environment", "production"), ("region", "west-africa")]);
    let inputs = vec![
        input(
            "regional",
            "contextual",
            Selector::exact([("environment", "production"), ("region", "west-africa")]),
            "server.timeout",
            json!(25),
        ),
        input(
            "cli",
            "invocation",
            Selector::any(),
            "server.timeout",
            json!(9),
        ),
        input(
            "pw",
            "contextual",
            Selector::any(),
            "database.password",
            json!("secret://db"),
        ),
    ];
    assert_eq!(
        resolver
            .resolve(&context, &inputs)
            .unwrap()
            .get("server.timeout"),
        Some(&json!(9))
    );
}

#[test]
fn selector_using_unavailable_context_fails() {
    let resolver = Resolver::new(schema(), LayerOrder::new(["contextual"]).unwrap()).unwrap();
    let inputs = vec![input(
        "x",
        "contextual",
        Selector::exact([("profile", "release")]),
        "server.timeout",
        json!(20),
    )];
    assert!(
        matches!(resolver.resolve(&Context::new(),&inputs),Err(ResolveError::UnavailableContext{attribute}) if attribute=="profile")
    );
}

#[test]
fn source_kind_does_not_determine_precedence() {
    let resolver =
        Resolver::new(schema(), LayerOrder::new(["high-file", "low-env"]).unwrap()).unwrap();
    let inputs = vec![
        input(
            "file",
            "high-file",
            Selector::any(),
            "server.timeout",
            json!(30),
        ),
        input(
            "environment",
            "low-env",
            Selector::any(),
            "server.timeout",
            json!(10),
        ),
        input(
            "pw",
            "high-file",
            Selector::any(),
            "database.password",
            json!("secret://db"),
        ),
    ];
    assert_eq!(
        resolver
            .resolve(&Context::new(), &inputs)
            .unwrap()
            .get("server.timeout"),
        Some(&json!(10))
    );
}

#[test]
fn nonmatching_selector_is_ignored() {
    let resolver =
        Resolver::new(schema(), LayerOrder::new(["base", "contextual"]).unwrap()).unwrap();
    let context = Context::from([("environment", "development")]);
    let inputs = vec![
        input("base", "base", Selector::any(), "server.timeout", json!(30)),
        input(
            "prod",
            "contextual",
            Selector::exact([("environment", "production")]),
            "server.timeout",
            json!(20),
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
    assert_eq!(out.get("server.timeout"), Some(&json!(30)));
    assert!(out.information.iter().any(|info| matches!(
        info,
        ResolutionInformation::InputIgnored { input_id, .. } if input_id == "prod"
    )));
}
