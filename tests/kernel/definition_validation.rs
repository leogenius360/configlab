use configlab::*;
use serde_json::json;

#[test]
fn schema_rejects_unknown_conditional_paths_and_incompatible_removal_policies() {
    let mut unknown_condition = Schema::new();
    let mut field = SettingSpec::typed("certificate", ValueType::Path);
    field.requirement = Requirement::RequiredWhen(Condition::boolean("missing.flag"));
    unknown_condition.insert(field).unwrap();
    assert!(matches!(
        Resolver::new(unknown_condition, LayerOrder::new(["base"]).unwrap()),
        Err(ResolveError::InvalidSchema(_))
    ));

    let mut invalid_removal = Schema::new();
    let mut scalar = SettingSpec::typed("port", ValueType::UnsignedInteger);
    scalar.removal = RemovalPolicy::unset_only().with_list_item_removal();
    invalid_removal.insert(scalar).unwrap();
    assert!(matches!(
        Resolver::new(invalid_removal, LayerOrder::new(["base"]).unwrap()),
        Err(ResolveError::InvalidSchema(_))
    ));
}

#[test]
fn definition_validation_rejects_semantically_invalid_conditions_and_constraints() {
    let mut invalid_condition = Schema::new();
    invalid_condition
        .insert(SettingSpec::typed("enabled", ValueType::Boolean))
        .unwrap();
    let mut dependent = SettingSpec::typed("value", ValueType::Text);
    dependent.requirement = Requirement::RequiredWhen(Condition::Equals {
        path: "enabled".into(),
        value: json!("yes"),
    });
    invalid_condition.insert(dependent).unwrap();
    assert!(matches!(
        Resolver::new(invalid_condition, LayerOrder::new(["base"]).unwrap()),
        Err(ResolveError::InvalidSchema(_))
    ));

    let mut invalid_constraint = Schema::new();
    let mut text = SettingSpec::typed("name", ValueType::Text);
    text.constraints.push(Constraint::UnsignedMin(1));
    invalid_constraint.insert(text).unwrap();
    assert!(matches!(
        Resolver::new(invalid_constraint, LayerOrder::new(["base"]).unwrap()),
        Err(ResolveError::InvalidSchema(_))
    ));

    let mut invalid_default = Schema::new();
    let mut port = SettingSpec::typed("port", ValueType::UnsignedInteger);
    port.constraints.push(Constraint::UnsignedMin(1));
    port.default = Some(DefaultRule::Fixed(json!(0)));
    invalid_default.insert(port).unwrap();
    assert!(matches!(
        Resolver::new(invalid_default, LayerOrder::new(["base"]).unwrap()),
        Err(ResolveError::InvalidSchema(_))
    ));
}

#[test]
fn definition_validation_rejects_null_equality_invalid_choices_ranges_and_nonfinite_bounds() {
    let mut null_condition = Schema::new();
    null_condition
        .insert(SettingSpec::typed("flag", ValueType::Boolean))
        .unwrap();
    let mut dependent = SettingSpec::typed("dependent", ValueType::Text);
    dependent.requirement = Requirement::RequiredWhen(Condition::Equals {
        path: "flag".into(),
        value: serde_json::Value::Null,
    });
    null_condition.insert(dependent).unwrap();
    assert!(matches!(
        Resolver::new(null_condition, LayerOrder::new(["base"]).unwrap()),
        Err(ResolveError::InvalidSchema(_))
    ));

    for choices in [Vec::<String>::new(), vec!["a".into(), "a".into()]] {
        let mut schema = Schema::new();
        let mut mode = SettingSpec::typed("mode", ValueType::Text);
        mode.constraints.push(Constraint::OneOf(choices));
        schema.insert(mode).unwrap();
        assert!(matches!(
            Resolver::new(schema, LayerOrder::new(["base"]).unwrap()),
            Err(ResolveError::InvalidSchema(_))
        ));
    }

    let mut contradictory = Schema::new();
    let mut workers = SettingSpec::typed("workers", ValueType::UnsignedInteger);
    workers.constraints.push(Constraint::UnsignedMin(10));
    workers.constraints.push(Constraint::UnsignedMax(2));
    contradictory.insert(workers).unwrap();
    assert!(matches!(
        Resolver::new(contradictory, LayerOrder::new(["base"]).unwrap()),
        Err(ResolveError::InvalidSchema(_))
    ));

    let mut nonfinite = Schema::new();
    let mut ratio = SettingSpec::typed("ratio", ValueType::Float);
    ratio.constraints.push(Constraint::FloatMin(f64::NAN));
    nonfinite.insert(ratio).unwrap();
    assert!(matches!(
        Resolver::new(nonfinite, LayerOrder::new(["base"]).unwrap()),
        Err(ResolveError::InvalidSchema(_))
    ));
}

#[test]
fn rejected_duplicate_schema_insert_does_not_mutate_the_existing_definition() {
    let mut schema = Schema::new();
    let original = SettingSpec::typed("port", ValueType::UnsignedInteger);
    schema.insert(original.clone()).unwrap();

    let replacement = SettingSpec::typed("port", ValueType::Text);
    assert!(matches!(
        schema.insert(replacement),
        Err(ResolveError::InvalidSchema(_))
    ));
    assert_eq!(schema.get("port"), Some(&original));
}

#[test]
fn definition_paths_require_nonempty_trimmed_segments() {
    for invalid in [
        "",
        ".server",
        "server.",
        "server..port",
        "server. port",
        "server.port ",
    ] {
        let mut schema = Schema::new();
        let result = schema.insert(SettingSpec::typed(invalid, ValueType::UnsignedInteger));
        assert!(matches!(result, Err(ResolveError::InvalidSchema(_))));
        assert!(schema.iter().next().is_none());
    }

    let mut schema = Schema::new();
    schema
        .insert(SettingSpec::typed(
            "server.http-port",
            ValueType::UnsignedInteger,
        ))
        .unwrap();
    assert!(schema.get("server.http-port").is_some());
}

#[test]
fn schema_rejects_setting_paths_that_overlap_by_ancestry_in_either_order() {
    for (first, second) in [("service", "service.port"), ("service.port", "service")] {
        let mut schema = Schema::new();
        schema
            .insert(SettingSpec::typed(first, ValueType::Text))
            .unwrap();
        let error = schema
            .insert(SettingSpec::typed(second, ValueType::UnsignedInteger))
            .unwrap_err();
        let rendered = error.to_string();
        assert!(rendered.contains("`service`"));
        assert!(rendered.contains("`service.port`"));
        assert!(rendered.contains("overlap by ancestry"));
        assert!(schema.get(first).is_some());
        assert!(schema.get(second).is_none());
    }
}

#[test]
fn resolver_revalidates_deserialized_schema_structure() {
    let mut schema = Schema::new();
    schema
        .insert(SettingSpec::typed(
            "service.port",
            ValueType::UnsignedInteger,
        ))
        .unwrap();
    let mut encoded = serde_json::to_value(schema).unwrap();
    let settings = encoded["settings"].as_object_mut().unwrap();
    let setting = settings.remove("service.port").unwrap();
    settings.insert("different.key".into(), setting);
    let deserialized: Schema = serde_json::from_value(encoded).unwrap();

    let error = match Resolver::new(deserialized, LayerOrder::new(["base"]).unwrap()) {
        Ok(_) => panic!("deserialized schema structure must be revalidated"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("does not match setting path"));
}

#[test]
fn schema_prefixes_are_validated_before_a_schema_is_returned() {
    let mut schema = Schema::new();
    schema
        .insert(SettingSpec::typed("port", ValueType::UnsignedInteger))
        .unwrap();

    assert!(schema.prefixed("service").is_ok());
    assert!(matches!(
        schema.prefixed("service..invalid"),
        Err(ResolveError::InvalidSchema(_))
    ));
}

#[test]
fn any_shape_rejects_structural_merge_and_removal_policies() {
    for merge in [
        MergePolicy::Deep,
        MergePolicy::Append,
        MergePolicy::Prepend,
        MergePolicy::CombineByKey,
    ] {
        let mut schema = Schema::new();
        let mut setting = SettingSpec::new("dynamic");
        setting.shape = ValueShape::Any;
        setting.merge = merge;
        schema.insert(setting).unwrap();
        assert!(matches!(
            Resolver::new(schema, LayerOrder::new(["base"]).unwrap()),
            Err(ResolveError::InvalidSchema(_))
        ));
    }

    for removal in [
        RemovalPolicy::unset_only().with_clear(),
        RemovalPolicy::unset_only().with_map_entry_removal(),
        RemovalPolicy::unset_only().with_list_item_removal(),
    ] {
        let mut schema = Schema::new();
        let mut setting = SettingSpec::new("dynamic");
        setting.shape = ValueShape::Any;
        setting.removal = removal;
        schema.insert(setting).unwrap();
        assert!(matches!(
            Resolver::new(schema, LayerOrder::new(["base"]).unwrap()),
            Err(ResolveError::InvalidSchema(_))
        ));
    }
}

#[test]
fn resolver_enforces_logical_input_resource_limits() {
    let mut schema = Schema::new();
    schema.insert(SettingSpec::new("payload")).unwrap();
    let limits = ResolutionLimits {
        max_value_depth: 2,
        ..ResolutionLimits::default()
    };
    let resolver =
        Resolver::with_limits(schema, LayerOrder::new(["base"]).unwrap(), limits).unwrap();
    let input = LogicalInput::new("nested", "base", Origin::new("test")).push(Operation::Set {
        path: "payload".into(),
        value: json!({"level": {"too_deep": true}}),
    });

    assert!(matches!(
        resolver.resolve(&Context::new(), &[input]),
        Err(ResolveError::LimitExceeded { .. })
    ));

    let mut schema = Schema::new();
    schema.insert(SettingSpec::new("payload")).unwrap();
    let limits = ResolutionLimits {
        max_scalar_bytes: 64,
        max_logical_bytes: 16,
        ..ResolutionLimits::default()
    };
    let resolver =
        Resolver::with_limits(schema, LayerOrder::new(["base"]).unwrap(), limits).unwrap();
    let input =
        LogicalInput::new("metadata", "base", Origin::new("logical source")).push(Operation::Set {
            path: "payload".into(),
            value: json!(true),
        });
    assert!(matches!(
        resolver.resolve(&Context::new(), &[input]),
        Err(ResolveError::LimitExceeded { resource, .. })
            if resource == "aggregate logical bytes"
    ));
}
