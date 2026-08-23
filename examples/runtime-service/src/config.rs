use configlab::{Config, ConfigBuilder, ConfigError, ResolutionLimits, SecretRef};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Config)]
pub struct ServerConfig {
    #[config(value_type = "address")]
    pub address: String,
    pub port: u16,
}

#[derive(Debug, Config)]
pub struct LoggingConfig {
    #[config(value, value_type = "text")]
    pub level: LogLevel,
    pub format: String,
}

#[derive(Debug, Config)]
#[config(validate = "validate_traffic")]
pub struct TrafficConfig {
    pub requests_per_second: u64,
    pub max_in_flight: u64,
}

fn validate_traffic(value: &TrafficConfig) -> Result<(), &'static str> {
    if value.requests_per_second == 0 {
        return Err("requests_per_second must be greater than zero");
    }
    if value.max_in_flight == 0 {
        return Err("max_in_flight must be greater than zero");
    }
    Ok(())
}

#[derive(Debug, Config)]
pub struct FeatureConfig {
    pub checkout_v2: bool,
}

#[derive(Debug, Config)]
pub struct UpstreamConfig {
    #[config(value_type = "address")]
    pub endpoint: String,
    pub credential_ref: SecretRef,
}

#[derive(Debug, Config)]
pub struct ServiceConfig {
    pub service_name: String,
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    pub traffic: TrafficConfig,
    pub features: FeatureConfig,
    pub upstream: UpstreamConfig,
}

pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("config")
        .join(name)
}

pub fn builder() -> Result<ConfigBuilder<ServiceConfig>, ConfigError> {
    ServiceConfig::builder()
        .limits(ResolutionLimits {
            max_document_bytes: 1024 * 1024,
            max_logical_bytes: 4 * 1024 * 1024,
            ..ResolutionLimits::default()
        })
        .map(|builder| builder.file(fixture("base.toml")))
}
