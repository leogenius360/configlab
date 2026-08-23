use configlab::*;
use serde_json::json;

#[test]
fn application_definition_tracks_identity_component_state_and_cross_component_rules() {
    let mut http_schema = Schema::new();
    let mut request_timeout = SettingSpec::typed("request_timeout", ValueType::UnsignedInteger);
    request_timeout.default = Some(DefaultRule::Fixed(json!(30)));
    http_schema.insert(request_timeout).unwrap();
    let http = ComponentDefinition::new("http-server", "1", http_schema);

    let mut database_schema = Schema::new();
    let mut connect_timeout = SettingSpec::typed("connect_timeout", ValueType::UnsignedInteger);
    connect_timeout.default = Some(DefaultRule::Fixed(json!(5)));
    database_schema.insert(connect_timeout).unwrap();
    let database = ComponentDefinition::new("database-client", "1", database_schema);

    let mut email_schema = Schema::new();
    let mut enabled = SettingSpec::typed("enabled", ValueType::Boolean);
    enabled.default = Some(DefaultRule::Fixed(json!(false)));
    email_schema.insert(enabled).unwrap();
    let mut provider = SettingSpec::typed("provider", ValueType::Text);
    provider.requirement = Requirement::RequiredWhen(Condition::boolean("enabled"));
    email_schema.insert(provider).unwrap();
    let email = ComponentDefinition::new("email-notifications", "1", email_schema)
        .enabled_when(Condition::boolean("enabled"));

    let mut application = ApplicationDefinition::new("orders-api", "1");
    application.compose("http", "http", http).unwrap();
    application
        .compose("database", "database", database)
        .unwrap();
    application.compose("email", "email", email).unwrap();
    application.add_rule(ValidationRule::error(
        ValidationLevel::Application,
        Condition::LessThan {
            left: "database.connect_timeout".into(),
            right: "http.request_timeout".into(),
        },
        ["database.connect_timeout", "http.request_timeout"],
        "database connect timeout must be shorter than HTTP request timeout",
    ));

    let resolver =
        Resolver::for_application(&application, LayerOrder::new(["base", "override"]).unwrap())
            .unwrap();
    let effective = resolver.resolve(&Context::new(), &[]).unwrap();

    assert_eq!(
        effective.definition,
        Some(DefinitionIdentity::new("orders-api", "1"))
    );
    assert_eq!(
        effective.component_presence("http"),
        ComponentPresence::Enabled
    );
    assert_eq!(
        effective.component_presence("email"),
        ComponentPresence::Disabled
    );
    assert_eq!(
        effective.component_presence("not-composed"),
        ComponentPresence::Absent
    );

    let enabled_email = LogicalInput::new("email-on", "base", Origin::new("component input"))
        .target_component("email")
        .push(Operation::Set {
            path: "enabled".into(),
            value: json!(true),
        });
    let report = resolver
        .resolve_report(&Context::new(), &[enabled_email])
        .unwrap();
    assert!(report.effective.is_none());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::MissingRequired
            && diagnostic.path.as_deref() == Some("email.provider")
    }));

    let complete_email =
        LogicalInput::new("email-complete", "override", Origin::new("component input"))
            .target_component("email")
            .push(Operation::Set {
                path: "enabled".into(),
                value: json!(true),
            })
            .push(Operation::Set {
                path: "provider".into(),
                value: json!("smtp"),
            });
    let effective = resolver
        .resolve(&Context::new(), &[complete_email])
        .unwrap();
    assert_eq!(
        effective.component_presence("email"),
        ComponentPresence::Enabled
    );
    assert_eq!(
        effective
            .explain("email.provider")
            .last()
            .map(|entry| entry.target.clone()),
        Some(InputTarget::Component("email".into()))
    );

    let invalid_timeout =
        LogicalInput::new("invalid", "override", Origin::new("test")).push(Operation::Set {
            path: "database.connect_timeout".into(),
            value: json!(40),
        });
    let report = resolver
        .resolve_report(&Context::new(), &[invalid_timeout])
        .unwrap();
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::RuleViolation
            && diagnostic.message.contains("shorter than HTTP")
    }));
}

#[test]
fn application_definition_rejects_overlapping_component_ownership_paths() {
    let component = ComponentDefinition::new("component", "1", Schema::new());
    let mut application = ApplicationDefinition::new("app", "1");
    application
        .compose("first", "service", component.clone())
        .unwrap();
    assert!(matches!(
        application.compose("second", "service.http", component),
        Err(ResolveError::InvalidSchema(_))
    ));
}

#[test]
fn component_target_rejects_unknown_application_component() {
    let schema = Schema::new();
    let resolver = Resolver::new(schema, LayerOrder::new(["base"]).unwrap()).unwrap();
    let input = LogicalInput::new("input", "base", Origin::new("test"))
        .target_component("missing")
        .push(Operation::Set {
            path: "value".into(),
            value: json!(1),
        });
    assert!(matches!(
        resolver.resolve(&Context::new(), &[input]),
        Err(ResolveError::UnknownTarget(target)) if target == "missing"
    ));
}

#[test]
fn definition_ownership_rules_are_enforced_even_for_direct_data_construction() {
    let mut component_schema = Schema::new();
    component_schema
        .insert(SettingSpec::typed("value", ValueType::UnsignedInteger))
        .unwrap();
    component_schema.add_rule(ValidationRule::error(
        ValidationLevel::Application,
        Condition::Present {
            path: "value".into(),
        },
        ["value"],
        "component cannot own this level",
    ));
    let component = ComponentDefinition::new("component", "1", component_schema);
    assert!(matches!(
        component.validate(),
        Err(ResolveError::InvalidSchema(_))
    ));

    let mut app_schema = Schema::new();
    app_schema
        .insert(SettingSpec::typed("value", ValueType::UnsignedInteger))
        .unwrap();
    let mut application = ApplicationDefinition::new("app", "1").with_settings(app_schema);
    application.rules.push(ValidationRule::error(
        ValidationLevel::Component,
        Condition::Present {
            path: "value".into(),
        },
        ["value"],
        "application rule has the wrong ownership level",
    ));
    assert!(matches!(
        application.compile(),
        Err(ResolveError::InvalidSchema(_))
    ));
}

#[test]
fn application_compile_rechecks_public_composition_data_invariants() {
    let component = ComponentDefinition::new("component", "1", Schema::new());

    let mut duplicate_keys = ApplicationDefinition::new("app", "1");
    duplicate_keys.components = vec![
        ComponentComposition {
            key: "duplicate".into(),
            path: "service_a".into(),
            definition: component.clone(),
        },
        ComponentComposition {
            key: "duplicate".into(),
            path: "service_b".into(),
            definition: component.clone(),
        },
    ];
    assert!(matches!(
        duplicate_keys.compile(),
        Err(ResolveError::InvalidSchema(_))
    ));

    let mut overlapping_paths = ApplicationDefinition::new("app", "1");
    overlapping_paths.components = vec![
        ComponentComposition {
            key: "service".into(),
            path: "service".into(),
            definition: component.clone(),
        },
        ComponentComposition {
            key: "http".into(),
            path: "service.http".into(),
            definition: component,
        },
    ];
    assert!(matches!(
        overlapping_paths.compile(),
        Err(ResolveError::InvalidSchema(_))
    ));
}
