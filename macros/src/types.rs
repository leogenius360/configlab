//! Rust type inspection and generated-name helpers.

use syn::{GenericArgument, PathArguments, ReturnType, Type, TypeParamBound};

pub(crate) fn generic_type_arg<'a>(ty: &'a Type, expected: &str, index: usize) -> Option<&'a Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    if segment.ident != expected {
        return None;
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    arguments
        .args
        .iter()
        .filter_map(|argument| match argument {
            GenericArgument::Type(ty) => Some(ty),
            _ => None,
        })
        .nth(index)
}

pub(crate) fn generic_type_arg_by_index(ty: &Type, index: usize) -> Option<&Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    arguments
        .args
        .iter()
        .filter_map(|argument| match argument {
            GenericArgument::Type(ty) => Some(ty),
            _ => None,
        })
        .nth(index)
}

pub(crate) fn type_name(ty: &Type) -> Option<String> {
    let Type::Path(path) = ty else {
        return None;
    };
    Some(path.path.segments.last()?.ident.to_string())
}

pub(crate) fn is_obvious_leaf(ty: &Type) -> bool {
    if generic_type_arg(ty, "Vec", 0).is_some() || generic_type_arg(ty, "Secret", 0).is_some() {
        return true;
    }
    matches!(
        type_name(ty).as_deref(),
        Some(
            "String"
                | "str"
                | "bool"
                | "i8"
                | "i16"
                | "i32"
                | "i64"
                | "i128"
                | "isize"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "u128"
                | "usize"
                | "f32"
                | "f64"
                | "PathBuf"
                | "SecretRef"
                | "Value"
                | "BTreeMap"
                | "HashMap"
        )
    )
}

pub(crate) fn contains_secret_type(ty: &Type) -> bool {
    if is_secret_reference_type(ty) || generic_type_arg(ty, "Secret", 0).is_some() {
        return true;
    }

    match ty {
        Type::Array(array) => contains_secret_type(&array.elem),
        Type::FnPtr(function) => {
            function
                .inputs
                .iter()
                .any(|input| contains_secret_type(&input.ty))
                || return_type_contains_secret(&function.output)
        }
        Type::Group(group) => contains_secret_type(&group.elem),
        Type::ImplTrait(object) => object.bounds.iter().any(bound_contains_secret),
        Type::Paren(paren) => contains_secret_type(&paren.elem),
        Type::Path(path) => path
            .path
            .segments
            .iter()
            .any(|segment| path_arguments_contain_secret(&segment.arguments)),
        Type::Ptr(pointer) => contains_secret_type(&pointer.elem),
        Type::Reference(reference) => contains_secret_type(&reference.elem),
        Type::Slice(slice) => contains_secret_type(&slice.elem),
        Type::TraitObject(object) => object.bounds.iter().any(bound_contains_secret),
        Type::Tuple(tuple) => tuple.elems.iter().any(contains_secret_type),
        _ => false,
    }
}

pub(crate) fn is_secret_reference_type(ty: &Type) -> bool {
    type_name(ty).as_deref() == Some("SecretRef")
}

pub(crate) fn semantic_value_type(mut ty: &Type) -> &Type {
    loop {
        if let Some(inner) = generic_type_arg(ty, "Secret", 0) {
            ty = inner;
            continue;
        }
        if let Some(inner) = generic_type_arg(ty, "Vec", 0) {
            ty = inner;
            continue;
        }
        if matches!(type_name(ty).as_deref(), Some("BTreeMap" | "HashMap"))
            && let Some(inner) = generic_type_arg_by_index(ty, 1)
        {
            ty = inner;
            continue;
        }
        break;
    }
    ty
}

fn path_arguments_contain_secret(arguments: &PathArguments) -> bool {
    match arguments {
        PathArguments::None => false,
        PathArguments::AngleBracketed(arguments) => {
            arguments.args.iter().any(|argument| match argument {
                GenericArgument::Type(ty) => contains_secret_type(ty),
                GenericArgument::AssocType(binding) => contains_secret_type(&binding.ty),
                GenericArgument::Constraint(constraint) => {
                    constraint.bounds.iter().any(bound_contains_secret)
                }
                _ => false,
            })
        }
        PathArguments::Parenthesized(arguments) => {
            arguments
                .inputs
                .iter()
                .any(|input| contains_secret_type(&input.ty))
                || return_type_contains_secret(&arguments.output)
        }
    }
}

fn bound_contains_secret(bound: &TypeParamBound) -> bool {
    let TypeParamBound::Trait(bound) = bound else {
        return false;
    };
    bound
        .path
        .segments
        .iter()
        .any(|segment| path_arguments_contain_secret(&segment.arguments))
}

fn return_type_contains_secret(output: &ReturnType) -> bool {
    match output {
        ReturnType::Default => false,
        ReturnType::Type(_, ty) => contains_secret_type(ty),
    }
}
pub(crate) fn screaming(value: &str) -> String {
    let mut output = String::new();
    let mut previous_lower = false;
    for character in value.chars() {
        if character.is_ascii_uppercase() && previous_lower {
            output.push('_');
        }
        if character == '-' {
            output.push('_');
            previous_lower = false;
        } else {
            output.push(character.to_ascii_uppercase());
            previous_lower = character.is_ascii_lowercase() || character.is_ascii_digit();
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::contains_secret_type;
    use syn::Type;

    fn parse_type(source: &str) -> Type {
        syn::parse_str(source).expect("test type should parse")
    }

    #[test]
    fn secret_detection_walks_syn3_function_pointer_arguments_and_outputs() {
        assert!(contains_secret_type(&parse_type(
            "fn(Secret<String>) -> bool"
        )));
        assert!(contains_secret_type(&parse_type(
            "fn(String) -> Secret<String>"
        )));
        assert!(!contains_secret_type(&parse_type("fn(String) -> bool")));
    }

    #[test]
    fn secret_detection_walks_syn3_parenthesized_trait_arguments() {
        assert!(contains_secret_type(&parse_type(
            "dyn Fn(Secret<String>) -> bool"
        )));
        assert!(contains_secret_type(&parse_type(
            "dyn Fn(String) -> Secret<String>"
        )));
        assert!(!contains_secret_type(&parse_type("dyn Fn(String) -> bool")));
    }
}
