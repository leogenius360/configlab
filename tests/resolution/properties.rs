use configlab::*;
use proptest::collection::{btree_map, vec};
use proptest::prelude::*;
use serde_json::{Map, Value};

fn json_tree() -> BoxedStrategy<Value> {
    prop_oneof![
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(|value| serde_json::json!(value)),
        "[a-z0-9]{0,8}".prop_map(Value::String),
        Just(Value::Null),
    ]
    .prop_recursive(3, 64, 8, |inner| {
        btree_map("[a-z]{1,5}", inner, 0..5)
            .prop_map(|entries| Value::Object(entries.into_iter().collect::<Map<_, _>>()))
    })
    .boxed()
}

fn json_object() -> BoxedStrategy<Value> {
    btree_map("[a-z]{1,5}", json_tree(), 0..6)
        .prop_map(|entries| Value::Object(entries.into_iter().collect::<Map<_, _>>()))
        .boxed()
}

fn structured_resolver(policy: MergePolicy) -> Resolver {
    let mut schema = Schema::new();
    let mut setting = SettingSpec::new("value");
    setting.shape = match policy {
        MergePolicy::CombineByKey => ValueShape::NamedMap,
        MergePolicy::Deep | MergePolicy::Replace => ValueShape::Object,
        MergePolicy::Append | MergePolicy::Prepend => unreachable!(),
    };
    setting.merge = policy;
    schema.insert(setting).unwrap();
    Resolver::new(schema, LayerOrder::new(["base"]).unwrap()).unwrap()
}

fn input(id: &str, value: Value) -> LogicalInput {
    LogicalInput::new(id, "base", Origin::new(format!("property source {id}"))).push(
        Operation::Set {
            path: "value".into(),
            value,
        },
    )
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 192,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn structured_resolution_is_invariant_under_input_permutation(
        left in json_object(),
        right in json_object(),
        policy in prop_oneof![
            Just(MergePolicy::Replace),
            Just(MergePolicy::Deep),
            Just(MergePolicy::CombineByKey),
        ],
    ) {
        let resolver = structured_resolver(policy);
        let left = input("left", left);
        let right = input("right", right);

        let forward = resolver.resolve(&Context::new(), &[left.clone(), right.clone()]);
        let reverse = resolver.resolve(&Context::new(), &[right, left]);

        prop_assert_eq!(forward, reverse);
    }

    #[test]
    fn explicitly_ordered_accumulation_is_invariant_under_input_permutation(
        left in vec(any::<i16>(), 0..8),
        right in vec(any::<i16>(), 0..8),
    ) {
        let mut schema = Schema::new();
        let mut setting = SettingSpec::new("value");
        setting.shape = ValueShape::OrderedList;
        setting.merge = MergePolicy::Append;
        schema.insert(setting).unwrap();
        let resolver = Resolver::new(schema, LayerOrder::new(["base"]).unwrap()).unwrap();

        let left = input("left", serde_json::json!(left)).ordered(10);
        let right = input("right", serde_json::json!(right)).ordered(20);
        let forward = resolver.resolve(&Context::new(), &[left.clone(), right.clone()]);
        let reverse = resolver.resolve(&Context::new(), &[right, left]);

        prop_assert_eq!(forward, reverse);
    }
}
