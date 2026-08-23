#![forbid(unsafe_code)]

//! Smallest realistic `configlab` consumer.

use configlab::{Config, SecretRef, args, env, path};

#[derive(Debug, Config)]
struct AppConfig {
    /// HTTP port.
    #[config(default = 8080, env = "APP_PORT", cli = "port")]
    port: u16,
    /// Log filter.
    #[config(default = "info", env = "APP_LOG_LEVEL")]
    log_level: String,
    /// Optional secret reference used only in deployments that need it.
    #[config(env = "APP_DATABASE_PASSWORD")]
    database_password: Option<SecretRef>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::builder()
        .file(path("config.toml").optional())
        .environment(env())
        .arguments(args())
        .load()?;

    println!("port: {}", config.port);
    println!("log level: {}", config.log_level);
    println!(
        "database credentials configured: {}",
        config.database_password.is_some()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use configlab::{Config, json, toml, yaml};

    #[test]
    fn defaults_and_inline_override_work_without_extra_setup() {
        let config = AppConfig::builder()
            .inline(json("test.json", r#"{"port":9000}"#))
            .load()
            .unwrap();
        assert_eq!(config.port, 9000);
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn toml_json_and_yaml_documents_share_one_schema() {
        let cases = [
            toml("config.toml", "port = 9101"),
            json("config.json", r#"{"port":9101}"#),
            yaml("config.yaml", "port: 9101"),
        ];

        for source in cases {
            let config = AppConfig::builder().inline(source).load().unwrap();
            assert_eq!(config.port, 9101);
            assert_eq!(config.log_level, "info");
        }
    }
}
