//! Validation and normalization of one Rust struct field.

use crate::attrs::{docs, parse_field_attrs};
use crate::model::{FieldAttrs, Info};
use crate::schema::validate_removal_attribute;
use crate::types::{generic_type_arg, is_obvious_leaf, is_secret_reference_type};
use syn::{Field, Ident, Type, spanned::Spanned};

struct FieldRole {
    value_ty: Type,
    name: String,
    optional: bool,
    nested: bool,
}

pub(crate) fn field_info(field: &Field) -> syn::Result<Info> {
    let (ident, attrs) = parse_field(field)?;
    validate_attribute_combinations(field, &attrs)?;

    let role = infer_field_role(&ident, &field.ty, &attrs);
    validate_field_role(field, &attrs, &role)?;

    Ok(Info {
        ident,
        ty: field.ty.clone(),
        value_ty: role.value_ty,
        name: role.name,
        attrs,
        optional: role.optional,
        nested: role.nested,
        description: docs(&field.attrs),
    })
}

fn parse_field(field: &Field) -> syn::Result<(Ident, FieldAttrs)> {
    let ident = field
        .ident
        .clone()
        .ok_or_else(|| syn::Error::new(field.span(), "expected named field"))?;
    let attrs = parse_field_attrs(&field.attrs)?;
    Ok((ident, attrs))
}

fn validate_attribute_combinations(field: &Field, attrs: &FieldAttrs) -> syn::Result<()> {
    if attrs.nested && attrs.value {
        return Err(syn::Error::new(
            field.span(),
            "field cannot be both nested and value",
        ));
    }
    if attrs.default.is_some() && attrs.default_with.is_some() {
        return Err(syn::Error::new(
            field.span(),
            "use either default or default_with, not both",
        ));
    }
    if attrs.required && attrs.required_when.is_some() {
        return Err(syn::Error::new(
            field.span(),
            "use either required or required_when, not both",
        ));
    }
    Ok(())
}

fn infer_field_role(ident: &Ident, ty: &Type, attrs: &FieldAttrs) -> FieldRole {
    let optional_inner = generic_type_arg(ty, "Option", 0);
    let optional = optional_inner.is_some();
    let value_ty = optional_inner.cloned().unwrap_or_else(|| ty.clone());
    let nested = attrs.nested
        || attrs.env_prefix.is_some()
        || (!has_leaf_hint(attrs) && !is_obvious_leaf(&value_ty));
    let name = attrs.rename.clone().unwrap_or_else(|| ident.to_string());

    FieldRole {
        value_ty,
        name,
        optional,
        nested,
    }
}

fn has_leaf_hint(attrs: &FieldAttrs) -> bool {
    attrs.default.is_some()
        || attrs.default_with.is_some()
        || attrs.env.is_some()
        || attrs.merge.is_some()
        || attrs.cli.is_some()
        || attrs.value
        || attrs.value_type.is_some()
        || attrs.removal.is_some()
        || attrs.external_reference
}

fn validate_field_role(field: &Field, attrs: &FieldAttrs, role: &FieldRole) -> syn::Result<()> {
    validate_optionality(field, attrs, role)?;
    validate_names(field, attrs, role)?;
    validate_policies(field, attrs)?;
    validate_representation(field, attrs, role)?;
    Ok(())
}

fn validate_optionality(field: &Field, attrs: &FieldAttrs, role: &FieldRole) -> syn::Result<()> {
    if attrs.required_when.is_some() && !role.optional {
        return Err(syn::Error::new(
            field.span(),
            "required_when is only valid on Option<T> or optional nested configuration",
        ));
    }
    Ok(())
}

fn validate_names(field: &Field, attrs: &FieldAttrs, role: &FieldRole) -> syn::Result<()> {
    if role.name.is_empty() || role.name.contains('.') || role.name == "__config_present" {
        return Err(syn::Error::new(
            field.span(),
            "config field name must be one non-empty, non-reserved path segment",
        ));
    }
    if attrs.env.as_deref() == Some("") {
        return Err(syn::Error::new(field.span(), "env name cannot be empty"));
    }
    if attrs.env_prefix.as_deref() == Some("") {
        return Err(syn::Error::new(field.span(), "env_prefix cannot be empty"));
    }
    if attrs
        .required_when
        .as_deref()
        .is_some_and(|path| path.is_empty())
    {
        return Err(syn::Error::new(
            field.span(),
            "required_when path cannot be empty",
        ));
    }
    if let Some(flag) = attrs.cli.as_deref()
        && !valid_cli_flag(flag)
    {
        return Err(syn::Error::new(
            field.span(),
            "cli name must start with an ASCII letter or digit and contain only ASCII letters, digits, `-`, or `_`",
        ));
    }
    Ok(())
}

fn validate_policies(field: &Field, attrs: &FieldAttrs) -> syn::Result<()> {
    if let Some(merge) = attrs.merge.as_deref()
        && !matches!(
            merge,
            "replace" | "deep" | "append" | "prepend" | "combine_by_key"
        )
    {
        return Err(syn::Error::new(
            field.span(),
            format!("unsupported merge policy `{merge}`"),
        ));
    }
    if let Some(value_type) = attrs.value_type.as_deref()
        && !matches!(
            value_type,
            "any"
                | "bool"
                | "boolean"
                | "integer"
                | "unsigned"
                | "float"
                | "text"
                | "duration"
                | "byte_size"
                | "path"
                | "address"
        )
    {
        return Err(syn::Error::new(
            field.span(),
            format!("unsupported value_type `{value_type}`"),
        ));
    }
    if let Some(removal) = attrs.removal.as_deref() {
        validate_removal_attribute(field, removal)?;
    }
    Ok(())
}

fn validate_representation(field: &Field, attrs: &FieldAttrs, role: &FieldRole) -> syn::Result<()> {
    if attrs.external_reference && is_secret_reference_type(&role.value_ty) {
        return Err(syn::Error::new(
            field.span(),
            "SecretRef already has secret-reference representation and cannot be external_reference",
        ));
    }

    if role.nested
        && (attrs.default.is_some()
            || attrs.default_with.is_some()
            || attrs.env.is_some()
            || attrs.merge.is_some()
            || attrs.cli.is_some()
            || attrs.value
            || attrs.nullable
            || attrs.value_type.is_some()
            || attrs.removal.is_some()
            || attrs.external_reference
            || attrs.deprecated.is_some())
    {
        return Err(syn::Error::new(
            field.span(),
            "nested fields cannot use scalar defaults, exact env, merge, cli, value, nullable, value_type, removal, external_reference, or deprecated attributes",
        ));
    }
    Ok(())
}

fn valid_cli_flag(flag: &str) -> bool {
    let mut characters = flag.chars();
    characters
        .next()
        .is_some_and(|character| character.is_ascii_alphanumeric())
        && characters
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

#[cfg(test)]
mod tests {
    use super::field_info;
    use syn::{Field, ItemStruct, parse_quote};

    fn field_error(field: &Field) -> String {
        match field_info(field) {
            Ok(_) => panic!("field unexpectedly passed validation"),
            Err(error) => error.to_string(),
        }
    }

    #[test]
    fn contradictory_nested_and_value_roles_have_a_targeted_error() {
        let item: ItemStruct = parse_quote! {
            struct Example {
                #[config(nested, value)]
                child: String,
            }
        };
        let field = item.fields.iter().next().unwrap();
        assert_eq!(field_error(field), "field cannot be both nested and value");
    }

    #[test]
    fn required_when_on_non_optional_field_has_a_targeted_error() {
        let item: ItemStruct = parse_quote! {
            struct Example {
                #[config(required_when = "enabled")]
                endpoint: String,
            }
        };
        let field = item.fields.iter().next().unwrap();
        assert_eq!(
            field_error(field),
            "required_when is only valid on Option<T> or optional nested configuration"
        );
    }

    #[test]
    fn unsupported_merge_policy_names_the_invalid_value() {
        let item: ItemStruct = parse_quote! {
            struct Example {
                #[config(merge = "mystery")]
                values: Vec<String>,
            }
        };
        let field = item.fields.iter().next().unwrap();
        assert_eq!(field_error(field), "unsupported merge policy `mystery`");
    }

    #[test]
    fn internal_presence_marker_name_is_reserved() {
        let item: ItemStruct = parse_quote! {
            struct Example {
                #[config(rename = "__config_present")]
                value: bool,
            }
        };
        let field = item.fields.iter().next().unwrap();
        assert!(field_error(field).contains("non-reserved"));
    }

    #[test]
    fn cli_names_reject_unaddressable_characters() {
        let item: ItemStruct = parse_quote! {
            struct Example {
                #[config(cli = "port=value")]
                value: bool,
            }
        };
        let field = item.fields.iter().next().unwrap();
        assert!(field_error(field).contains("ASCII"));
    }
}
