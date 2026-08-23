use configlab::*;

#[test]
fn toml_json_and_yaml_all_flow_through_the_same_resolver() {
    #[derive(Debug, Config)]
    struct FormatConfig {
        value: u16,
    }

    assert_eq!(
        FormatConfig::builder()
            .inline(toml("x.toml", "value=1"))
            .load()
            .unwrap()
            .value,
        1
    );
    assert_eq!(
        FormatConfig::builder()
            .inline(json("x.json", r#"{"value":2}"#))
            .load()
            .unwrap()
            .value,
        2
    );
    assert_eq!(
        FormatConfig::builder()
            .inline(yaml("x.yaml", "value: 3\n"))
            .load()
            .unwrap()
            .value,
        3
    );
}

#[test]
fn unknown_document_fields_are_rejected_before_resolution() {
    #[derive(Debug, Config)]
    struct Strict {
        #[config(default = 1)]
        value: u16,
    }
    let result = Strict::builder()
        .inline(toml("bad.toml", "value=2\nvaleu=3\n"))
        .load();
    assert!(matches!(result, Err(ConfigError::Parse { .. })));
}

#[test]
fn unknown_empty_sections_are_rejected_too() {
    #[derive(Debug, Config)]
    struct StrictEmpty {
        #[config(default = 1)]
        value: u16,
    }
    assert!(matches!(
        StrictEmpty::builder()
            .inline(json("bad.json", r#"{"typo":{}}"#))
            .load(),
        Err(ConfigError::Parse { .. })
    ));
}

#[test]
fn syntax_errors_do_not_echo_source_secret_text() {
    #[derive(Debug, Config)]
    struct SecretSyntax {
        password: SecretRef,
    }
    let error = SecretSyntax::builder()
        .inline(toml("bad.toml", "password = 'super-secret' trailing"))
        .load()
        .expect_err("invalid syntax must fail");
    assert!(!error.to_string().contains("super-secret"));
}

#[test]
fn arbitrary_values_preserve_keys_that_only_resemble_internal_markers() {
    #[derive(Debug, Config)]
    struct ArbitraryConfig {
        payload: serde_json::Value,
    }

    let loaded = ArbitraryConfig::builder()
        .inline(json(
            "marker.json",
            r#"{"payload":{"nested":{"__config_present":true}}}"#,
        ))
        .resolve()
        .unwrap();
    assert_eq!(
        loaded.redacted_value()["payload"]["nested"]["__config_present"],
        serde_json::json!(true)
    );
}
