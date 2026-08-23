use configlab::*;
use serde_json::json;

#[test]
fn removal_operations_require_definition_permission() {
    let mut denied_schema = Schema::new();
    let mut denied = SettingSpec::typed("paths", ValueType::Text);
    denied.shape = ValueShape::OrderedList;
    denied.merge = MergePolicy::Append;
    denied.default = Some(DefaultRule::Fixed(json!(["a", "b"])));
    denied_schema.insert(denied).unwrap();

    let removal = LogicalInput::new("remove", "override", Origin::new("cli")).push(
        Operation::RemoveListItem {
            path: "paths".into(),
            value: json!("a"),
        },
    );
    let denied_resolver =
        Resolver::new(denied_schema, LayerOrder::new(["override"]).unwrap()).unwrap();
    assert!(matches!(
        denied_resolver.resolve(&Context::new(), std::slice::from_ref(&removal)),
        Err(ResolveError::RemovalNotAllowed { .. })
    ));

    let mut allowed_schema = Schema::new();
    let mut allowed = SettingSpec::typed("paths", ValueType::Text);
    allowed.shape = ValueShape::OrderedList;
    allowed.merge = MergePolicy::Append;
    allowed.removal = RemovalPolicy::unset_only().with_list_item_removal();
    allowed.default = Some(DefaultRule::Fixed(json!(["a", "b"])));
    allowed_schema.insert(allowed).unwrap();
    let allowed_resolver =
        Resolver::new(allowed_schema, LayerOrder::new(["override"]).unwrap()).unwrap();
    let effective = allowed_resolver
        .resolve(&Context::new(), &[removal])
        .unwrap();
    assert_eq!(effective.get("paths"), Some(&json!(["b"])));
}

#[test]
fn nonexistent_item_removal_is_resolution_information_not_a_diagnostic() {
    let mut schema = Schema::new();
    let mut paths = SettingSpec::typed("paths", ValueType::Text);
    paths.shape = ValueShape::OrderedList;
    paths.merge = MergePolicy::Append;
    paths.removal = RemovalPolicy::unset_only().with_list_item_removal();
    paths.default = Some(DefaultRule::Fixed(json!(["a"])));
    schema.insert(paths).unwrap();

    let resolver = Resolver::new(schema, LayerOrder::new(["override"]).unwrap()).unwrap();
    let input = LogicalInput::new("remove", "override", Origin::new("cli")).push(
        Operation::RemoveListItem {
            path: "paths".into(),
            value: json!("missing"),
        },
    );
    let report = resolver.resolve_report(&Context::new(), &[input]).unwrap();

    assert!(report.is_success());
    assert!(report.diagnostics.is_empty());
    assert!(report.information.iter().any(|information| matches!(
        information,
        ResolutionInformation::RemovalNoop {
            path,
            operation: RemovalKind::ListItem,
        } if path == "paths"
    )));
}
