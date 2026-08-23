use configlab::*;
use serde_json::json;

#[test]
fn operational_verification_is_a_separate_structured_report() {
    let mut schema = Schema::new();
    let mut endpoint = SettingSpec::typed("database.endpoint", ValueType::Address);
    endpoint.default = Some(DefaultRule::Fixed(json!("db.internal:5432")));
    schema.insert(endpoint).unwrap();
    let resolver = Resolver::new(schema, LayerOrder::new(["base"]).unwrap()).unwrap();
    let resolution = resolver.resolve_report(&Context::new(), &[]).unwrap();

    let verification = VerificationReport::new().push(VerificationCheck {
        name: "database-connectivity".into(),
        subjects: vec!["database.endpoint".into()],
        result: VerificationResult::Failed,
        message: Some("connection refused".into()),
    });

    assert!(resolution.is_success());
    assert_eq!(verification.checks.len(), 1);
    assert_eq!(verification.checks[0].result, VerificationResult::Failed);
    assert!(resolution.diagnostics.is_empty());
}
