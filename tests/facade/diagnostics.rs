use configlab::*;
use std::error::Error;

#[test]
fn warning_diagnostics_do_not_block_effective_configuration() {
    #[derive(Debug, Config)]
    struct WarnConfig {
        #[config(default = 4)]
        workers: u16,
    }

    let loaded = WarnConfig::builder()
        .validate_with(|config| {
            if config.workers < 8 {
                vec![ValidationIssue::warning_at("workers", "low worker count")]
            } else {
                vec![]
            }
        })
        .resolve()
        .unwrap();
    assert_eq!(loaded.workers, 4);
    assert_eq!(loaded.warnings().len(), 1);
    assert_eq!(loaded.warnings()[0].severity, configlab::Severity::Warning);
}

#[test]
fn facade_errors_expose_stable_machine_readable_codes() {
    let cases = [
        (
            ConfigError::Resolve(ResolveError::UnknownLayer("typo".into())),
            "unknown_layer",
        ),
        (
            ConfigError::Io {
                path: "config.toml".into(),
                message: "not found".into(),
            },
            "io",
        ),
        (
            ConfigError::UnsupportedFormat("config.ini".into()),
            "unsupported_format",
        ),
        (
            ConfigError::Parse {
                source: "config.toml".into(),
                message: "invalid syntax".into(),
            },
            "parse",
        ),
        (
            ConfigError::Decode {
                path: "server.port".into(),
                message: "expected integer".into(),
            },
            "decode",
        ),
        (ConfigError::Validation(Vec::new()), "typed_validation"),
        (ConfigError::Schema("invalid definition".into()), "schema"),
        (ConfigError::Argument("bad flag".into()), "argument"),
        (ConfigError::Environment("bad value".into()), "environment"),
        (
            ConfigError::Limit {
                resource: "input".into(),
                limit: 1,
            },
            "limit_exceeded",
        ),
    ];

    for (error, expected) in cases {
        assert_eq!(error.code(), expected);
    }
}

#[test]
fn facade_resolve_errors_preserve_their_error_chain() {
    let error = ConfigError::Resolve(ResolveError::UnknownLayer("typo".into()));

    let source = error.source().expect("resolver error should be chained");
    let resolver = source
        .downcast_ref::<ResolveError>()
        .expect("source should retain its concrete resolver error type");

    assert_eq!(resolver, &ResolveError::UnknownLayer("typo".into()));
    assert_eq!(resolver.code(), error.code());
}

#[test]
fn string_only_facade_errors_do_not_claim_an_underlying_source() {
    let io = ConfigError::Io {
        path: "config.toml".into(),
        message: "permission denied".into(),
    };
    let parse = ConfigError::Parse {
        source: "config.toml".into(),
        message: "invalid syntax".into(),
    };

    assert!(io.source().is_none());
    assert!(parse.source().is_none());
}

#[test]
fn typed_resolution_report_preserves_all_blocking_diagnostics() {
    #[derive(Debug, Config)]
    struct RequiredConfig {
        first: String,
        second: u16,
    }

    let report = RequiredConfig::builder().resolve_report().unwrap();
    assert!(!report.is_success());
    assert!(report.loaded().is_none());
    assert_eq!(report.diagnostics().len(), 2);
    assert!(report.diagnostics().iter().all(|diagnostic| {
        diagnostic.code == DiagnosticCode::MissingRequired
            && diagnostic.severity == DiagnosticSeverity::Error
    }));
}

#[test]
fn typed_resolution_report_preserves_all_application_validation_issues() {
    #[derive(Debug, Config)]
    struct ValidatedConfig {
        #[config(default = 1)]
        value: u16,
    }

    let report = ValidatedConfig::builder()
        .validate_with(|_| {
            vec![
                ValidationIssue::at("value", "first error"),
                ValidationIssue::at("value", "second error"),
                ValidationIssue::warning_at("value", "warning"),
            ]
        })
        .resolve_report()
        .unwrap();
    assert!(!report.is_success());
    assert_eq!(report.validation().len(), 3);
}
