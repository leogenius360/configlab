use super::support::set_input;
use configlab::*;
use serde_json::json;

#[test]
fn conditional_requirements_are_declarative_and_evaluated_after_resolution() {
    let mut schema = Schema::new();

    let mut enabled = SettingSpec::typed("tls.enabled", ValueType::Boolean);
    enabled.default = Some(DefaultRule::Fixed(json!(false)));
    schema.insert(enabled).unwrap();

    let mut certificate = SettingSpec::typed("tls.certificate", ValueType::Path);
    certificate.requirement = Requirement::RequiredWhen(Condition::boolean("tls.enabled"));
    schema.insert(certificate).unwrap();

    let mut private_key = SettingSpec::typed("tls.private_key", ValueType::Text);
    private_key.representation = Representation::SecretReference;
    private_key.disclosure = Disclosure::Sensitive;
    private_key.requirement = Requirement::RequiredWhen(Condition::boolean("tls.enabled"));
    schema.insert(private_key).unwrap();

    let resolver = Resolver::new(schema, LayerOrder::new(["base"]).unwrap()).unwrap();
    assert!(resolver.resolve(&Context::new(), &[]).is_ok());

    let enabled_input = set_input("tls.enabled", json!(true));
    let report = resolver
        .resolve_report(&Context::new(), std::slice::from_ref(&enabled_input))
        .unwrap();
    assert_eq!(
        report
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagnosticCode::MissingRequired)
            .count(),
        2
    );

    let inputs = [
        enabled_input,
        set_input("tls.certificate", json!("/etc/tls/cert.pem")),
        set_input("tls.private_key", json!("secret://orders/tls-key")),
    ];
    assert!(resolver.resolve(&Context::new(), &inputs).is_ok());
}

#[test]
fn contains_condition_supports_collection_driven_requirements() {
    let mut schema = Schema::new();

    let mut destinations = SettingSpec::typed("logging.destinations", ValueType::Text);
    destinations.shape = ValueShape::OrderedList;
    destinations.default = Some(DefaultRule::Fixed(json!(["stdout"])));
    schema.insert(destinations).unwrap();

    let mut file_path = SettingSpec::typed("logging.file_path", ValueType::Path);
    file_path.requirement = Requirement::RequiredWhen(Condition::Contains {
        path: "logging.destinations".into(),
        value: json!("file"),
    });
    schema.insert(file_path).unwrap();

    let resolver = Resolver::new(schema, LayerOrder::new(["base"]).unwrap()).unwrap();
    assert!(resolver.resolve(&Context::new(), &[]).is_ok());

    let report = resolver
        .resolve_report(
            &Context::new(),
            &[set_input("logging.destinations", json!(["stdout", "file"]))],
        )
        .unwrap();
    assert!(report.effective.is_none());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::MissingRequired
            && diagnostic.path.as_deref() == Some("logging.file_path")
    }));
}
