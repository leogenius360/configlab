use super::support::tempdir;
use configlab::*;
use std::fs;

#[test]
fn optional_file_descriptor_supports_missing_files_in_explicit_layers() {
    #[derive(Debug, Config)]
    struct OptionalProjectConfig {
        #[config(default = 3)]
        value: u16,
    }

    let missing = std::env::temp_dir().join("configlab-optional-project-missing.toml");
    let loaded = OptionalProjectConfig::builder()
        .layer_order(["defaults", "project", "override"])
        .unwrap()
        .file(path(missing).layer("project").optional())
        .load()
        .unwrap();

    assert_eq!(loaded.value, 3);
}

#[test]
fn optional_file_descriptor_uses_contextual_specificity_when_file_exists() {
    #[derive(Debug, Config)]
    struct OptionalContextConfig {
        #[config(default = 3)]
        value: u16,
    }

    let root = tempdir("optional-contextual");
    let production_path = root.join("production.toml");
    fs::write(&production_path, "value=9\n").unwrap();

    let loaded = OptionalContextConfig::builder()
        .context("environment", "production")
        .file(
            path(&production_path)
                .layer("contextual")
                .optional()
                .when(selector!(environment = "production")),
        )
        .load()
        .unwrap();

    assert_eq!(loaded.value, 9);
    fs::remove_dir_all(root).ok();
}

#[test]
fn missing_optional_file_still_rejects_an_undeclared_layer() {
    #[derive(Debug, Config)]
    struct OptionalLayerConfig {
        #[config(default = 3)]
        value: u16,
    }

    let missing = std::env::temp_dir().join("configlab-optional-unknown-layer.toml");
    let error = OptionalLayerConfig::builder()
        .layer_order(["defaults", "project", "override"])
        .unwrap()
        .file(path(missing).layer("typo").optional())
        .load()
        .expect_err("an undeclared layer is a definition error even when the file is absent");

    assert!(matches!(
        error,
        ConfigError::Resolve(configlab::ResolveError::UnknownLayer(layer)) if layer == "typo"
    ));
}
