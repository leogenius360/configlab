use configlab::*;

#[test]
fn exact_environment_binding_is_not_modified_by_builder_prefix() {
    #[derive(Debug, Config)]
    struct EnvConfig {
        #[config(env = "EXACT_PORT")]
        port: u16,
    }
    let config = EnvConfig::builder()
        .environment(
            env()
                .prefix("IGNORED_FOR_EXACT")
                .from([("EXACT_PORT", "8081"), ("IGNORED_FOR_EXACT_PORT", "9999")]),
        )
        .load()
        .unwrap();
    assert_eq!(config.port, 8081);
}

#[test]
fn invalid_process_environment_binding_names_fail_before_lookup() {
    #[derive(Debug, Config)]
    struct EnvConfig {
        #[config(env = "BAD=NAME")]
        value: String,
    }

    let error = match EnvConfig::builder().environment(env()).load() {
        Ok(_) => panic!("invalid environment binding must fail schema validation"),
        Err(error) => error,
    };
    let rendered = error.to_string();
    assert!(rendered.contains("environment binding names cannot contain"));
    assert!(!rendered.contains("value"));
}

#[test]
fn argument_sources_are_strict_by_default_and_can_explicitly_ignore_unknowns() {
    #[derive(Debug, Config)]
    struct CliConfig {
        #[config(default = 8080, cli = "port")]
        port: u16,
    }

    let error = CliConfig::builder()
        .arguments(args().from(["--porrt", "9090"]))
        .load()
        .unwrap_err();
    assert_eq!(error.code(), "argument");
    assert!(error.to_string().contains("unknown configuration flag"));

    let config = CliConfig::builder()
        .arguments(args().ignore_unknown().from(["--porrt", "9090"]))
        .load()
        .unwrap();
    assert_eq!(config.port, 8080);
}

#[test]
fn duplicate_argument_bindings_fail_at_the_source_boundary() {
    #[derive(Debug, Config)]
    struct CliConfig {
        #[config(default = 8080, cli = "port")]
        port: u16,
    }

    let error = CliConfig::builder()
        .arguments(args().from(["--port", "8081", "--port", "8082"]))
        .load()
        .unwrap_err();
    assert_eq!(error.code(), "argument");
    assert!(error.to_string().contains("supplied more than once"));
}
