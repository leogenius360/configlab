use super::support::tempdir;
use configlab::*;
use std::fs;

#[test]
fn discovery_finds_project_and_exposes_stable_context() {
    #[derive(Debug, Config)]
    struct Project {
        #[config(default = 1)]
        jobs: u16,
    }
    let root = tempdir("discover");
    fs::create_dir(root.join(".project")).unwrap();
    fs::write(root.join("config.toml"), "jobs=4\n").unwrap();
    let nested = root.join("src/deep");
    fs::create_dir_all(&nested).unwrap();
    let loaded = Project::builder()
        .layer_order(["defaults", "project", "override"])
        .unwrap()
        .discover_ancestor(&nested, ".project", "config.toml", "project")
        .unwrap()
        .resolve()
        .unwrap();
    assert_eq!(loaded.jobs, 4);
    assert_eq!(
        loaded.context().get("project_root"),
        Some(root.to_string_lossy().as_ref())
    );
    fs::remove_dir_all(root).ok();
}

#[test]
fn unavailable_selector_context_is_rejected_through_facade() {
    #[derive(Debug, Config)]
    struct One {
        #[config(default = 1)]
        value: u16,
    }
    let dir = tempdir("selector-missing");
    let file = dir.join("x.json");
    fs::write(&file, r#"{"value":2}"#).unwrap();
    let result = One::builder()
        .file(
            path(file)
                .layer("contextual")
                .when(selector!(profile = "release")),
        )
        .load();
    assert!(
        matches!(result,Err(ConfigError::Resolve(configlab::ResolveError::UnavailableContext{attribute})) if attribute=="profile")
    );
    fs::remove_dir_all(dir).ok();
}

#[test]
fn resolution_request_is_distinct_from_context_and_can_drive_discovery() {
    #[derive(Debug, Config)]
    struct ProjectRequestConfig {
        #[config(default = 1)]
        jobs: u16,
    }
    let root = tempdir("request-discovery");
    fs::create_dir(root.join(".project")).unwrap();
    fs::write(root.join("config.toml"), "jobs=6\n").unwrap();
    let nested = root.join("src/deep");
    fs::create_dir_all(&nested).unwrap();
    let request = ResolutionRequest::from_facts([
        ("working_directory", nested.to_string_lossy().to_string()),
        ("profile", "release".to_string()),
    ]);
    let loaded = ProjectRequestConfig::builder()
        .layer_order(["defaults", "project", "override"])
        .unwrap()
        .resolution_request(request)
        .context_from_request("profile", "profile")
        .unwrap()
        .discover_ancestor_from_request("working_directory", ".project", "config.toml", "project")
        .unwrap()
        .resolve()
        .unwrap();
    assert_eq!(loaded.jobs, 6);
    assert_eq!(loaded.context().get("profile"), Some("release"));
    assert_eq!(
        loaded.context().get("project_root"),
        Some(root.to_string_lossy().as_ref())
    );
    fs::remove_dir_all(root).ok();
}
