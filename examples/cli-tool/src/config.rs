use std::path::{Path, PathBuf};

use configlab::{Config, ConfigBuilder, ConfigError, LoadedConfig, SecretRef, args, env, path};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildProfile {
    Development,
    Release,
}

#[derive(Debug, Config)]
#[config(validate = "validate_config")]
pub struct CliConfig {
    #[config(
        default = "development",
        value,
        value_type = "text",
        env = "FORGEPACK_PROFILE",
        cli = "profile"
    )]
    pub profile: BuildProfile,
    #[config(
        default_with = "default_input_dir",
        env = "FORGEPACK_INPUT",
        cli = "input"
    )]
    pub input_dir: PathBuf,
    #[config(
        default_with = "default_output_dir",
        env = "FORGEPACK_OUTPUT",
        cli = "output"
    )]
    pub output_dir: PathBuf,
    #[config(default = 4, env = "FORGEPACK_JOBS", cli = "jobs")]
    pub jobs: u16,
    #[config(default = true, env = "FORGEPACK_MINIFY", cli = "minify")]
    pub minify: bool,
    #[config(
        default = false,
        env = "FORGEPACK_FAIL_ON_WARNING",
        cli = "fail-on-warning"
    )]
    pub fail_on_warning: bool,
    #[config(default = Vec::<String>::new(), merge = "append", removal = "unset,items")]
    pub include_paths: Vec<String>,
    #[config(env = "FORGEPACK_REGISTRY_TOKEN")]
    pub registry_token: Option<SecretRef>,
}

fn default_input_dir() -> PathBuf {
    PathBuf::from("assets")
}
fn default_output_dir() -> PathBuf {
    PathBuf::from("dist")
}

fn validate_config(value: &CliConfig) -> Result<(), &'static str> {
    (value.input_dir != value.output_dir)
        .then_some(())
        .ok_or("input_dir and output_dir must be different")
}

pub fn project_config_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("config")
        .join("forgepack.toml")
}

pub fn process_builder() -> Result<ConfigBuilder<CliConfig>, ConfigError> {
    Ok(CliConfig::builder()
        .layer_order([
            "defaults",
            "user",
            "project",
            "environment",
            "invocation",
            "override",
        ])?
        .file(path(project_config_path()).layer("project").optional())
        .environment(env())
        .arguments(args()))
}

pub fn load_from_process() -> Result<LoadedConfig<CliConfig>, ConfigError> {
    process_builder()?.resolve()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn project_file_drives_cli_configuration() {
        let loaded = CliConfig::builder()
            .layer_order([
                "defaults",
                "project",
                "environment",
                "invocation",
                "override",
            ])
            .unwrap()
            .file(path(project_config_path()).layer("project").optional())
            .resolve()
            .unwrap();
        assert_eq!(loaded.profile, BuildProfile::Release);
        assert_eq!(loaded.jobs, 6);
    }

    #[test]
    fn deterministic_environment_and_arguments_follow_layer_order() {
        let loaded = CliConfig::builder()
            .layer_order([
                "defaults",
                "project",
                "environment",
                "invocation",
                "override",
            ])
            .unwrap()
            .file(path(project_config_path()).layer("project").optional())
            .environment(env().from([("FORGEPACK_JOBS", "8")]))
            .arguments(args().from(["--jobs", "12", "--no-minify"]))
            .load()
            .unwrap();
        assert_eq!(loaded.jobs, 12);
        assert!(!loaded.minify);
    }

    #[test]
    fn shared_argv_can_explicitly_pass_through_unknown_flags() {
        let loaded = CliConfig::builder()
            .arguments(args().ignore_unknown().from([
                "--owned-by-another-parser=value",
                "--jobs",
                "9",
            ]))
            .load()
            .unwrap();
        assert_eq!(loaded.jobs, 9);
    }

    #[test]
    fn project_configuration_can_be_discovered_from_a_nested_working_directory() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("configlab-cli-discovery-{nonce}"));
        let nested = root.join("workspace").join("package");
        fs::create_dir_all(&nested).unwrap();
        fs::write(root.join(".forgepack-project"), "").unwrap();
        fs::write(
            root.join("forgepack.toml"),
            "profile = \"release\"\njobs = 14\n",
        )
        .unwrap();

        let loaded = CliConfig::builder()
            .layer_order(["defaults", "project", "override"])
            .unwrap()
            .discover_ancestor(&nested, ".forgepack-project", "forgepack.toml", "project")
            .unwrap()
            .resolve()
            .unwrap();
        assert_eq!(loaded.profile, BuildProfile::Release);
        assert_eq!(loaded.jobs, 14);
        assert!(loaded.context().get("project_root").is_some());

        fs::remove_dir_all(root).unwrap();
    }
}
