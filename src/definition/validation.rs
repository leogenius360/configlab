//! Definition-time invariant validation and value checking helpers.

use super::rules::{Condition, Constraint, Requirement};
use super::schema::Schema;
use super::setting::SettingSpec;
use super::value::{MergePolicy, Representation, ValueShape, ValueType};
use crate::{ResolveError, get_path};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::BTreeSet;

pub(super) fn validate_shape_policy(setting: &SettingSpec) -> Result<(), ResolveError> {
    let compatible = match setting.merge {
        MergePolicy::Replace => true,
        MergePolicy::Deep => matches!(setting.shape, ValueShape::Object | ValueShape::NamedMap),
        MergePolicy::Append | MergePolicy::Prepend => setting.shape == ValueShape::OrderedList,
        MergePolicy::CombineByKey => setting.shape == ValueShape::NamedMap,
    };

    if compatible {
        Ok(())
    } else {
        Err(ResolveError::InvalidSchema(format!(
            "merge policy {:?} is incompatible with shape {:?} for `{}`",
            setting.merge, setting.shape, setting.path
        )))
    }
}

pub(super) fn validate_removal_policy(setting: &SettingSpec) -> Result<(), ResolveError> {
    if setting.removal.clear
        && !matches!(
            setting.shape,
            ValueShape::Object | ValueShape::NamedMap | ValueShape::OrderedList
        )
    {
        return Err(ResolveError::InvalidSchema(format!(
            "clear removal is incompatible with shape {:?} for `{}`",
            setting.shape, setting.path
        )));
    }
    if setting.removal.remove_map_entry && setting.shape != ValueShape::NamedMap {
        return Err(ResolveError::InvalidSchema(format!(
            "map-entry removal is incompatible with shape {:?} for `{}`",
            setting.shape, setting.path
        )));
    }
    if setting.removal.remove_list_item && setting.shape != ValueShape::OrderedList {
        return Err(ResolveError::InvalidSchema(format!(
            "list-item removal is incompatible with shape {:?} for `{}`",
            setting.shape, setting.path
        )));
    }
    Ok(())
}

pub(super) fn validate_requirement(
    schema: &Schema,
    requirement: &Requirement,
    setting_path: &str,
) -> Result<(), ResolveError> {
    if let Requirement::RequiredWhen(condition) = requirement {
        validate_condition(
            schema,
            condition,
            &format!("requirement for `{setting_path}`"),
        )?;
    }
    Ok(())
}

pub(super) fn validate_condition(
    schema: &Schema,
    condition: &Condition,
    owner: &str,
) -> Result<(), ResolveError> {
    match condition {
        Condition::Equals { path, value } => {
            let setting = condition_setting(schema, path, owner)?;
            if value.is_null() && !setting.nullable {
                return Err(ResolveError::InvalidSchema(format!(
                    "{owner} compares non-nullable setting `{path}` with null"
                )));
            }
            if !value.is_null()
                && (!value_matches_shape(setting.shape, value)
                    || !value_matches_type(setting, value))
            {
                return Err(ResolveError::InvalidSchema(format!(
                    "{owner} compares `{path}` with a value incompatible with its definition"
                )));
            }
        }
        Condition::Present { path } => {
            condition_setting(schema, path, owner)?;
        }
        Condition::Contains { path, value } => {
            let setting = condition_setting(schema, path, owner)?;
            match setting.shape {
                ValueShape::OrderedList => {
                    if setting.value_type != ValueType::Any
                        && !scalar_matches_type(&setting.value_type, value)
                    {
                        return Err(ResolveError::InvalidSchema(format!(
                            "{owner} uses a contains value incompatible with `{path}`"
                        )));
                    }
                }
                ValueShape::Object | ValueShape::NamedMap => {
                    if !value.is_string() {
                        return Err(ResolveError::InvalidSchema(format!(
                            "{owner} must use a string key when testing object/map `{path}`"
                        )));
                    }
                }
                ValueShape::Any => {}
                ValueShape::Scalar => {
                    return Err(ResolveError::InvalidSchema(format!(
                        "{owner} cannot use contains with scalar setting `{path}`"
                    )));
                }
            }
        }
        Condition::LessThan { left, right } | Condition::LessOrEqual { left, right } => {
            let left_setting = condition_setting(schema, left, owner)?;
            let right_setting = condition_setting(schema, right, owner)?;
            if !is_numeric_setting(left_setting) || !is_numeric_setting(right_setting) {
                return Err(ResolveError::InvalidSchema(format!(
                    "{owner} numeric comparison requires numeric scalar settings `{left}` and `{right}`"
                )));
            }
        }
        Condition::All(conditions) | Condition::Any(conditions) => {
            if conditions.is_empty() {
                return Err(ResolveError::InvalidSchema(format!(
                    "{owner} cannot use an empty composite condition"
                )));
            }
            for child in conditions {
                validate_condition(schema, child, owner)?;
            }
        }
        Condition::Not(condition) => validate_condition(schema, condition, owner)?,
    }
    Ok(())
}

fn condition_setting<'a>(
    schema: &'a Schema,
    path: &str,
    owner: &str,
) -> Result<&'a SettingSpec, ResolveError> {
    schema.settings.get(path).ok_or_else(|| {
        ResolveError::InvalidSchema(format!("{owner} references unknown setting `{path}`"))
    })
}

fn is_numeric_setting(setting: &SettingSpec) -> bool {
    matches!(setting.shape, ValueShape::Scalar | ValueShape::Any)
        && matches!(
            &setting.value_type,
            ValueType::Integer | ValueType::UnsignedInteger | ValueType::Float | ValueType::Any
        )
}

pub(super) fn validate_value_type_shape(setting: &SettingSpec) -> Result<(), ResolveError> {
    if setting.shape == ValueShape::Object && setting.value_type != ValueType::Any {
        return Err(ResolveError::InvalidSchema(format!(
            "object setting `{}` must use ValueType::Any because object fields have independent types",
            setting.path
        )));
    }
    if let ValueType::Enumeration(variants) = &setting.value_type {
        if variants.is_empty() {
            return Err(ResolveError::InvalidSchema(format!(
                "enumeration setting `{}` must declare at least one variant",
                setting.path
            )));
        }
        let mut seen = BTreeSet::new();
        if variants
            .iter()
            .any(|variant| variant.is_empty() || !seen.insert(variant))
        {
            return Err(ResolveError::InvalidSchema(format!(
                "enumeration setting `{}` contains empty or duplicate variants",
                setting.path
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_representation_type(setting: &SettingSpec) -> Result<(), ResolveError> {
    if setting.representation == Representation::Ordinary {
        return Ok(());
    }
    let text_like = matches!(
        &setting.value_type,
        ValueType::Any | ValueType::Text | ValueType::Path | ValueType::Address
    );
    if !matches!(setting.shape, ValueShape::Scalar | ValueShape::Any) || !text_like {
        return Err(ResolveError::InvalidSchema(format!(
            "reference representation for `{}` requires a text-like scalar value",
            setting.path
        )));
    }
    Ok(())
}

pub(super) fn validate_constraint_definitions(setting: &SettingSpec) -> Result<(), ResolveError> {
    let mut bounds = ConstraintBounds::default();

    for constraint in &setting.constraints {
        validate_constraint_payload(setting, constraint)?;
        bounds.include(constraint);
        if !constraint_is_compatible(setting, constraint) {
            return Err(ResolveError::InvalidSchema(format!(
                "constraint {constraint:?} is incompatible with `{}` ({:?}, {:?})",
                setting.path, setting.shape, setting.value_type
            )));
        }
    }

    if bounds.has_empty_range() {
        return Err(ResolveError::InvalidSchema(format!(
            "constraints for `{}` define an empty range",
            setting.path
        )));
    }

    Ok(())
}

fn validate_constraint_payload(
    setting: &SettingSpec,
    constraint: &Constraint,
) -> Result<(), ResolveError> {
    match constraint {
        Constraint::FloatMin(value) | Constraint::FloatMax(value) if !value.is_finite() => {
            Err(ResolveError::InvalidSchema(format!(
                "float constraint for `{}` must be finite",
                setting.path
            )))
        }
        Constraint::OneOf(items) if items.is_empty() => Err(ResolveError::InvalidSchema(format!(
            "one-of constraint for `{}` must contain at least one value",
            setting.path
        ))),
        Constraint::OneOf(items) => {
            let mut seen = BTreeSet::new();
            if items.iter().any(|item| !seen.insert(item)) {
                return Err(ResolveError::InvalidSchema(format!(
                    "one-of constraint for `{}` contains duplicate values",
                    setting.path
                )));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn constraint_is_compatible(setting: &SettingSpec, constraint: &Constraint) -> bool {
    let scalar = matches!(setting.shape, ValueShape::Scalar | ValueShape::Any);
    match constraint {
        Constraint::IntegerMin(_) | Constraint::IntegerMax(_) => {
            scalar && matches!(&setting.value_type, ValueType::Integer | ValueType::Any)
        }
        Constraint::UnsignedMin(_) | Constraint::UnsignedMax(_) => {
            scalar
                && matches!(
                    &setting.value_type,
                    ValueType::UnsignedInteger | ValueType::Any
                )
        }
        Constraint::FloatMin(_) | Constraint::FloatMax(_) => {
            scalar && matches!(&setting.value_type, ValueType::Float | ValueType::Any)
        }
        Constraint::NonEmpty => {
            matches!(
                setting.shape,
                ValueShape::OrderedList | ValueShape::Object | ValueShape::NamedMap
            ) || matches!(
                &setting.value_type,
                ValueType::Any
                    | ValueType::Text
                    | ValueType::Duration
                    | ValueType::ByteSize
                    | ValueType::Path
                    | ValueType::Address
                    | ValueType::Enumeration(_)
            )
        }
        Constraint::OneOf(_) => {
            scalar
                && matches!(
                    &setting.value_type,
                    ValueType::Any | ValueType::Text | ValueType::Enumeration(_)
                )
        }
    }
}

#[derive(Default)]
struct ConstraintBounds {
    integer_min: Option<i128>,
    integer_max: Option<i128>,
    unsigned_min: Option<u128>,
    unsigned_max: Option<u128>,
    float_min: Option<f64>,
    float_max: Option<f64>,
}

impl ConstraintBounds {
    fn include(&mut self, constraint: &Constraint) {
        match constraint {
            Constraint::IntegerMin(value) => {
                self.integer_min = Some(
                    self.integer_min
                        .map_or(*value, |current| current.max(*value)),
                );
            }
            Constraint::IntegerMax(value) => {
                self.integer_max = Some(
                    self.integer_max
                        .map_or(*value, |current| current.min(*value)),
                );
            }
            Constraint::UnsignedMin(value) => {
                self.unsigned_min = Some(
                    self.unsigned_min
                        .map_or(*value, |current| current.max(*value)),
                );
            }
            Constraint::UnsignedMax(value) => {
                self.unsigned_max = Some(
                    self.unsigned_max
                        .map_or(*value, |current| current.min(*value)),
                );
            }
            Constraint::FloatMin(value) => {
                self.float_min = Some(self.float_min.map_or(*value, |current| current.max(*value)));
            }
            Constraint::FloatMax(value) => {
                self.float_max = Some(self.float_max.map_or(*value, |current| current.min(*value)));
            }
            Constraint::NonEmpty | Constraint::OneOf(_) => {}
        }
    }

    fn has_empty_range(&self) -> bool {
        self.integer_min
            .zip(self.integer_max)
            .is_some_and(|(min, max)| min > max)
            || self
                .unsigned_min
                .zip(self.unsigned_max)
                .is_some_and(|(min, max)| min > max)
            || self
                .float_min
                .zip(self.float_max)
                .is_some_and(|(min, max)| min > max)
    }
}

/// Returns whether a definition path contains only non-empty, trimmed dot-separated segments.
///
/// Definition paths are logical setting/component paths, not filesystem paths.
/// Whitespace at segment boundaries is rejected so path identity is stable and
/// cannot differ only by invisible leading or trailing spaces.
pub(super) fn valid_definition_path(path: &str) -> bool {
    !path.is_empty()
        && path
            .split('.')
            .all(|segment| !segment.is_empty() && segment.trim() == segment)
}

/// Reads a JSON number as a signed 128-bit integer when it is integral and in range.
///
/// `serde_json::Value` exposes number access through `Number`; the 128-bit
/// conversion methods live on `serde_json::Number`, not on `Value`.
fn json_i128(value: &Value) -> Option<i128> {
    value.as_number().and_then(serde_json::Number::as_i128)
}

/// Reads a JSON number as an unsigned 128-bit integer when it is integral and in range.
fn json_u128(value: &Value) -> Option<u128> {
    value.as_number().and_then(serde_json::Number::as_u128)
}

pub(super) fn validate_definition_value(
    setting: &SettingSpec,
    value: &Value,
) -> Result<(), String> {
    if value.is_null() {
        if setting.nullable {
            return Ok(());
        }
        return Err("null is not permitted".into());
    }

    if !value_matches_shape(setting.shape, value) {
        return Err(format!(
            "value does not match declared shape {:?}",
            setting.shape
        ));
    }
    if !value_matches_type(setting, value) {
        return Err(format!(
            "value does not match declared type {:?}",
            setting.value_type
        ));
    }
    if !value_matches_representation(setting, value) {
        return Err(format!(
            "value does not match representation {:?}",
            setting.representation
        ));
    }
    for constraint in &setting.constraints {
        if let Some(message) = constraint_violation(constraint, value) {
            return Err(message);
        }
    }
    Ok(())
}

/// Returns whether a value conforms to a structural setting shape.
pub(crate) fn value_matches_shape(shape: ValueShape, value: &Value) -> bool {
    if value.is_null() {
        return true;
    }
    match shape {
        ValueShape::Scalar => !value.is_array() && !value.is_object(),
        ValueShape::Object | ValueShape::NamedMap => value.is_object(),
        ValueShape::OrderedList => value.is_array(),
        ValueShape::Any => true,
    }
}

/// Returns whether a setting value and its collection elements conform to the
/// declared semantic type.
pub(crate) fn value_matches_type(setting: &SettingSpec, value: &Value) -> bool {
    if value.is_null() || setting.value_type == ValueType::Any {
        return true;
    }

    match setting.shape {
        ValueShape::OrderedList => value.as_array().is_some_and(|items| {
            items
                .iter()
                .all(|item| scalar_matches_type(&setting.value_type, item))
        }),
        ValueShape::NamedMap => value.as_object().is_some_and(|map| {
            map.values()
                .all(|item| scalar_matches_type(&setting.value_type, item))
        }),
        ValueShape::Object => setting.value_type == ValueType::Any,
        ValueShape::Scalar | ValueShape::Any => scalar_matches_type(&setting.value_type, value),
    }
}

pub(crate) fn value_matches_representation(setting: &SettingSpec, value: &Value) -> bool {
    if value.is_null() {
        return true;
    }
    match setting.representation {
        Representation::Ordinary => true,
        Representation::ExternalReference | Representation::SecretReference => value.is_string(),
    }
}

fn scalar_matches_type(value_type: &ValueType, value: &Value) -> bool {
    match value_type {
        ValueType::Any => true,
        ValueType::Boolean => value.is_boolean(),
        ValueType::Integer => json_i128(value).is_some(),
        ValueType::UnsignedInteger => json_u128(value).is_some(),
        ValueType::Float => value.as_f64().is_some(),
        ValueType::Text
        | ValueType::Duration
        | ValueType::ByteSize
        | ValueType::Path
        | ValueType::Address => value.is_string(),
        ValueType::Enumeration(variants) => value
            .as_str()
            .is_some_and(|candidate| variants.iter().any(|variant| variant == candidate)),
    }
}

pub(crate) fn constraint_violation(constraint: &Constraint, value: &Value) -> Option<String> {
    match constraint {
        Constraint::IntegerMin(min)
            if json_i128(value).is_some_and(|candidate| candidate < *min) =>
        {
            Some(format!("must be >= {min}"))
        }
        Constraint::IntegerMax(max)
            if json_i128(value).is_some_and(|candidate| candidate > *max) =>
        {
            Some(format!("must be <= {max}"))
        }
        Constraint::UnsignedMin(min)
            if json_u128(value).is_some_and(|candidate| candidate < *min) =>
        {
            Some(format!("must be >= {min}"))
        }
        Constraint::UnsignedMax(max)
            if json_u128(value).is_some_and(|candidate| candidate > *max) =>
        {
            Some(format!("must be <= {max}"))
        }
        Constraint::FloatMin(min) if value.as_f64().is_some_and(|candidate| candidate < *min) => {
            Some(format!("must be >= {min}"))
        }
        Constraint::FloatMax(max) if value.as_f64().is_some_and(|candidate| candidate > *max) => {
            Some(format!("must be <= {max}"))
        }
        Constraint::NonEmpty if value_is_empty(value) => Some("must not be empty".into()),
        Constraint::OneOf(items)
            if value
                .as_str()
                .is_some_and(|candidate| !items.iter().any(|item| item == candidate)) =>
        {
            Some(format!("must be one of {}", items.join(", ")))
        }
        _ => None,
    }
}

fn value_is_empty(value: &Value) -> bool {
    match value {
        Value::String(value) => value.is_empty(),
        Value::Array(value) => value.is_empty(),
        Value::Object(value) => value.is_empty(),
        _ => false,
    }
}

pub(super) fn compare_paths(values: &Value, left: &str, right: &str) -> Option<Ordering> {
    let left = get_path(values, left)?;
    let right = get_path(values, right)?;
    compare_numbers(left, right)
}

fn compare_numbers(left: &Value, right: &Value) -> Option<Ordering> {
    if let (Some(left), Some(right)) = (json_i128(left), json_i128(right)) {
        return Some(left.cmp(&right));
    }
    if let (Some(left), Some(right)) = (json_u128(left), json_u128(right)) {
        return Some(left.cmp(&right));
    }
    if let (Some(left), Some(right)) = (json_i128(left), json_u128(right)) {
        return if left < 0 {
            Some(Ordering::Less)
        } else {
            Some((left as u128).cmp(&right))
        };
    }
    if let (Some(left), Some(right)) = (json_u128(left), json_i128(right)) {
        return if right < 0 {
            Some(Ordering::Greater)
        } else {
            Some(left.cmp(&(right as u128)))
        };
    }

    if let Some(left) = json_i128(left) {
        return compare_i128_f64(left, right.as_f64()?);
    }
    if let Some(left) = json_u128(left) {
        return compare_u128_f64(left, right.as_f64()?);
    }
    if let Some(right) = json_i128(right) {
        return compare_i128_f64(right, left.as_f64()?).map(Ordering::reverse);
    }
    if let Some(right) = json_u128(right) {
        return compare_u128_f64(right, left.as_f64()?).map(Ordering::reverse);
    }

    left.as_f64()
        .zip(right.as_f64())
        .and_then(|(left, right)| left.partial_cmp(&right))
}

fn compare_i128_f64(integer: i128, float: f64) -> Option<Ordering> {
    if float.is_nan() {
        return None;
    }
    if float == 0.0 {
        return Some(integer.cmp(&0));
    }
    if integer >= 0 {
        if float.is_sign_negative() {
            Some(Ordering::Greater)
        } else {
            compare_u128_f64(integer as u128, float)
        }
    } else if !float.is_sign_negative() {
        Some(Ordering::Less)
    } else {
        compare_u128_f64(integer.unsigned_abs(), -float).map(Ordering::reverse)
    }
}

fn compare_u128_f64(integer: u128, float: f64) -> Option<Ordering> {
    if float.is_nan() {
        return None;
    }
    if float == 0.0 {
        return Some(integer.cmp(&0));
    }
    if float.is_sign_negative() {
        return Some(Ordering::Greater);
    }
    if float.is_infinite() {
        return Some(Ordering::Less);
    }

    let bits = float.to_bits();
    let exponent_bits = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1_u64 << 52) - 1);
    let (significand, exponent) = if exponent_bits == 0 {
        (fraction, -1074)
    } else {
        ((1_u64 << 52) | fraction, exponent_bits - 1023 - 52)
    };

    if exponent >= 0 {
        let shift = exponent as u32;
        if shift >= 128 || (significand as u128) > (u128::MAX >> shift) {
            return Some(Ordering::Less);
        }
        return Some(integer.cmp(&((significand as u128) << shift)));
    }

    let shift = (-exponent) as u32;
    let (integer_part, has_fraction) = if shift >= 64 {
        (0_u64, significand != 0)
    } else {
        let integer_part = significand >> shift;
        let remainder_mask = (1_u64 << shift) - 1;
        (integer_part, significand & remainder_mask != 0)
    };
    match integer.cmp(&(integer_part as u128)) {
        Ordering::Equal if has_fraction => Some(Ordering::Less),
        ordering => Some(ordering),
    }
}

pub(super) fn prefix_path(prefix: &str, path: &str) -> String {
    if prefix.is_empty() {
        path.to_string()
    } else if path.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}.{path}")
    }
}

#[cfg(test)]
mod numeric_comparison_tests {
    use super::compare_numbers;
    use proptest::prelude::*;
    use serde_json::json;
    use std::cmp::Ordering;

    #[derive(Clone, Debug)]
    enum Integer {
        Signed(i128),
        Unsigned(u128),
    }

    impl Integer {
        fn value(&self) -> serde_json::Value {
            match self {
                Self::Signed(value) => serde_json::to_value(value).unwrap(),
                Self::Unsigned(value) => serde_json::to_value(value).unwrap(),
            }
        }
    }

    fn integer() -> impl Strategy<Value = Integer> {
        prop_oneof![
            any::<i128>().prop_map(Integer::Signed),
            any::<u128>().prop_map(Integer::Unsigned),
        ]
    }

    #[test]
    fn mixed_integer_float_comparisons_remain_exact_above_f64_integer_precision() {
        let integer = serde_json::to_value(9_007_199_254_740_993_u128).unwrap();
        let float = json!(9_007_199_254_740_992.0_f64);
        assert_eq!(compare_numbers(&integer, &float), Some(Ordering::Greater));
        assert_eq!(compare_numbers(&float, &integer), Some(Ordering::Less));
    }

    #[test]
    fn u128_max_is_less_than_the_exact_f64_value_two_to_the_128() {
        let integer = serde_json::to_value(u128::MAX).unwrap();
        let float = serde_json::to_value(2_f64.powi(128)).unwrap();
        assert_eq!(compare_numbers(&integer, &float), Some(Ordering::Less));
        assert_eq!(compare_numbers(&float, &integer), Some(Ordering::Greater));
    }

    #[test]
    fn negative_mixed_numeric_comparisons_preserve_fractional_ordering() {
        let integer = serde_json::to_value(-9_007_199_254_740_993_i128).unwrap();
        let float = json!(-9_007_199_254_740_992.0_f64);
        assert_eq!(compare_numbers(&integer, &float), Some(Ordering::Less));
        assert_eq!(compare_numbers(&float, &integer), Some(Ordering::Greater));

        let minus_one = json!(-1_i128);
        let minus_half = json!(-0.5_f64);
        assert_eq!(
            compare_numbers(&minus_one, &minus_half),
            Some(Ordering::Less)
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 256,
            failure_persistence: None,
            ..ProptestConfig::default()
        })]

        #[test]
        fn integer_comparison_is_antisymmetric(left in integer(), right in integer()) {
            let left = left.value();
            let right = right.value();
            let forward = compare_numbers(&left, &right).unwrap();
            let reverse = compare_numbers(&right, &left).unwrap();

            prop_assert_eq!(forward, reverse.reverse());
        }

        #[test]
        fn integer_comparison_is_transitive(
            first in integer(),
            second in integer(),
            third in integer(),
        ) {
            let first = first.value();
            let second = second.value();
            let third = third.value();
            let first_second = compare_numbers(&first, &second).unwrap();
            let second_third = compare_numbers(&second, &third).unwrap();

            if first_second != Ordering::Greater && second_third != Ordering::Greater {
                prop_assert_ne!(compare_numbers(&first, &third), Some(Ordering::Greater));
            }
        }

        #[test]
        fn integer_float_comparison_matches_f64_inside_the_exact_integer_range(
            integer in -9_007_199_254_740_992_i64..=9_007_199_254_740_992_i64,
            float in -1.0e16_f64..1.0e16_f64,
        ) {
            let integer_value = serde_json::to_value(integer).unwrap();
            let float_value = serde_json::to_value(float).unwrap();
            let expected = (integer as f64).partial_cmp(&float);

            prop_assert_eq!(compare_numbers(&integer_value, &float_value), expected);
            prop_assert_eq!(
                compare_numbers(&float_value, &integer_value),
                expected.map(Ordering::reverse),
            );
        }
    }
}
