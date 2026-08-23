use super::support::set_input;
use configlab::*;
use serde_json::json;

#[test]
fn semantic_value_types_are_validated_by_the_kernel() {
    let mut schema = Schema::new();

    let mut port = SettingSpec::typed("server.port", ValueType::UnsignedInteger);
    port.requirement = Requirement::Required;
    schema.insert(port).unwrap();

    let mut modes = SettingSpec::typed(
        "server.modes",
        ValueType::Enumeration(vec!["http".into(), "https".into()]),
    );
    modes.shape = ValueShape::OrderedList;
    schema.insert(modes).unwrap();

    let resolver = Resolver::new(schema, LayerOrder::new(["base"]).unwrap()).unwrap();
    let inputs = [
        set_input("server.port", json!("8080")),
        set_input("server.modes", json!(["http", "invalid"])),
    ];

    let report = resolver.resolve_report(&Context::new(), &inputs).unwrap();
    assert!(!report.is_success());
    assert!(report.effective.is_none());
    assert_eq!(
        report
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagnosticCode::InvalidType)
            .count(),
        2
    );
}

#[test]
fn full_width_integer_types_round_trip_through_the_kernel() {
    let mut schema = Schema::new();
    let mut signed = SettingSpec::typed("signed", ValueType::Integer);
    signed.requirement = Requirement::Required;
    schema.insert(signed).unwrap();
    let mut unsigned = SettingSpec::typed("unsigned", ValueType::UnsignedInteger);
    unsigned.requirement = Requirement::Required;
    schema.insert(unsigned).unwrap();

    let input = LogicalInput::new("wide", "base", Origin::new("test"))
        .push(Operation::Set {
            path: "signed".into(),
            value: serde_json::to_value(i128::MIN).unwrap(),
        })
        .push(Operation::Set {
            path: "unsigned".into(),
            value: serde_json::to_value(u128::MAX).unwrap(),
        });
    let resolver = Resolver::new(schema, LayerOrder::new(["base"]).unwrap()).unwrap();
    let effective = resolver.resolve(&Context::new(), &[input]).unwrap();

    assert_eq!(
        effective
            .get("signed")
            .and_then(serde_json::Value::as_number)
            .and_then(serde_json::Number::as_i128),
        Some(i128::MIN)
    );
    assert_eq!(
        effective
            .get("unsigned")
            .and_then(serde_json::Value::as_number)
            .and_then(serde_json::Number::as_u128),
        Some(u128::MAX)
    );
}
