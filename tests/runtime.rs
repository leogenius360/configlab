use configlab::*;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Config)]
struct TestConfig {
    #[config(default = 8080)]
    port: u16,
    #[config(default = "info")]
    level: String,
    #[config(default = "local-reference", sensitive)]
    token_reference: String,
}

fn temporary_file(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("configlab-{name}-{nonce}.toml"))
}

fn setting_input(id: &str, origin: &str, path: &str, value: serde_json::Value) -> LogicalInput {
    LogicalInput::new(id, "override", Origin::new(origin)).push(Operation::Set {
        path: path.into(),
        value,
    })
}

#[test]
fn reusable_plan_rereads_registered_physical_sources() {
    let file = temporary_file("plan-refresh");
    fs::write(&file, "port = 8081\n").unwrap();
    let plan = TestConfig::builder().file(&file).plan().unwrap();
    assert_eq!(plan.resolve().unwrap().port, 8081);

    fs::write(&file, "port = 9090\n").unwrap();
    assert_eq!(plan.resolve().unwrap().port, 9090);
    fs::remove_file(file).unwrap();
}

#[test]
fn plan_compilation_validates_the_schema_before_source_acquisition() {
    struct InvalidConfig;

    impl Config for InvalidConfig {
        fn schema(_prefix: &str) -> Result<Schema, ConfigError> {
            let mut schema = Schema::new();
            let mut scalar = SettingSpec::typed("value", ValueType::Text);
            scalar.merge = MergePolicy::Deep;
            schema.insert(scalar)?;
            Ok(schema)
        }

        fn decode(_effective: &EffectiveConfiguration, _prefix: &str) -> Result<Self, ConfigError> {
            unreachable!("an invalid plan must fail before decoding")
        }
    }

    assert!(matches!(
        InvalidConfig::builder().plan(),
        Err(ConfigError::Resolve(ResolveError::InvalidSchema(_)))
    ));
}

#[test]
fn live_refresh_replaces_the_current_value_and_reports_provenance() {
    let file = temporary_file("live-refresh");
    fs::write(&file, "port = 8081\nlevel = 'info'\n").unwrap();
    let mut config = TestConfig::builder().file(&file).live().unwrap();
    assert_eq!(config.port, 8081);

    fs::write(&file, "port = 9090\nlevel = 'warning'\n").unwrap();
    let report = config.refresh().unwrap();
    assert!(report.changed());
    assert_eq!(report.changes().len(), 2);
    assert_eq!(config.port, 9090);
    assert_eq!(config.level, "warning");

    let port = report
        .changes()
        .changes()
        .iter()
        .find(|change| change.path() == "port")
        .unwrap();
    assert_eq!(port.kind(), ChangeKind::Modified);
    assert_eq!(port.previous(), Some(&json!(8081)));
    assert_eq!(port.current(), Some(&json!(9090)));
    assert!(
        port.current_provenance()
            .iter()
            .any(|entry| entry.origin.source == file.display().to_string())
    );
    fs::remove_file(file).unwrap();
}

#[test]
fn failed_refresh_preserves_the_current_configuration() {
    let file = temporary_file("failed-refresh");
    fs::write(&file, "port = 8081\n").unwrap();
    let mut config = TestConfig::builder().file(&file).live().unwrap();

    fs::write(&file, "port = 'not-a-port'\n").unwrap();
    assert!(config.refresh().is_err());
    assert_eq!(config.port, 8081);

    fs::write(&file, "port = 9090\n").unwrap();
    config.refresh().unwrap();
    assert_eq!(config.port, 9090);
    fs::remove_file(file).unwrap();
}

#[test]
fn reconcile_replaces_the_complete_external_contribution() {
    let mut config = TestConfig::builder().live().unwrap();
    let first = setting_input(
        "remote-port",
        "remote api revision one",
        "port",
        json!(9090),
    );
    let report = config.reconcile([first]).unwrap();
    assert!(report.changed());
    assert_eq!(config.port, 9090);
    assert_eq!(config.external_input_count(), 1);

    let same_value_new_origin = setting_input(
        "remote-port",
        "remote api revision two",
        "port",
        json!(9090),
    );
    let report = config.reconcile([same_value_new_origin]).unwrap();
    assert_eq!(report.changes().len(), 1);
    assert_eq!(
        report.changes().changes()[0].kind(),
        ChangeKind::ProvenanceChanged
    );
    assert!(
        report.changes().changes()[0]
            .current_provenance()
            .iter()
            .any(|entry| entry.origin.source == "remote api revision two")
    );
}

#[test]
fn retained_external_inputs_participate_in_later_physical_refreshes() {
    let file = temporary_file("external-refresh");
    fs::write(&file, "level = 'info'\n").unwrap();
    let mut config = TestConfig::builder().file(&file).live().unwrap();
    config
        .reconcile([setting_input(
            "remote-port",
            "remote api",
            "port",
            json!(9090),
        )])
        .unwrap();

    fs::write(&file, "level = 'warning'\n").unwrap();
    config.refresh().unwrap();
    assert_eq!(config.port, 9090);
    assert_eq!(config.level, "warning");
    fs::remove_file(file).unwrap();
}

#[test]
fn failed_reconciliation_preserves_current_value_and_external_inputs() {
    let mut config = TestConfig::builder().live().unwrap();
    config
        .reconcile([setting_input("valid", "remote api", "port", json!(9090))])
        .unwrap();

    let invalid = setting_input("invalid", "remote api", "port", json!("bad"));
    assert!(config.reconcile([invalid]).is_err());
    assert_eq!(config.port, 9090);
    assert_eq!(config.external_input_count(), 1);
    config.refresh().unwrap();
    assert_eq!(config.port, 9090);
}

#[test]
fn clearing_external_inputs_restores_registered_sources_and_defaults() {
    let mut config = TestConfig::builder().live().unwrap();
    config
        .refresh_with([setting_input("remote", "remote api", "port", json!(9090))])
        .unwrap();
    assert_eq!(config.port, 9090);

    let report = config.clear_external().unwrap();
    assert!(report.changed());
    assert_eq!(config.port, 8080);
    assert_eq!(config.external_input_count(), 0);
}

#[test]
fn optional_file_removal_is_reconciled_on_refresh() {
    let file = temporary_file("optional-removal");
    fs::write(&file, "port = 9090\n").unwrap();
    let mut config = TestConfig::builder()
        .file(path(&file).optional())
        .live()
        .unwrap();
    assert_eq!(config.port, 9090);

    fs::remove_file(&file).unwrap();
    config.refresh().unwrap();
    assert_eq!(config.port, 8080);
}

#[test]
fn sensitive_change_values_and_provenance_are_redacted() {
    let mut config = TestConfig::builder().live().unwrap();
    let secret = "vault://production/token";
    let report = config
        .reconcile([setting_input(
            "remote-secret",
            "remote secret provider",
            "token_reference",
            json!(secret),
        )])
        .unwrap();

    let change = &report.changes().changes()[0];
    assert_eq!(config.token_reference, secret);
    assert!(change.is_sensitive());
    assert!(change.previous().is_none());
    assert!(change.current().is_none());
    assert!(
        change
            .current_provenance()
            .iter()
            .all(|entry| entry.value.as_ref() == Some(&json!("[REDACTED]")))
    );
    assert!(!serde_json::to_string(&report).unwrap().contains(secret));
    assert!(!format!("{report:?}").contains(secret));
}
