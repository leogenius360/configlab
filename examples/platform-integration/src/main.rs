#![forbid(unsafe_code)]

//! Advanced source-neutral integration without derive macros or format features.

use configlab::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn fixed(path: &str, value_type: ValueType, value: Value) -> SettingSpec {
    let mut setting = SettingSpec::typed(path, value_type);
    setting.default = Some(DefaultRule::Fixed(value));
    setting
}

fn server_component() -> Result<ComponentDefinition, ResolveError> {
    let mut schema = Schema::new();
    schema.insert(fixed(
        "request_timeout",
        ValueType::UnsignedInteger,
        json!(30),
    ))?;

    let mut listeners = fixed("listeners", ValueType::Address, json!(["0.0.0.0:8080"]));
    listeners.shape = ValueShape::OrderedList;
    listeners.merge = MergePolicy::Append;
    listeners.removal = RemovalPolicy::unset_only()
        .with_clear()
        .with_list_item_removal();
    listeners.description = Some("ordered service listener addresses".into());
    schema.insert(listeners)?;

    Ok(ComponentDefinition::new("http-server", "1", schema))
}

fn database_component() -> Result<ComponentDefinition, ResolveError> {
    let mut schema = Schema::new();

    let mut endpoint = SettingSpec::typed("endpoint", ValueType::Address);
    endpoint.requirement = Requirement::Required;
    schema.insert(endpoint)?;

    let mut credential = SettingSpec::typed("credential_ref", ValueType::Text);
    credential.requirement = Requirement::Required;
    credential.representation = Representation::SecretReference;
    credential.disclosure = Disclosure::Sensitive;
    schema.insert(credential)?;

    schema.insert(fixed(
        "connect_timeout",
        ValueType::UnsignedInteger,
        json!(5),
    ))?;
    Ok(ComponentDefinition::new("database-client", "1", schema))
}

fn notifications_component() -> Result<ComponentDefinition, ResolveError> {
    let mut schema = Schema::new();
    schema.insert(fixed("enabled", ValueType::Boolean, json!(false)))?;
    let mut provider = SettingSpec::typed("provider", ValueType::Text);
    provider.requirement = Requirement::RequiredWhen(Condition::boolean("enabled"));
    schema.insert(provider)?;
    Ok(ComponentDefinition::new("notifications", "1", schema)
        .enabled_when(Condition::boolean("enabled")))
}

fn application() -> Result<ApplicationDefinition, ResolveError> {
    let mut root = Schema::new();
    root.insert(fixed(
        "profile",
        ValueType::Enumeration(vec!["development".into(), "production".into()]),
        json!("production"),
    ))?;

    let mut workers = SettingSpec::typed("workers", ValueType::UnsignedInteger);
    workers.default = Some(DefaultRule::Lookup {
        source: "profile".into(),
        cases: BTreeMap::from([
            ("development".into(), json!(2)),
            ("production".into(), json!(12)),
        ]),
    });
    root.insert(workers)?;

    let mut labels = fixed(
        "labels",
        ValueType::Text,
        json!({"environment":"production","obsolete":"remove-me"}),
    );
    labels.shape = ValueShape::NamedMap;
    labels.merge = MergePolicy::CombineByKey;
    labels.removal = RemovalPolicy::unset_only()
        .with_clear()
        .with_map_entry_removal();
    root.insert(labels)?;

    let mut legacy_zone = SettingSpec::typed("legacy_zone", ValueType::Text);
    legacy_zone.deprecation = Some(Deprecation {
        message: "use the stable region selector instead".into(),
        replacement: None,
    });
    root.insert(legacy_zone)?;

    let mut app = ApplicationDefinition::new("payments-platform", "1").with_settings(root);
    app.compose("server", "server", server_component()?)?;
    app.compose("database", "database", database_component()?)?;
    app.compose("notifications", "notifications", notifications_component()?)?;
    app.add_rule(ValidationRule::error(
        ValidationLevel::Application,
        Condition::LessThan {
            left: "database.connect_timeout".into(),
            right: "server.request_timeout".into(),
        },
        ["database.connect_timeout", "server.request_timeout"],
        "database connection timeout must be shorter than request timeout",
    ));
    Ok(app)
}

fn resolver() -> Result<Resolver, ResolveError> {
    Resolver::for_application(
        &application()?,
        LayerOrder::new(["base", "contextual", "override"])?,
    )
}

fn database_input() -> LogicalInput {
    LogicalInput::new("database", "base", Origin::new("platform database binding"))
        .target_component("database")
        .push(Operation::Set {
            path: "endpoint".into(),
            value: json!("db.internal.example:5432"),
        })
        .push(Operation::Set {
            path: "credential_ref".into(),
            value: json!("vault://payments/database"),
        })
}

fn production_inputs() -> Vec<LogicalInput> {
    vec![
        database_input(),
        LogicalInput::new(
            "regional-listener",
            "contextual",
            Origin::new("west-africa platform policy"),
        )
        .target_component("server")
        .when(Selector::exact([("region", "west-africa")]))
        .ordered(10)
        .push(Operation::Set {
            path: "listeners".into(),
            value: json!(["10.20.0.10:8080"]),
        }),
        LogicalInput::new(
            "platform-labels",
            "contextual",
            Origin::new("deployment metadata"),
        )
        .push(Operation::Set {
            path: "labels".into(),
            value: json!({"team":"payments","region":"west-africa"}),
        }),
        LogicalInput::new("cleanup", "override", Origin::new("metadata migration"))
            .push(Operation::RemoveMapEntry {
                path: "labels".into(),
                key: "obsolete".into(),
            })
            .push(Operation::Set {
                path: "legacy_zone".into(),
                value: json!("wa-1"),
            }),
    ]
}

fn invalid_notification_input() -> LogicalInput {
    LogicalInput::new(
        "notifications-enabled",
        "override",
        Origin::new("notification controller"),
    )
    .target_component("notifications")
    .push(Operation::Set {
        path: "enabled".into(),
        value: json!(true),
    })
}

fn operational_verification(effective: &EffectiveConfiguration) -> VerificationReport {
    let endpoint_present = effective.get("database.endpoint").is_some();
    VerificationReport::new()
        .push(VerificationCheck {
            name: "database-dns".into(),
            subjects: vec!["database.endpoint".into()],
            result: if endpoint_present {
                VerificationResult::Verified
            } else {
                VerificationResult::NotPerformed
            },
            message: endpoint_present.then(|| "endpoint resolved in deployment DNS".into()),
        })
        .push(VerificationCheck {
            name: "listener-bind".into(),
            subjects: vec!["server.listeners".into()],
            result: VerificationResult::NotPerformed,
            message: Some("binding is performed by the service process".into()),
        })
}

fn run() -> Result<(EffectiveConfiguration, VerificationReport, ResolutionReport), ResolveError> {
    let resolver = resolver()?;
    let context = Context::from([("region", "west-africa")]);

    let mut invalid_inputs = production_inputs();
    invalid_inputs.push(invalid_notification_input());
    let invalid_report = resolver.resolve_report(&context, &invalid_inputs)?;

    let effective = resolver.resolve(&context, &production_inputs())?;
    let verification = operational_verification(&effective);
    Ok((effective, verification, invalid_report))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (effective, verification, invalid_report) = run()?;
    let schema = application()?.compile()?;

    println!(
        "application: {}@{}",
        effective
            .definition
            .as_ref()
            .map_or("unknown", |id| &id.name),
        effective
            .definition
            .as_ref()
            .map_or("unknown", |id| &id.version)
    );
    println!(
        "workers from derived default: {}",
        effective.get("workers").unwrap_or(&Value::Null)
    );
    println!(
        "notifications component: {:?}",
        effective.component_presence("notifications")
    );
    println!(
        "database origin: {}",
        effective
            .explain("database.endpoint")
            .last()
            .map_or("unknown", |entry| entry.origin.source.as_str())
    );
    println!(
        "safe effective configuration: {}",
        serde_json::to_string_pretty(&effective.redacted(&schema))?
    );
    println!(
        "invalid notification diagnostics: {}",
        invalid_report.diagnostics.len()
    );
    println!("operational checks: {}", verification.checks.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_neutral_platform_workflow_is_executable() {
        let (effective, verification, invalid_report) = run().unwrap();

        assert_eq!(effective.get("workers"), Some(&json!(12)));
        assert_eq!(
            effective.get("server.listeners"),
            Some(&json!(["0.0.0.0:8080", "10.20.0.10:8080"]))
        );
        assert_eq!(
            effective.get("labels"),
            Some(&json!({
                "environment":"production",
                "region":"west-africa",
                "team":"payments"
            }))
        );
        assert_eq!(
            effective.component_presence("notifications"),
            ComponentPresence::Disabled
        );
        assert!(effective.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::DeprecatedSetting
                && diagnostic.severity == DiagnosticSeverity::Warning
        }));
        assert!(invalid_report.effective.is_none());
        assert!(invalid_report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::MissingRequired
                && diagnostic.path.as_deref() == Some("notifications.provider")
        }));
        assert_eq!(
            effective
                .explain("database.endpoint")
                .last()
                .map(|entry| &entry.target),
            Some(&InputTarget::Component("database".into()))
        );
        assert_eq!(verification.checks[0].result, VerificationResult::Verified);

        let safe = effective.redacted(&application().unwrap().compile().unwrap());
        assert_eq!(safe["database"]["credential_ref"], "[REDACTED]");
    }
}
