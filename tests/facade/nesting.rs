use configlab::*;

#[test]
fn nested_optional_null_and_group_unset_work() {
    #[derive(Debug, Config)]
    struct Child {
        #[config(default = 42)]
        answer: u16,
    }
    #[derive(Debug, Config)]
    struct Parent {
        child: Option<Child>,
    }

    let absent = Parent::builder().load().unwrap();
    assert!(absent.child.is_none());
    let present = Parent::builder()
        .inline(json("present.json", r#"{"child":{}}"#))
        .load()
        .unwrap();
    assert_eq!(present.child.unwrap().answer, 42);
    let nulled = Parent::builder()
        .inline(json("base.json", r#"{"child":{"answer":7}}"#))
        .inline(json("null.json", r#"{"child":null}"#).layer("contextual"))
        .load()
        .unwrap();
    assert!(nulled.child.is_none());
    let unset = Parent::builder()
        .inline(json("base.json", r#"{"child":{"answer":7}}"#))
        .unset("child")
        .load()
        .unwrap();
    assert!(unset.child.is_none());
}

#[test]
fn unknown_group_unset_is_not_silently_ignored() {
    #[derive(Debug, Config)]
    struct App {
        #[config(default = 42)]
        answer: u16,
    }

    let error = App::builder()
        .unset("missing_group")
        .load()
        .expect_err("an unknown unset target must reach resolver validation");

    assert!(matches!(
        error,
        ConfigError::Resolve(ResolveError::UnknownSetting(path)) if path == "missing_group"
    ));
}

#[test]
fn facade_hides_optional_group_bookkeeping_from_public_resolution_metadata() {
    #[derive(Debug, Config)]
    struct Child {
        #[config(default = 7)]
        value: u8,
    }

    #[derive(Debug, Config)]
    struct Parent {
        child: Option<Child>,
    }

    let loaded = Parent::builder().resolve().unwrap();
    assert!(loaded.child.is_none());
    assert!(
        loaded
            .information()
            .iter()
            .all(|information| !format!("{information:?}").contains("__config_present"))
    );
    assert!(
        !loaded
            .redacted_value()
            .to_string()
            .contains("__config_present")
    );
}

#[test]
fn required_optional_nested_section_reports_the_public_group_path() {
    #[derive(Debug, Config)]
    struct Child {
        #[config(default = 7)]
        value: u8,
    }

    #[derive(Debug, Config)]
    struct Parent {
        #[config(required)]
        child: Option<Child>,
    }

    let error = Parent::builder()
        .load()
        .expect_err("required child must fail");
    assert!(matches!(
        error,
        ConfigError::Resolve(configlab::ResolveError::MissingRequired(path)) if path == "child"
    ));
}
