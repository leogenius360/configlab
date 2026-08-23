use crate::config::{self, LogLevel};
use configlab::path;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct DemoOutcome {
    pub startup_port: u16,
    pub refreshed_log_level: LogLevel,
    pub refreshed_rate: u64,
    pub partial_write_preserved_value: bool,
    pub invalid_update_preserved_value: bool,
    pub listener_changed_immediately: bool,
    pub removal_restored_defaults: bool,
    pub sensitive_change_was_redacted: bool,
    pub provenance_tracked: bool,
}

pub fn run(verbose: bool) -> Result<DemoOutcome, Box<dyn std::error::Error>> {
    let directory = DemoDirectory::new()?;
    let runtime_path = directory.path().join("runtime.json");
    let mut live = config::builder()?
        .file(path(&runtime_path).layer("override").optional())
        .live()?;
    let startup_port = live.server.port;

    if verbose {
        println!(
            "startup {} on {}:{}; logging={} {:?}; checkout_v2={}; upstream={}; credential={}",
            live.service_name,
            live.server.address,
            live.server.port,
            live.logging.format,
            live.logging.level,
            live.features.checkout_v2,
            live.upstream.endpoint,
            live.upstream.credential_ref,
        );
    }

    fs::copy(config::fixture("runtime.json"), &runtime_path)?;
    let initial = live.refresh()?;
    print_changes(verbose, "initial refresh", initial.changes());
    let sensitive_change_was_redacted = initial
        .changes()
        .changes()
        .iter()
        .find(|change| change.path() == "upstream.credential_ref")
        .is_some_and(|change| {
            change.is_sensitive() && change.previous().is_none() && change.current().is_none()
        });

    // A file can be observed between truncate and replace. Parse failure leaves
    // the current configuration untouched, and a later refresh can recover.
    fs::write(&runtime_path, "{")?;
    let previous_level = live.logging.level.clone();
    let partial_write_preserved_value =
        live.refresh().is_err() && live.logging.level == previous_level;

    // A syntactically valid but semantically invalid complete source also leaves
    // the current value untouched.
    fs::write(&runtime_path, r#"{"traffic":{"requests_per_second":0}}"#)?;
    let previous_rate = live.traffic.requests_per_second;
    let invalid_update_preserved_value =
        live.refresh().is_err() && live.traffic.requests_per_second == previous_rate;

    // Valid values take effect immediately after complete resolution and validation.
    fs::write(&runtime_path, r#"{"server":{"port":9090}}"#)?;
    let listener = live.refresh()?;
    print_changes(verbose, "listener refresh", listener.changes());
    let listener_changed_immediately = live.server.port == 9090;

    fs::write(
        &runtime_path,
        r#"{"logging":{"level":"warning"},"traffic":{"requests_per_second":400}}"#,
    )?;
    let hot = live.refresh()?;
    print_changes(verbose, "hot refresh", hot.changes());
    let provenance_tracked = live
        .loaded()
        .explain("logging.level")
        .iter()
        .any(|entry| entry.origin.source == runtime_path.display().to_string());
    let refreshed_log_level = live.logging.level.clone();
    let refreshed_rate = live.traffic.requests_per_second;

    // Removing an optional source removes all of its contributions on the next
    // complete refresh and restores lower-layer values.
    fs::remove_file(&runtime_path)?;
    let removed = live.refresh()?;
    print_changes(verbose, "source removal refresh", removed.changes());
    let removal_restored_defaults = live.logging.level == LogLevel::Info
        && live.traffic.requests_per_second == 100
        && live.server.port == startup_port;

    Ok(DemoOutcome {
        startup_port,
        refreshed_log_level,
        refreshed_rate,
        partial_write_preserved_value,
        invalid_update_preserved_value,
        listener_changed_immediately,
        removal_restored_defaults,
        sensitive_change_was_redacted,
        provenance_tracked,
    })
}

fn print_changes(verbose: bool, label: &str, changes: &configlab::ChangeSet) {
    if !verbose {
        return;
    }
    println!("{label}:");
    for change in changes.changes() {
        println!(
            "  {}: {:?}{}",
            change.path(),
            change.kind(),
            if change.is_sensitive() {
                " [values redacted]"
            } else {
                ""
            }
        );
    }
}

struct DemoDirectory(PathBuf);

impl DemoDirectory {
    fn new() -> std::io::Result<Self> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "configlab-runtime-example-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path)?;
        Ok(Self(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for DemoDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
