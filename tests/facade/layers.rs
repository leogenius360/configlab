use super::support::tempdir;
use configlab::*;
use serde_json::json;
use std::fs;

#[derive(Debug, Config)]
struct CliConfig {
    #[config(default = 1)]
    jobs: u16,
    #[config(default = false, cli = "warnings-as-errors")]
    warnings_as_errors: bool,
    #[config(default = Vec::<String>::new(), merge = "append")]
    include_paths: Vec<String>,
}

#[test]
fn ephemeral_cli_accumulation_uses_explicit_order() {
    let first = configlab::Input::new("system", "project", configlab::Origin::new("system"))
        .ordered(1)
        .push(configlab::InputOperation::Set {
            path: "include_paths".into(),
            value: json!(["/usr/include"]),
        });
    let second = configlab::Input::new("project", "project", configlab::Origin::new("project"))
        .ordered(2)
        .push(configlab::InputOperation::Set {
            path: "include_paths".into(),
            value: json!(["./include"]),
        });
    let config = CliConfig::builder()
        .layer_order(["defaults", "project", "invocation", "override"])
        .unwrap()
        .input(first)
        .input(second)
        .arguments(args().from(["--warnings-as-errors"]))
        .load()
        .unwrap();
    assert_eq!(config.include_paths, vec!["/usr/include", "./include"]);
    assert!(config.warnings_as_errors);
}

#[test]
fn custom_layer_order_not_source_kind_controls_winner() {
    #[derive(Debug, Config)]
    struct One {
        value: u16,
    }
    let dir = tempdir("layers");
    let file = dir.join("x.json");
    fs::write(&file, r#"{"value":7}"#).unwrap();
    let config = One::builder()
        .layer_order(["defaults", "environment-low", "file-high", "override"])
        .unwrap()
        .environment(env().layer("environment-low").from([("VALUE", "9")]))
        .file(path(file).layer("file-high"))
        .load()
        .unwrap();
    assert_eq!(config.value, 7);
    fs::remove_dir_all(dir).ok();
}

#[test]
fn direct_override_is_highest_only_because_it_is_in_the_explicit_override_layer() {
    #[derive(Debug, Config)]
    struct Direct {
        #[config(default = 1)]
        value: u16,
    }
    let builder = Direct::builder()
        .layer_order(["defaults", "base", "environment", "override"])
        .unwrap()
        .inline(toml("base.toml", "value=2"))
        .environment(env().layer("environment").from([("VALUE", "3")]));
    let config = builder.set("value", 4u16).unwrap().load().unwrap();
    assert_eq!(config.value, 4);
}
