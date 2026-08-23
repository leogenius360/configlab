use configlab::{Config, ConfigError, ResolutionLimits, args, env, json, path, selector};

#[derive(Debug, Config)]
struct AppConfig {
    #[config(default = 1, env = "APP_VALUE", cli = "value")]
    value: u16,
}

#[test]
fn file_description_composes_independent_modifiers() {
    let loaded = AppConfig::builder()
        .context("environment", "production")
        .inline(json("base.json", r#"{"value":2}"#))
        .file(
            path(std::env::temp_dir().join("definitely-missing-configlab-source-api.toml"))
                .layer("contextual")
                .optional()
                .when(selector!(environment = "production")),
        )
        .load()
        .unwrap();
    assert_eq!(loaded.value, 2);
}

#[test]
fn source_description_debug_output_redacts_payloads() {
    let inline = json("secret.json", r#"{"password":"do-not-print"}"#);
    let environment = env().from([("PASSWORD", "do-not-print")]);
    let arguments = args().from(["--config", "password=do-not-print"]);

    assert!(!format!("{inline:?}").contains("do-not-print"));
    assert!(!format!("{environment:?}").contains("do-not-print"));
    assert!(!format!("{arguments:?}").contains("do-not-print"));
}

#[test]
fn custom_environment_prefix_is_local_to_that_source() {
    #[derive(Debug, Config)]
    struct Prefixed {
        value: u16,
    }

    let config = Prefixed::builder()
        .environment(env().prefix("APP").from([("APP_VALUE", "9")]))
        .load()
        .unwrap();
    assert_eq!(config.value, 9);
}

#[test]
fn optional_file_cannot_hide_an_unknown_layer() {
    let missing = std::env::temp_dir().join("configlab-source-api-unknown-layer.toml");
    let error = AppConfig::builder()
        .layer_order(["defaults", "base", "override"])
        .unwrap()
        .file(path(missing).layer("typo").optional())
        .load()
        .expect_err("unknown layer must fail before optional file materialization");

    assert!(matches!(
        error,
        ConfigError::Resolve(configlab::ResolveError::UnknownLayer(layer)) if layer == "typo"
    ));
}

#[test]
fn deterministic_argument_descriptor_overrides_environment_by_explicit_layer() {
    let loaded = AppConfig::builder()
        .layer_order(["defaults", "environment-low", "invocation-high", "override"])
        .unwrap()
        .environment(env().layer("environment-low").from([("APP_VALUE", "4")]))
        .arguments(args().layer("invocation-high").from(["--value", "7"]))
        .load()
        .unwrap();
    assert_eq!(loaded.value, 7);
}

#[test]
fn environment_descriptor_composes_selector_prefix_and_separator() {
    #[derive(Debug, Config)]
    struct Prefixed {
        value: u16,
    }

    let config = Prefixed::builder()
        .context("profile", "release")
        .environment(
            env()
                .prefix("APP")
                .separator("__")
                .when(selector!(profile = "release"))
                .from([("APP__VALUE", "11")]),
        )
        .load()
        .unwrap();

    assert_eq!(config.value, 11);
}

#[test]
fn physical_sources_enforce_configurable_resource_limits() {
    let limits = ResolutionLimits {
        max_document_bytes: 4,
        ..ResolutionLimits::default()
    };
    let error = AppConfig::builder()
        .limits(limits)
        .unwrap()
        .inline(json("oversized.json", r#"{"value":2}"#))
        .load()
        .unwrap_err();
    assert_eq!(error.code(), "limit_exceeded");
}
