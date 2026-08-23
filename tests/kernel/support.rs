use configlab::{LogicalInput, Operation, Origin};

pub(crate) fn set_input(path: &str, value: serde_json::Value) -> LogicalInput {
    LogicalInput::new("input", "base", Origin::new("test")).push(Operation::Set {
        path: path.to_string(),
        value,
    })
}
