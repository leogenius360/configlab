use configlab::*;
use serde_json::json;

pub(crate) fn schema() -> Schema {
    let mut schema = Schema::new();
    let mut timeout = SettingSpec::typed("server.timeout", ValueType::Integer);
    timeout.requirement = Requirement::Required;
    schema.insert(timeout).unwrap();

    let mut includes = SettingSpec::typed("compiler.includes", ValueType::Text);
    includes.shape = ValueShape::OrderedList;
    includes.merge = MergePolicy::Append;
    includes.default = Some(DefaultRule::Fixed(json!([])));
    schema.insert(includes).unwrap();

    let mut password = SettingSpec::typed("database.password", ValueType::Text);
    password.disclosure = Disclosure::Sensitive;
    password.representation = Representation::SecretReference;
    password.requirement = Requirement::Required;
    schema.insert(password).unwrap();
    schema
}

pub(crate) fn input(
    id: &str,
    layer: &str,
    selector: Selector,
    path: &str,
    value: serde_json::Value,
) -> LogicalInput {
    LogicalInput::new(id, layer, Origin::new(id))
        .when(selector)
        .push(Operation::Set {
            path: path.into(),
            value,
        })
}
