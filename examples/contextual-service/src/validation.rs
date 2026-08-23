use super::config::{DatabaseConfig, GatewayConfig};

pub fn database(value: &DatabaseConfig) -> Result<(), &'static str> {
    (value.pool_minimum <= value.pool_maximum)
        .then_some(())
        .ok_or("pool_minimum must not exceed pool_maximum")
}

pub fn gateway(value: &GatewayConfig) -> Result<(), &'static str> {
    (value.database.connect_timeout < value.server.request_timeout)
        .then_some(())
        .ok_or("database connect_timeout must be shorter than server request_timeout")
}
