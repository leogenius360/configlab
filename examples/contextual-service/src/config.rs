use std::path::{Path, PathBuf};

use configlab::{
    Config, ConfigBuilder, ConfigError, LoadedConfig, SecretRef, args, env, path, selector,
};
use serde::{Deserialize, Serialize};

use super::validation;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Config)]
pub struct ServerConfig {
    #[config(default = "0.0.0.0", value_type = "address")]
    pub address: String,
    #[config(default = 8080, cli = "port")]
    pub port: u16,
    #[config(default = 30)]
    pub request_timeout: u64,
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Config)]
pub struct TlsConfig {
    #[config(default = false)]
    pub enabled: bool,
    #[config(required_when = "enabled")]
    pub certificate: Option<PathBuf>,
    #[config(required_when = "enabled")]
    pub private_key: Option<SecretRef>,
}

#[derive(Debug, Config)]
#[config(validate = "validation::database")]
pub struct DatabaseConfig {
    #[config(value_type = "address")]
    pub endpoint: String,
    pub name: String,
    #[config(sensitive)]
    pub username: String,
    pub password: SecretRef,
    #[config(default = 1)]
    pub pool_minimum: u16,
    #[config(default = 10)]
    pub pool_maximum: u16,
    #[config(default = 5)]
    pub connect_timeout: u64,
}

#[derive(Debug, Config)]
pub struct LoggingConfig {
    #[config(default = "info", value, value_type = "text")]
    pub level: LogLevel,
    #[config(default = "text")]
    pub format: String,
}

#[derive(Debug, Config)]
pub struct MetricsConfig {
    #[config(default = true)]
    pub enabled: bool,
    #[config(default = "/metrics")]
    pub path: String,
}

#[derive(Debug, Config)]
pub struct AlertingConfig {
    #[config(default = false)]
    pub enabled: bool,
    #[config(required_when = "enabled")]
    pub sender: Option<String>,
    #[config(required_when = "enabled", value_type = "address")]
    pub smtp_endpoint: Option<String>,
}

#[derive(Debug, Config)]
#[config(validate = "validation::gateway")]
pub struct GatewayConfig {
    #[config(env_prefix = "SERVER")]
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub logging: LoggingConfig,
    pub metrics: MetricsConfig,
    pub alerting: Option<AlertingConfig>,
}

fn config_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("config")
        .join(name)
}

pub fn process_builder(
    environment: &str,
    region: &str,
) -> Result<ConfigBuilder<GatewayConfig>, ConfigError> {
    Ok(GatewayConfig::builder()
        .layer_order([
            "defaults",
            "base",
            "contextual",
            "local",
            "environment",
            "invocation",
            "override",
        ])?
        .context("environment", environment)
        .context("region", region)
        .file(config_path("base.toml"))
        .file(
            path(config_path("production.toml"))
                .layer("contextual")
                .optional()
                .when(selector!(environment = "production")),
        )
        .file(
            path(config_path("west-africa.toml"))
                .layer("contextual")
                .optional()
                .when(selector!(
                    environment = "production",
                    region = "west-africa"
                )),
        )
        .file(path(config_path("local.toml")).layer("local").optional())
        .environment(env())
        .arguments(args()))
}

pub fn load_from_process(
    environment: &str,
    region: &str,
) -> Result<LoadedConfig<GatewayConfig>, ConfigError> {
    process_builder(environment, region)?.resolve()
}

#[cfg(test)]
mod tests {
    use super::*;
    use configlab::{ChangeKind, LogicalInput, Operation, Origin};

    #[test]
    fn regional_selector_wins_inside_contextual_layer() {
        let loaded = GatewayConfig::builder()
            .context("environment", "production")
            .context("region", "west-africa")
            .file(config_path("base.toml"))
            .file(
                path(config_path("production.toml"))
                    .layer("contextual")
                    .when(selector!(environment = "production")),
            )
            .file(
                path(config_path("west-africa.toml"))
                    .layer("contextual")
                    .when(selector!(
                        environment = "production",
                        region = "west-africa"
                    )),
            )
            .resolve()
            .unwrap();
        assert_eq!(loaded.server.request_timeout, 25);
        assert_eq!(loaded.database.pool_maximum, 30);
    }

    #[test]
    fn redacted_output_hides_database_credentials() {
        let loaded = GatewayConfig::builder()
            .file(config_path("base.toml"))
            .resolve()
            .unwrap();
        let redacted = loaded.redacted_value();
        assert_eq!(redacted["database"]["username"], "[REDACTED]");
        assert_eq!(redacted["database"]["password"], "[REDACTED]");
    }

    #[test]
    fn live_configuration_reconciles_complete_external_inputs() {
        let mut config = GatewayConfig::builder()
            .file(config_path("base.toml"))
            .live()
            .unwrap();
        let listener_and_logging = LogicalInput::new(
            "operations",
            "override",
            Origin::new("operations control plane"),
        )
        .push(Operation::Set {
            path: "server.port".into(),
            value: 9090.into(),
        })
        .push(Operation::Set {
            path: "logging.level".into(),
            value: "debug".into(),
        });
        let report = config.reconcile([listener_and_logging]).unwrap();
        assert_eq!(config.server.port, 9090);
        assert_eq!(config.logging.level, LogLevel::Debug);
        assert_eq!(report.changes().len(), 2);

        let logging_only = LogicalInput::new(
            "operations",
            "override",
            Origin::new("operations control plane"),
        )
        .push(Operation::Set {
            path: "logging.level".into(),
            value: "debug".into(),
        });
        let report = config.reconcile([logging_only]).unwrap();
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.logging.level, LogLevel::Debug);
        assert!(report.changes().changes().iter().any(|change| {
            change.path() == "server.port" && change.kind() == ChangeKind::Modified
        }));
    }
}
