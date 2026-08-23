use super::support::set_input;
use configlab::*;
use serde_json::json;

#[test]
fn structured_report_separates_blocking_errors_warnings_and_information() {
    let mut schema = Schema::new();

    let mut required = SettingSpec::typed("service.port", ValueType::UnsignedInteger);
    required.requirement = Requirement::Required;
    schema.insert(required).unwrap();

    let mut old = SettingSpec::typed("service.old_mode", ValueType::Text);
    old.deprecation = Some(Deprecation {
        message: "old_mode is deprecated".into(),
        replacement: None,
    });
    schema.insert(old).unwrap();

    let resolver = Resolver::new(schema, LayerOrder::new(["base"]).unwrap()).unwrap();
    let report = resolver
        .resolve_report(
            &Context::new(),
            &[set_input("service.old_mode", json!("legacy"))],
        )
        .unwrap();

    assert!(report.effective.is_none());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == DiagnosticSeverity::Error
            && diagnostic.code == DiagnosticCode::MissingRequired
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == DiagnosticSeverity::Warning
            && diagnostic.code == DiagnosticCode::DeprecatedSetting
    }));
    assert!(!report.information.iter().any(|information| matches!(
        information,
        ResolutionInformation::OptionalOmitted { path } if path == "service.port"
    )));
}

#[test]
fn higher_layer_replace_is_recorded_as_resolution_information() {
    let mut schema = Schema::new();
    schema
        .insert(SettingSpec::typed("port", ValueType::UnsignedInteger))
        .unwrap();
    let resolver = Resolver::new(schema, LayerOrder::new(["base", "override"]).unwrap()).unwrap();
    let base = LogicalInput::new("base", "base", Origin::new("base")).push(Operation::Set {
        path: "port".into(),
        value: json!(8080),
    });
    let higher =
        LogicalInput::new("override", "override", Origin::new("override")).push(Operation::Set {
            path: "port".into(),
            value: json!(9090),
        });

    let effective = resolver.resolve(&Context::new(), &[base, higher]).unwrap();
    assert_eq!(effective.get("port"), Some(&json!(9090)));
    assert!(effective.information.iter().any(|information| matches!(
        information,
        ResolutionInformation::ValueOverridden {
            path,
            previous_layer,
            winning_layer,
        } if path == "port" && previous_layer == "base" && winning_layer == "override"
    )));
}

#[test]
fn warning_only_report_retains_effective_configuration() {
    let mut schema = Schema::new();
    let mut old = SettingSpec::typed("old_mode", ValueType::Text);
    old.default = Some(DefaultRule::Fixed(json!("legacy")));
    old.deprecation = Some(Deprecation {
        message: "old_mode is deprecated".into(),
        replacement: None,
    });
    schema.insert(old).unwrap();

    let resolver = Resolver::new(schema, LayerOrder::new(["base"]).unwrap()).unwrap();
    let report = resolver.resolve_report(&Context::new(), &[]).unwrap();
    assert!(report.is_success());
    assert!(report.effective.is_some());
    assert_eq!(report.warnings().count(), 1);
}

#[test]
fn ignored_component_target_is_retained_in_resolution_information() {
    let mut component_schema = Schema::new();
    component_schema
        .insert(SettingSpec::typed("enabled", ValueType::Boolean))
        .unwrap();
    let component = ComponentDefinition::new("mail", "1", component_schema);

    let mut application = ApplicationDefinition::new("app", "1");
    application.compose("mail", "mail", component).unwrap();
    let resolver =
        Resolver::for_application(&application, LayerOrder::new(["contextual"]).unwrap()).unwrap();

    let input = LogicalInput::new("mail-prod", "contextual", Origin::new("test"))
        .target_component("mail")
        .when(Selector::exact([("environment", "production")]))
        .push(Operation::Set {
            path: "enabled".into(),
            value: json!(true),
        });
    let context = Context::from([("environment", "development")]);
    let report = resolver.resolve_report(&context, &[input]).unwrap();

    assert!(report.information.iter().any(|information| matches!(
        information,
        ResolutionInformation::InputIgnored {
            input_id,
            target: InputTarget::Component(key),
            ..
        } if input_id == "mail-prod" && key == "mail"
    )));
}

#[test]
fn diagnostic_and_resolver_codes_are_stable_descriptive_identifiers() {
    let diagnostic_codes = [
        (DiagnosticCode::MissingRequired, "missing_required"),
        (DiagnosticCode::InvalidType, "invalid_type"),
        (DiagnosticCode::ConstraintViolation, "constraint_violation"),
        (
            DiagnosticCode::InvalidRepresentation,
            "invalid_representation",
        ),
        (DiagnosticCode::RuleViolation, "rule_violation"),
        (DiagnosticCode::DeprecatedSetting, "deprecated_setting"),
    ];
    for (code, expected) in diagnostic_codes {
        assert_eq!(code.as_str(), expected);
    }

    let resolver_errors = [
        (
            ResolveError::InvalidLayerOrder("duplicate layer".into()),
            "invalid_layer_order",
        ),
        (ResolveError::UnknownLayer("typo".into()), "unknown_layer"),
        (
            ResolveError::UnavailableContext {
                attribute: "region".into(),
            },
            "unavailable_context",
        ),
        (
            ResolveError::UnknownTarget("worker".into()),
            "unknown_target",
        ),
        (
            ResolveError::UnknownSetting("server.port".into()),
            "unknown_setting",
        ),
        (
            ResolveError::InvalidSchema("bad definition".into()),
            "invalid_schema",
        ),
        (
            ResolveError::InvalidPath("server..port".into()),
            "invalid_path",
        ),
        (
            ResolveError::Ambiguous {
                path: "port".into(),
                layer: "contextual".into(),
                specificity: 1,
                inputs: vec!["a".into(), "b".into()],
            },
            "ambiguous_peer",
        ),
        (
            ResolveError::UndefinedOrder {
                path: "includes".into(),
                inputs: vec!["a".into(), "b".into()],
            },
            "undefined_order",
        ),
        (
            ResolveError::MissingRequired("server.port".into()),
            "missing_required",
        ),
        (
            ResolveError::InvalidValue {
                path: "server.port".into(),
                message: "expected integer".into(),
            },
            "invalid_value",
        ),
        (
            ResolveError::RemovalNotAllowed {
                path: "server.port".into(),
                operation: "unset".into(),
            },
            "removal_not_allowed",
        ),
        (
            ResolveError::LimitExceeded {
                resource: "inputs".into(),
                limit: 1,
            },
            "limit_exceeded",
        ),
    ];
    for (error, expected) in resolver_errors {
        assert_eq!(error.code(), expected);
    }
}
