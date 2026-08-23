use configlab::{Config, SecretRef};
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Config)]
pub(crate) struct HttpConfig {
    #[config(default = "0.0.0.0")]
    pub(crate) address: String,
    #[config(default = 8080, cli = "port")]
    pub(crate) port: u16,
    #[config(default = 30)]
    pub(crate) request_timeout: u64,
}

#[derive(Debug, Config)]
#[config(validate = "validate_database")]
pub(crate) struct DatabaseConfig {
    pub(crate) endpoint: String,
    pub(crate) name: String,
    #[config(sensitive)]
    pub(crate) username: String,
    pub(crate) password: SecretRef,
    #[config(default = 1)]
    pub(crate) pool_minimum: u16,
    #[config(default = 10)]
    pub(crate) pool_maximum: u16,
    #[config(default = 5)]
    pub(crate) connect_timeout: u64,
}

fn validate_database(value: &DatabaseConfig) -> Result<(), &'static str> {
    if value.pool_minimum <= value.pool_maximum {
        Ok(())
    } else {
        Err("pool_minimum must not exceed pool_maximum")
    }
}

#[derive(Debug, Config)]
pub(crate) struct LoggingConfig {
    #[config(default = "info", value)]
    pub(crate) level: LogLevel,
    #[config(default = "text")]
    pub(crate) format: String,
}

#[derive(Debug, Config)]
pub(crate) struct EmailConfig {
    #[config(default = false)]
    pub(crate) enabled: bool,
    #[config(required_when = "enabled")]
    pub(crate) sender: Option<String>,
    #[config(required_when = "enabled")]
    pub(crate) smtp_host: Option<String>,
}

#[derive(Debug, Config)]
#[config(validate = "validate_orders")]
pub(crate) struct OrdersConfig {
    #[config(env_prefix = "SERVER")]
    pub(crate) server: HttpConfig,
    pub(crate) database: DatabaseConfig,
    pub(crate) logging: LoggingConfig,
    pub(crate) email: Option<EmailConfig>,
}

fn validate_orders(value: &OrdersConfig) -> Result<(), &'static str> {
    if value.database.connect_timeout >= value.server.request_timeout {
        return Err("database connect_timeout must be shorter than server request_timeout");
    }
    Ok(())
}

pub(crate) fn tempdir(name: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("configlab-{name}-{nonce}"));
    fs::create_dir_all(&path).unwrap();
    path
}
