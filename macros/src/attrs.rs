//! Parsing of `#[config(...)]` and Rust doc attributes.

use crate::model::{FieldAttrs, StructAttrs};
use syn::{Expr, LitStr};

pub(crate) fn parse_struct_attrs(attrs: &[syn::Attribute]) -> syn::Result<StructAttrs> {
    let mut output = StructAttrs::default();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("config")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("validate") {
                output.validate = Some(meta.value()?.parse::<LitStr>()?.parse()?);
            } else {
                return Err(meta.error("unsupported struct config attribute"));
            }
            Ok(())
        })?;
    }
    Ok(output)
}

pub(crate) fn parse_field_attrs(attrs: &[syn::Attribute]) -> syn::Result<FieldAttrs> {
    let mut output = FieldAttrs::default();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("config")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("default") {
                output.default = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("default_with") {
                output.default_with = Some(meta.value()?.parse::<LitStr>()?.parse()?);
            } else if meta.path.is_ident("env") {
                output.env = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("env_prefix") {
                output.env_prefix = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("sensitive") {
                output.sensitive = true;
            } else if meta.path.is_ident("required") {
                output.required = true;
            } else if meta.path.is_ident("required_when") {
                output.required_when = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("validate") {
                output.validate = Some(meta.value()?.parse::<LitStr>()?.parse()?);
            } else if meta.path.is_ident("merge") {
                output.merge = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("cli") {
                output.cli = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("rename") {
                output.rename = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("nested") {
                output.nested = true;
            } else if meta.path.is_ident("value") {
                output.value = true;
            } else if meta.path.is_ident("skip_env") {
                output.skip_env = true;
            } else if meta.path.is_ident("nullable") {
                output.nullable = true;
            } else if meta.path.is_ident("value_type") {
                output.value_type = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("removal") {
                output.removal = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("external_reference") {
                output.external_reference = true;
            } else if meta.path.is_ident("deprecated") {
                output.deprecated = Some(meta.value()?.parse::<LitStr>()?.value());
            } else {
                return Err(meta.error("unsupported field config attribute"));
            }
            Ok(())
        })?;
    }
    Ok(output)
}
pub(crate) fn docs(attrs: &[syn::Attribute]) -> Option<String> {
    let lines = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .filter_map(|attr| match &attr.meta {
            syn::Meta::NameValue(value) => match &value.value {
                Expr::Lit(value) => match &value.lit {
                    syn::Lit::Str(value) => Some(value.value().trim().to_string()),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    if lines.is_empty() {
        None
    } else {
        Some(lines.join(" "))
    }
}
