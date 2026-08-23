//! Generation of kernel schema metadata from normalized fields.

use crate::model::Info;
use crate::types::{
    contains_secret_type, generic_type_arg, is_obvious_leaf, is_secret_reference_type,
    semantic_value_type, type_name,
};
use quote::quote;
use syn::{Field, spanned::Spanned};

pub(crate) fn schema_tokens(field: &Info) -> proc_macro2::TokenStream {
    let name = &field.name;
    let path = quote!(configlab::__private::join(prefix, #name));

    if field.nested {
        nested_schema_tokens(field, &path)
    } else {
        leaf_setting_tokens(field, &path)
    }
}

fn nested_schema_tokens(field: &Info, path: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let ty = &field.value_ty;
    let optional = field.optional;
    let sensitive = field.attrs.sensitive;
    let marker_requirement = requirement_tokens(field);

    quote! {{
        let child_prefix = #path;
        let marker = configlab::__private::join(&child_prefix, "__config_present");
        if #optional {
            let mut marker_setting = configlab::__private::SettingSpec::new(marker.clone());
            marker_setting.value_type = configlab::__private::ValueType::Boolean;
            marker_setting.requirement = #marker_requirement;
            schema.insert(marker_setting)?;
        }

        let child = <#ty as configlab::__private::Config>::schema(&child_prefix)?;
        for (_, setting) in child.iter() {
            let mut setting = setting.clone();
            if #optional {
                let gate = configlab::__private::Condition::boolean(marker.clone());
                setting.requirement = setting.requirement.clone().gated_by(gate);
                if let Some(configlab::__private::DefaultRule::Fixed(value)) = setting.default.take() {
                    let mut cases = ::std::collections::BTreeMap::new();
                    cases.insert("true".to_string(), value);
                    setting.default = Some(configlab::__private::DefaultRule::Lookup {
                        source: marker.clone(),
                        cases,
                    });
                }
            }
            if #sensitive {
                setting.disclosure = configlab::__private::Disclosure::Sensitive;
            }
            schema.insert(setting)?;
        }

        for rule in child.rules() {
            let mut rule = rule.clone();
            if #optional {
                rule = rule.gated_by(configlab::__private::Condition::boolean(
                    marker.clone(),
                ));
            }
            schema.add_rule(rule);
        }
    }}
}

fn leaf_setting_tokens(field: &Info, path: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let nullable = field.attrs.nullable || field.optional;
    let shape = shape_tokens(field);
    let value_type = value_type_tokens(field);
    let merge = merge_tokens(field);
    let requirement = requirement_tokens(field);
    let disclosure = disclosure_tokens(field);
    let representation = representation_tokens(field);
    let removal = removal_tokens(field);
    let description = description_tokens(field);
    let deprecation = deprecation_tokens(field);
    let default = default_tokens(field);

    quote! {{
        let path = #path;
        let mut setting = configlab::__private::SettingSpec::new(path.clone());
        setting.shape = #shape;
        setting.value_type = #value_type;
        setting.merge = #merge;
        setting.requirement = #requirement;
        setting.nullable = #nullable;
        setting.representation = #representation;
        setting.disclosure = #disclosure;
        setting.removal = #removal;
        #description
        #deprecation
        #default
        schema.insert(setting)?;
    }}
}

fn disclosure_tokens(field: &Info) -> proc_macro2::TokenStream {
    if field.attrs.sensitive || contains_secret_type(&field.value_ty) {
        quote!(configlab::__private::Disclosure::Sensitive)
    } else {
        quote!(configlab::__private::Disclosure::Public)
    }
}

fn representation_tokens(field: &Info) -> proc_macro2::TokenStream {
    if is_secret_reference_type(&field.value_ty) {
        quote!(configlab::__private::Representation::SecretReference)
    } else if field.attrs.external_reference {
        quote!(configlab::__private::Representation::ExternalReference)
    } else {
        quote!(configlab::__private::Representation::Ordinary)
    }
}

fn description_tokens(field: &Info) -> Option<proc_macro2::TokenStream> {
    field.description.as_ref().map(|description| {
        quote! { setting.description = Some(#description.to_string()); }
    })
}

fn deprecation_tokens(field: &Info) -> Option<proc_macro2::TokenStream> {
    field.attrs.deprecated.as_ref().map(|message| {
        quote! {
            setting.deprecation = Some(configlab::__private::Deprecation {
                message: #message.to_string(),
                replacement: None,
            });
        }
    })
}

fn default_tokens(field: &Info) -> Option<proc_macro2::TokenStream> {
    if let Some(expr) = field.attrs.default.as_ref() {
        Some(quote! {
            setting.default = Some(configlab::__private::DefaultRule::Fixed(
                configlab::__private::encode(&(#expr), &path)?,
            ));
        })
    } else {
        field.attrs.default_with.as_ref().map(|function| {
            quote! {
                setting.default = Some(configlab::__private::DefaultRule::Fixed(
                    configlab::__private::encode(&(#function()), &path)?,
                ));
            }
        })
    }
}

fn requirement_tokens(field: &Info) -> proc_macro2::TokenStream {
    if field.attrs.required || !field.optional {
        quote!(configlab::__private::Requirement::Required)
    } else if let Some(relative) = &field.attrs.required_when {
        quote!(configlab::__private::Requirement::RequiredWhen(
            configlab::__private::Condition::boolean(
                configlab::__private::join(prefix, #relative),
            )
        ))
    } else {
        quote!(configlab::__private::Requirement::Optional)
    }
}

fn shape_tokens(field: &Info) -> proc_macro2::TokenStream {
    let ty = generic_type_arg(&field.value_ty, "Secret", 0).unwrap_or(&field.value_ty);
    if generic_type_arg(ty, "Vec", 0).is_some() {
        quote!(configlab::__private::ValueShape::OrderedList)
    } else if matches!(type_name(ty).as_deref(), Some("BTreeMap" | "HashMap")) {
        quote!(configlab::__private::ValueShape::NamedMap)
    } else if type_name(ty).as_deref() == Some("Value")
        || (!is_obvious_leaf(ty) && field.attrs.value)
    {
        quote!(configlab::__private::ValueShape::Any)
    } else {
        quote!(configlab::__private::ValueShape::Scalar)
    }
}

fn value_type_tokens(field: &Info) -> proc_macro2::TokenStream {
    if let Some(value_type) = field.attrs.value_type.as_deref() {
        return explicit_value_type_tokens(value_type);
    }

    let ty = semantic_value_type(&field.value_ty);

    match type_name(ty).as_deref() {
        Some("bool") => quote!(configlab::__private::ValueType::Boolean),
        Some("i8" | "i16" | "i32" | "i64" | "i128" | "isize") => {
            quote!(configlab::__private::ValueType::Integer)
        }
        Some("u8" | "u16" | "u32" | "u64" | "u128" | "usize") => {
            quote!(configlab::__private::ValueType::UnsignedInteger)
        }
        Some("f32" | "f64") => quote!(configlab::__private::ValueType::Float),
        Some("String" | "str" | "SecretRef") => {
            quote!(configlab::__private::ValueType::Text)
        }
        Some("PathBuf") => quote!(configlab::__private::ValueType::Path),
        _ => quote!(configlab::__private::ValueType::Any),
    }
}

fn explicit_value_type_tokens(value_type: &str) -> proc_macro2::TokenStream {
    match value_type {
        "any" => quote!(configlab::__private::ValueType::Any),
        "bool" | "boolean" => quote!(configlab::__private::ValueType::Boolean),
        "integer" => quote!(configlab::__private::ValueType::Integer),
        "unsigned" => quote!(configlab::__private::ValueType::UnsignedInteger),
        "float" => quote!(configlab::__private::ValueType::Float),
        "text" => quote!(configlab::__private::ValueType::Text),
        "duration" => quote!(configlab::__private::ValueType::Duration),
        "byte_size" => quote!(configlab::__private::ValueType::ByteSize),
        "path" => quote!(configlab::__private::ValueType::Path),
        "address" => quote!(configlab::__private::ValueType::Address),
        _ => unreachable!("value_type is validated before token generation"),
    }
}

fn merge_tokens(field: &Info) -> proc_macro2::TokenStream {
    match field.attrs.merge.as_deref() {
        Some("replace") | None => quote!(configlab::__private::MergePolicy::Replace),
        Some("deep") => quote!(configlab::__private::MergePolicy::Deep),
        Some("append") => quote!(configlab::__private::MergePolicy::Append),
        Some("prepend") => quote!(configlab::__private::MergePolicy::Prepend),
        Some("combine_by_key") => {
            quote!(configlab::__private::MergePolicy::CombineByKey)
        }
        Some(_) => unreachable!("merge policy is validated before token generation"),
    }
}

fn removal_tokens(field: &Info) -> proc_macro2::TokenStream {
    let Some(removal) = field.attrs.removal.as_deref() else {
        return quote!(configlab::__private::RemovalPolicy::default());
    };
    let (unset, clear, entries, items) = parse_removal(removal);
    quote!(configlab::__private::RemovalPolicy {
        unset: #unset,
        clear: #clear,
        remove_map_entry: #entries,
        remove_list_item: #items,
    })
}

pub(crate) fn validate_removal_attribute(field: &Field, removal: &str) -> syn::Result<()> {
    let tokens = removal
        .split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    if tokens.is_empty() {
        return Err(syn::Error::new(
            field.span(),
            "removal must name at least one operation",
        ));
    }
    if tokens
        .iter()
        .any(|token| !matches!(*token, "none" | "unset" | "clear" | "entries" | "items"))
    {
        return Err(syn::Error::new(
            field.span(),
            "removal accepts only none, unset, clear, entries, or items",
        ));
    }
    if tokens.contains(&"none") && tokens.len() != 1 {
        return Err(syn::Error::new(
            field.span(),
            "removal=\"none\" cannot be combined with other removal operations",
        ));
    }
    Ok(())
}

fn parse_removal(removal: &str) -> (bool, bool, bool, bool) {
    let tokens = removal.split(',').map(str::trim).collect::<Vec<_>>();
    if tokens.len() == 1 && tokens[0] == "none" {
        return (false, false, false, false);
    }
    (
        tokens.contains(&"unset"),
        tokens.contains(&"clear"),
        tokens.contains(&"entries"),
        tokens.contains(&"items"),
    )
}
