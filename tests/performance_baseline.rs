//! Manual release-mode performance baseline.
//!
//! This test is ignored during the ordinary behavioral suite and is executed
//! explicitly by `script/performance-baseline.sh`. It intentionally has no
//! timing threshold: its purpose is to record comparable numbers before any
//! performance optimization is proposed, not to make correctness depend on a
//! noisy wall-clock measurement.

use configlab::{
    Config, ConfigError, Context, DefaultRule, EffectiveConfiguration, LayerOrder, LiveConfig,
    LogicalInput, Operation, Origin, Resolver, Schema, SettingSpec, ValueType,
};
use serde_json::json;
use std::hint::black_box;
use std::time::Instant;

#[test]
#[ignore = "manual release-mode measurement; run script/performance-baseline.sh"]
fn resolves_five_layers_and_one_hundred_settings() {
    const SETTINGS: usize = 100;
    const ITERATIONS: usize = 1_000;

    let mut schema = Schema::new();
    for index in 0..SETTINGS {
        schema
            .insert(SettingSpec::typed(
                format!("service.value_{index}"),
                ValueType::UnsignedInteger,
            ))
            .unwrap();
    }

    let layers =
        LayerOrder::new(["defaults", "base", "contextual", "environment", "override"]).unwrap();
    let resolver = Resolver::new(schema, layers).unwrap();

    let make_input = |id: &str, layer: &str, value: u64| {
        let mut input = LogicalInput::new(id, layer, Origin::new("performance-baseline"));
        for index in 0..SETTINGS {
            input = input.push(Operation::Set {
                path: format!("service.value_{index}"),
                value: json!(value + index as u64),
            });
        }
        input
    };

    let inputs = vec![
        make_input("base", "base", 1_000),
        make_input("contextual", "contextual", 2_000),
        make_input("environment", "environment", 3_000),
        make_input("override", "override", 4_000),
    ];
    let context = Context::new();

    // Warm the allocator/code paths before timing.
    for _ in 0..20 {
        resolver.resolve(&context, &inputs).unwrap();
    }

    let started = Instant::now();
    for _ in 0..ITERATIONS {
        resolver.resolve(&context, &inputs).unwrap();
    }
    let elapsed = started.elapsed();
    let per_resolution = elapsed / ITERATIONS as u32;

    println!(
        "configlab performance baseline: settings={SETTINGS} inputs={} iterations={ITERATIONS} total={elapsed:?} per_resolution={per_resolution:?}",
        inputs.len()
    );
}

struct RuntimePerformanceConfig {
    values: Vec<u64>,
}

impl Config for RuntimePerformanceConfig {
    fn schema(prefix: &str) -> Result<Schema, ConfigError> {
        let mut schema = Schema::new();
        for index in 0..100 {
            let path = if prefix.is_empty() {
                format!("service.value_{index}")
            } else {
                format!("{prefix}.service.value_{index}")
            };
            let mut setting = SettingSpec::typed(path, ValueType::UnsignedInteger);
            setting.default = Some(DefaultRule::Fixed(json!(0)));
            schema.insert(setting)?;
        }
        Ok(schema)
    }

    fn decode(effective: &EffectiveConfiguration, prefix: &str) -> Result<Self, ConfigError> {
        let mut values = Vec::with_capacity(100);
        for index in 0..100 {
            let path = if prefix.is_empty() {
                format!("service.value_{index}")
            } else {
                format!("{prefix}.service.value_{index}")
            };
            let value = effective
                .get(&path)
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| ConfigError::Decode {
                    path,
                    message: "expected unsigned integer".into(),
                })?;
            values.push(value);
        }
        Ok(Self { values })
    }
}

#[test]
#[ignore = "manual release-mode measurement; run script/performance-baseline.sh"]
fn refreshes_live_configuration_for_one_hundred_settings() {
    const SETTINGS: usize = 100;
    const ITERATIONS: usize = 500;

    let mut config = RuntimePerformanceConfig::builder().live().unwrap();

    let evaluate = |config: &mut LiveConfig<RuntimePerformanceConfig>, iteration: usize| {
        let mut input = LogicalInput::new("runtime", "override", Origin::new("benchmark"));
        for index in 0..SETTINGS {
            input = input.push(Operation::Set {
                path: format!("service.value_{index}"),
                value: json!(1_000 + index + iteration % 2),
            });
        }
        let report = config.reconcile([input]).unwrap();
        black_box(config.value().values.len());
        black_box(report.changes().len());
    };

    for iteration in 0..20 {
        evaluate(&mut config, iteration);
    }
    let started = Instant::now();
    for iteration in 20..(ITERATIONS + 20) {
        evaluate(&mut config, iteration);
    }
    let elapsed = started.elapsed();
    let per_refresh = elapsed / ITERATIONS as u32;

    println!(
        "configlab live-refresh baseline: settings={SETTINGS} iterations={ITERATIONS} total={elapsed:?} per_refresh={per_refresh:?}"
    );
}
