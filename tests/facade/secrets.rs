use configlab::*;
use serde_json::json;

#[test]
fn secret_explanations_are_redacted() {
    #[derive(Debug, Config)]
    struct SecretConfig {
        password: SecretRef,
    }
    let loaded = SecretConfig::builder()
        .environment(env().from([("PASSWORD", "secret://value")]))
        .resolve()
        .unwrap();
    let explanation = loaded.explain("password");
    assert_eq!(
        explanation.last().and_then(|entry| entry.value.clone()),
        Some(json!("[REDACTED]"))
    );
}

#[test]
fn sensitive_decode_failures_redact_decoder_details() {
    #[derive(Debug, Config)]
    struct SensitiveDecode {
        #[config(sensitive)]
        password: String,
    }
    let error = SensitiveDecode::builder()
        .inline(json(
            "bad.json",
            r#"{"password":{"actual":"super-secret"}}"#,
        ))
        .load()
        .expect_err("wrong type must fail");
    let rendered = error.to_string();
    assert!(rendered.contains("sensitive value is invalid"));
    assert!(!rendered.contains("super-secret"));
}

#[test]
fn secret_collection_wrappers_keep_the_underlying_collection_shape() {
    #[derive(Debug, Config)]
    struct SecretList {
        values: configlab::Secret<Vec<String>>,
    }
    let config = SecretList::builder()
        .inline(json("values.json", r#"{"values":["a","b"]}"#))
        .load()
        .unwrap();
    assert_eq!(
        config.values.expose(),
        &vec!["a".to_string(), "b".to_string()]
    );
}

#[test]
fn secret_wrappers_redact_formatting_but_serialize_their_data_contract() {
    let reference = SecretRef::new("secret://database/password");
    assert_eq!(
        reference,
        SecretRef::new("secret://database/password"),
        "secret references are identifiers and retain ordinary equality"
    );
    assert_eq!(format!("{reference}"), "[REDACTED]");
    assert!(!format!("{reference:?}").contains("database/password"));
    assert_eq!(
        serde_json::to_value(&reference).unwrap(),
        json!("secret://database/password")
    );

    let material = Secret::new("do-not-log".to_string());
    assert_eq!(format!("{material}"), "[REDACTED]");
    assert!(!format!("{material:?}").contains("do-not-log"));
    assert_eq!(
        serde_json::to_value(&material).unwrap(),
        json!("do-not-log")
    );
}

#[test]
fn nested_secret_containers_are_sensitive_and_redact_their_complete_shape() {
    use std::collections::BTreeMap;

    #[derive(Debug, Config)]
    struct NestedSecrets {
        #[config(merge = "append")]
        tokens: Vec<Secret<String>>,
        references: Vec<SecretRef>,
        #[config(merge = "combine_by_key")]
        by_name: BTreeMap<String, Secret<String>>,
        optional_references: Option<Vec<SecretRef>>,
    }

    let loaded = NestedSecrets::builder()
        .inline(json(
            "secrets.json",
            r#"{
                "tokens":["token-one","token-two"],
                "references":["secret://one","secret://two"],
                "by_name":{"primary":"map-secret"},
                "optional_references":["secret://optional"]
            }"#,
        ))
        .resolve()
        .unwrap();

    for path in ["tokens", "references", "by_name", "optional_references"] {
        let setting = loaded.schema().get(path).unwrap();
        assert_eq!(setting.disclosure, Disclosure::Sensitive, "{path}");
        assert_eq!(setting.value_type, ValueType::Text, "{path}");
    }
    assert_eq!(
        loaded.schema().get("references").unwrap().representation,
        Representation::Ordinary
    );
    assert_eq!(
        loaded
            .schema()
            .get("optional_references")
            .unwrap()
            .representation,
        Representation::Ordinary
    );

    let redacted = loaded.redacted_value();
    assert_eq!(redacted["tokens"], json!("[REDACTED]"));
    assert_eq!(redacted["references"], json!("[REDACTED]"));
    assert_eq!(redacted["by_name"], json!("[REDACTED]"));
    assert_eq!(redacted["optional_references"], json!("[REDACTED]"));
    let rendered = redacted.to_string();
    for secret in [
        "token-one",
        "token-two",
        "secret://one",
        "secret://two",
        "primary",
        "map-secret",
        "secret://optional",
    ] {
        assert!(
            !rendered.contains(secret),
            "redacted output leaked {secret}"
        );
    }

    let token_trace = loaded.explain("tokens");
    assert_eq!(token_trace.len(), 1);
    assert_eq!(token_trace[0].item, None);
    assert_eq!(token_trace[0].value, Some(json!("[REDACTED]")));
    assert!(loaded.explain("by_name.primary").is_empty());
}
