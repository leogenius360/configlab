#![forbid(unsafe_code)]

mod config;
mod validation;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let loaded = config::load_from_process("production", "west-africa")?;
    println!(
        "service listening on {}:{}",
        loaded.server.address, loaded.server.port
    );
    println!("database endpoint: {}", loaded.database.endpoint);
    println!("database name: {}", loaded.database.name);
    println!(
        "database username configured: {}",
        !loaded.database.username.is_empty()
    );
    println!("database password reference: {}", loaded.database.password);
    println!("request timeout: {}", loaded.server.request_timeout);
    println!(
        "logging: {:?} ({})",
        loaded.logging.level, loaded.logging.format
    );
    println!(
        "metrics: enabled={} path={}",
        loaded.metrics.enabled, loaded.metrics.path
    );

    if let Some(tls) = &loaded.server.tls {
        println!(
            "tls: enabled={} certificate_configured={} private_key_configured={}",
            tls.enabled,
            tls.certificate.is_some(),
            tls.private_key.is_some()
        );
    } else {
        println!("tls: not configured");
    }

    if let Some(alerting) = &loaded.alerting {
        println!(
            "alerting: enabled={} sender_configured={} smtp_endpoint_configured={}",
            alerting.enabled,
            alerting.sender.is_some(),
            alerting.smtp_endpoint.is_some()
        );
    } else {
        println!("alerting: not configured");
    }

    println!(
        "safe configuration: {}",
        serde_json::to_string_pretty(&loaded.redacted_value())?
    );
    if let Some(origin) = loaded.explain("server.port").last() {
        println!(
            "server.port provenance: layer={} source={}",
            origin.layer, origin.origin.source
        );
    }
    println!(
        "resolution information records: {}; diagnostics: {}",
        loaded.information().len(),
        loaded.diagnostics().len()
    );
    Ok(())
}
