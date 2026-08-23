use configlab::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[test]
fn derived_default_lookup_works_through_facade() {
    #[derive(Debug, Config)]
    struct Db {
        #[config(default = "postgres")]
        engine: String,
        port: Option<u16>,
    }
    let config = Db::builder()
        .derived_default(
            "port",
            "engine",
            [("postgres", 5432u16), ("mysql", 3306u16)],
        )
        .unwrap()
        .load()
        .unwrap();
    assert_eq!(config.port, Some(5432));
}

#[test]
fn generated_environment_and_cli_binding_collisions_fail_at_schema_boundary() {
    #[derive(Debug, Config)]
    struct EnvCollision {
        #[config(env = "SAME")]
        first: u16,
        #[config(env = "SAME")]
        second: u16,
    }
    assert!(matches!(
        EnvCollision::builder().load(),
        Err(ConfigError::Schema(_))
    ));

    #[derive(Debug, Config)]
    struct CliCollision {
        #[config(cli = "same")]
        first: u16,
        #[config(cli = "same")]
        second: u16,
    }
    assert!(matches!(
        CliCollision::builder().load(),
        Err(ConfigError::Schema(_))
    ));
}

fn default_workers() -> u16 {
    6
}

#[test]
fn default_with_function_is_supported_by_the_derive_facade() {
    #[derive(Debug, Config)]
    struct FunctionalDefault {
        #[config(default_with = "default_workers")]
        workers: u16,
    }
    assert_eq!(FunctionalDefault::builder().load().unwrap().workers, 6);
}

#[test]
fn custom_leaf_objects_can_be_loaded_from_environment_json() {
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct Endpoint {
        host: String,
        port: u16,
    }
    #[derive(Debug, Config)]
    struct CustomLeaf {
        #[config(value, env = "ENDPOINT")]
        endpoint: Endpoint,
    }
    let config = CustomLeaf::builder()
        .environment(env().from([("ENDPOINT", r#"{"host":"db.internal","port":5432}"#)]))
        .load()
        .unwrap();
    assert_eq!(
        config.endpoint,
        Endpoint {
            host: "db.internal".into(),
            port: 5432
        }
    );
}

#[test]
fn derive_named_map_can_combine_by_key_across_layers() {
    use std::collections::BTreeMap;

    #[derive(Debug, Config)]
    struct MapConfig {
        #[config(merge = "combine_by_key")]
        outputs: BTreeMap<String, bool>,
    }

    let loaded = MapConfig::builder()
        .inline(json("base.json", r#"{"outputs":{"console":true}}"#))
        .inline(json("contextual.json", r#"{"outputs":{"audit":true}}"#).layer("contextual"))
        .resolve()
        .unwrap();
    assert_eq!(loaded.outputs.get("console"), Some(&true));
    assert_eq!(loaded.outputs.get("audit"), Some(&true));
    assert_eq!(loaded.explain("outputs.console")[0].input_id, "inline-0");
    assert_eq!(loaded.explain("outputs.audit")[0].input_id, "inline-1");
}

#[test]
fn derive_emits_kernel_type_representation_and_documentation_metadata() {
    #[derive(Debug, Config)]
    struct MetadataConfig {
        /// Maximum number of workers.
        #[config(default = 4)]
        workers: u16,
        secret: SecretRef,
        #[config(external_reference)]
        documentation: Option<String>,
    }

    let schema = MetadataConfig::schema("").unwrap();
    let workers = schema.get("workers").unwrap();
    assert_eq!(workers.value_type, configlab::ValueType::UnsignedInteger);
    assert_eq!(
        workers.description.as_deref(),
        Some("Maximum number of workers.")
    );

    let secret = schema.get("secret").unwrap();
    assert_eq!(
        secret.representation,
        configlab::Representation::SecretReference
    );
    assert_eq!(secret.disclosure, configlab::Disclosure::Sensitive);

    let documentation = schema.get("documentation").unwrap();
    assert_eq!(
        documentation.representation,
        configlab::Representation::ExternalReference
    );
}

#[test]
fn derive_declares_collection_removal_permissions_for_the_kernel() {
    #[derive(Debug, Config)]
    struct RemoveConfig {
        #[config(
            default = Vec::<String>::new(),
            merge = "append",
            removal = "unset,items"
        )]
        values: Vec<String>,
    }

    let base = configlab::Input::new("base", "base", configlab::Origin::new("base"))
        .ordered(1)
        .push(configlab::InputOperation::Set {
            path: "values".into(),
            value: json!(["a", "b"]),
        });
    let remove = configlab::Input::new("remove", "override", configlab::Origin::new("override"))
        .push(configlab::InputOperation::RemoveListItem {
            path: "values".into(),
            value: json!("a"),
        });

    let config = RemoveConfig::builder()
        .input(base)
        .input(remove)
        .load()
        .unwrap();
    assert_eq!(config.values, vec!["b"]);
}

#[test]
fn deprecated_derive_metadata_produces_a_nonblocking_kernel_warning() {
    #[derive(Debug, Config)]
    struct DeprecatedConfig {
        #[config(default = "legacy", deprecated = "use mode instead")]
        legacy_mode: String,
    }

    let loaded = DeprecatedConfig::builder().resolve().unwrap();
    assert_eq!(loaded.legacy_mode, "legacy");
    assert!(loaded.diagnostics().iter().any(|diagnostic| {
        diagnostic.severity == configlab::DiagnosticSeverity::Warning
            && diagnostic.code == configlab::DiagnosticCode::DeprecatedSetting
    }));
}

#[test]
fn full_width_integer_types_round_trip_through_the_typed_facade() {
    #[derive(Debug, Config)]
    struct WideIntegers {
        signed: i128,
        unsigned: u128,
    }

    let config = WideIntegers::builder()
        .set("signed", i128::MIN)
        .unwrap()
        .set("unsigned", u128::MAX)
        .unwrap()
        .load()
        .unwrap();

    assert_eq!(config.signed, i128::MIN);
    assert_eq!(config.unsigned, u128::MAX);
}

#[test]
fn derived_defaults_reject_duplicate_cases_and_duplicate_target_customization() {
    #[derive(Debug, Config)]
    struct Db {
        #[config(default = "postgres")]
        engine: String,
        port: Option<u16>,
    }

    let duplicate_cases = match Db::builder().derived_default(
        "port",
        "engine",
        [("postgres", 5432_u16), ("postgres", 15432_u16)],
    ) {
        Ok(_) => panic!("duplicate lookup cases must fail"),
        Err(error) => error,
    };
    assert!(
        duplicate_cases
            .to_string()
            .contains("duplicate lookup cases")
    );

    let builder = Db::builder()
        .derived_default("port", "engine", [("postgres", 5432_u16)])
        .unwrap();
    let duplicate_target = match builder.derived_default("port", "engine", [("mysql", 3306_u16)]) {
        Ok(_) => panic!("duplicate derived-default customization must fail"),
        Err(error) => error,
    };
    assert!(
        duplicate_target
            .to_string()
            .contains("configured more than once")
    );
}

#[test]
fn derived_default_encode_failures_name_the_target_path() {
    use serde::ser::{Error as _, Serializer};

    struct FailingValue;
    impl Serialize for FailingValue {
        fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            Err(S::Error::custom("intentional encode failure"))
        }
    }

    #[derive(Debug, Config)]
    struct Db {
        #[config(default = "postgres")]
        engine: String,
        port: Option<String>,
    }

    let error = match Db::builder().derived_default("port", "engine", [("postgres", FailingValue)])
    {
        Ok(_) => panic!("failing serialization must be surfaced"),
        Err(error) => error,
    };
    let rendered = error.to_string();
    assert!(rendered.contains("`port`"));
    assert!(rendered.contains("intentional encode failure"));
    assert!(!rendered.contains("invalid value for `postgres`"));
}
