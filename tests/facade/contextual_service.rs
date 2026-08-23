use super::support::{LogLevel, OrdersConfig, tempdir};
use configlab::*;
use serde_json::json;
use std::fs;

#[test]
fn derive_facade_runs_real_context_specific_server_scenario() {
    let dir = tempdir("orders");
    let base = dir.join("base.toml");
    let prod = dir.join("prod.toml");
    let wa = dir.join("wa.toml");
    fs::write(
        &base,
        r#"
[server]
port=8080
request_timeout=30
[database]
endpoint="db.internal"
name="orders"
pool_minimum=2
pool_maximum=20
[logging]
level="warning"
format="text"
"#,
    )
    .unwrap();
    fs::write(
        &prod,
        r#"
[server]
request_timeout=20
[logging]
level="info"
format="json"
"#,
    )
    .unwrap();
    fs::write(
        &wa,
        r#"
[server]
request_timeout=25
[database]
pool_maximum=30
"#,
    )
    .unwrap();

    let loaded = OrdersConfig::builder()
        .context("environment", "production")
        .context("region", "west-africa")
        .file(&base)
        .file(
            path(&prod)
                .layer("contextual")
                .when(selector!(environment = "production")),
        )
        .file(path(&wa).layer("contextual").when(selector!(
            environment = "production",
            region = "west-africa"
        )))
        .environment(env().from([
            ("DATABASE_USERNAME", "payments_service"),
            ("DATABASE_PASSWORD", "secret://orders/db"),
        ]))
        .arguments(args().from(["--port", "9090"]))
        .resolve()
        .unwrap();
    assert_eq!(loaded.server.port, 9090);
    assert_eq!(loaded.server.request_timeout, 25);
    assert_eq!(loaded.database.pool_maximum, 30);
    assert_eq!(loaded.logging.level, LogLevel::Info);
    assert_eq!(loaded.context().get("region"), Some("west-africa"));
    assert!(
        loaded
            .explain("server.request_timeout")
            .iter()
            .any(|p| p.specificity == 2)
    );
    assert_eq!(
        loaded.redacted_value()["database"]["password"],
        json!("[REDACTED]")
    );
    fs::remove_dir_all(dir).ok();
}

#[test]
fn facade_equal_specificity_conflict_is_error() {
    let dir = tempdir("ambiguous");
    let a = dir.join("a.toml");
    let b = dir.join("b.toml");
    fs::write(&a, "[server]\nrequest_timeout=20\n").unwrap();
    fs::write(&b, "[server]\nrequest_timeout=22\n").unwrap();
    let result = OrdersConfig::builder()
        .context("environment", "production")
        .inline(toml(
            "base",
            r#"[database]
endpoint="db"
name="orders"
username="u"
password="secret://db"
[logging]
level="info"
"#,
        ))
        .file(
            path(a)
                .layer("contextual")
                .when(selector!(environment = "production")),
        )
        .file(
            path(b)
                .layer("contextual")
                .when(selector!(environment = "production")),
        )
        .load();
    assert!(matches!(
        result,
        Err(ConfigError::Resolve(
            configlab::ResolveError::Ambiguous { .. }
        ))
    ));
    fs::remove_dir_all(dir).ok();
}

#[test]
fn application_validation_rejects_cross_component_timeout_invariant() {
    let source = r#"[server]
request_timeout=3
[database]
endpoint="db"
name="orders"
username="u"
password="secret://db"
connect_timeout=5
[logging]
level="info"
"#;
    assert!(matches!(
        OrdersConfig::builder().inline(toml("bad", source)).load(),
        Err(ConfigError::Validation(_))
    ));
}

#[test]
fn disabled_component_does_not_require_active_only_fields_but_validates_when_enabled() {
    let base = r#"[database]
endpoint="db"
name="orders"
username="u"
password="secret://db"
[logging]
level="info"
[email]
enabled=false
"#;
    assert!(
        OrdersConfig::builder()
            .inline(toml("base", base))
            .load()
            .is_ok()
    );
    let enabled = base.replace("enabled=false", "enabled=true");
    assert!(matches!(
        OrdersConfig::builder().inline(toml("enabled",enabled.clone())).load(),
        Err(ConfigError::Resolve(configlab::ResolveError::MissingRequired(path))) if path.starts_with("email.")
    ));
    let complete =
        format!("{enabled}sender=\"alerts@example.test\"\nsmtp_host=\"smtp.example.test\"\n");
    assert!(
        OrdersConfig::builder()
            .inline(toml("complete", complete))
            .load()
            .is_ok()
    );
}
