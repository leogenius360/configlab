#![forbid(unsafe_code)]

mod config;
mod demo;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let outcome = demo::run(true)?;
    println!(
        "final summary: startup_port={} refreshed_level={:?} refreshed_rate={} partial_write_preserved={} invalid_preserved={} listener_changed={} removal_restored={} sensitive_redacted={} provenance_tracked={}",
        outcome.startup_port,
        outcome.refreshed_log_level,
        outcome.refreshed_rate,
        outcome.partial_write_preserved_value,
        outcome.invalid_update_preserved_value,
        outcome.listener_changed_immediately,
        outcome.removal_restored_defaults,
        outcome.sensitive_change_was_redacted,
        outcome.provenance_tracked,
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_refresh_lifecycle_is_executable() {
        let outcome = demo::run(false).unwrap();
        assert_eq!(outcome.startup_port, 8080);
        assert_eq!(outcome.refreshed_log_level, config::LogLevel::Warning);
        assert_eq!(outcome.refreshed_rate, 400);
        assert!(outcome.partial_write_preserved_value);
        assert!(outcome.invalid_update_preserved_value);
        assert!(outcome.listener_changed_immediately);
        assert!(outcome.removal_restored_defaults);
        assert!(outcome.sensitive_change_was_redacted);
        assert!(outcome.provenance_tracked);
    }
}
