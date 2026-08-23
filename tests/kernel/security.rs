use super::support::set_input;
use configlab::*;
use serde_json::json;

#[test]
fn representation_and_disclosure_are_independent_and_provenance_is_redacted() {
    let mut schema = Schema::new();

    let mut secret = SettingSpec::typed("database.password", ValueType::Text);
    secret.representation = Representation::SecretReference;
    secret.disclosure = Disclosure::Sensitive;
    secret.requirement = Requirement::Required;
    schema.insert(secret).unwrap();

    let mut docs = SettingSpec::typed("service.documentation", ValueType::Text);
    docs.representation = Representation::ExternalReference;
    docs.requirement = Requirement::Required;
    schema.insert(docs).unwrap();

    let resolver = Resolver::new(schema.clone(), LayerOrder::new(["base"]).unwrap()).unwrap();
    let inputs = [
        set_input("database.password", json!("secret://database/password")),
        set_input(
            "service.documentation",
            json!("file:///srv/docs/index.html"),
        ),
    ];
    let effective = resolver.resolve(&Context::new(), &inputs).unwrap();

    assert_eq!(
        effective.explain("database.password")[0].value,
        Some(json!("[REDACTED]"))
    );
    assert_eq!(
        effective.explain("service.documentation")[0].value,
        Some(json!("file:///srv/docs/index.html"))
    );
    assert_eq!(
        effective.redacted(&schema)["database"]["password"],
        json!("[REDACTED]")
    );
}

#[test]
fn effective_and_report_debug_are_safe_while_serde_remains_explicitly_raw() {
    let mut schema = Schema::new();
    let mut token = SettingSpec::typed("token", ValueType::Text);
    token.disclosure = Disclosure::Sensitive;
    token.requirement = Requirement::Required;
    schema.insert(token).unwrap();

    let resolver = Resolver::new(schema, LayerOrder::new(["base"]).unwrap()).unwrap();
    let input = set_input("token", json!("debug-must-not-leak"));
    let report = resolver.resolve_report(&Context::new(), &[input]).unwrap();
    let effective = report.effective.as_ref().unwrap();

    assert!(!format!("{effective:?}").contains("debug-must-not-leak"));
    assert!(!format!("{report:?}").contains("debug-must-not-leak"));
    assert!(
        serde_json::to_string(effective)
            .unwrap()
            .contains("debug-must-not-leak")
    );
    assert!(
        serde_json::to_string(&report)
            .unwrap()
            .contains("debug-must-not-leak")
    );
}

#[test]
fn sensitive_collection_provenance_does_not_expose_dynamic_keys_or_indices() {
    let mut schema = Schema::new();
    let mut credentials = SettingSpec::new("credentials");
    credentials.shape = ValueShape::NamedMap;
    credentials.merge = MergePolicy::CombineByKey;
    credentials.disclosure = Disclosure::Sensitive;
    schema.insert(credentials).unwrap();

    let resolver = Resolver::new(schema, LayerOrder::new(["base"]).unwrap()).unwrap();
    let input = LogicalInput::new("base", "base", Origin::new("test")).push(Operation::Set {
        path: "credentials".into(),
        value: json!({"customer-secret-key": "customer-secret-value"}),
    });
    let effective = resolver.resolve(&Context::new(), &[input]).unwrap();

    assert!(
        effective
            .explain("credentials.customer-secret-key")
            .is_empty()
    );
    assert_eq!(effective.explain("credentials").len(), 1);
    assert_eq!(
        effective.explain("credentials")[0].value,
        Some(json!("[REDACTED]"))
    );
    let debug = format!("{effective:?}");
    assert!(!debug.contains("customer-secret-key"));
    assert!(!debug.contains("customer-secret-value"));
}
