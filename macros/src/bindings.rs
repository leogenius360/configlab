//! Generation of environment, CLI, and optional-group binding metadata.

use crate::model::Info;
use crate::types::{screaming, type_name};
use quote::quote;

pub(crate) fn env_tokens(field: &Info) -> proc_macro2::TokenStream {
    let name = &field.name;
    let path = quote!(configlab::__private::join(prefix, #name));

    if field.nested {
        let ty = &field.value_ty;
        if field.attrs.skip_env {
            return quote! {};
        }
        let env_prefix = field
            .attrs
            .env_prefix
            .clone()
            .unwrap_or_else(|| screaming(name));
        quote! {{
            let child_prefix = #path;
            for mut binding in <#ty as configlab::__private::Config>::env_bindings(&child_prefix) {
                if let configlab::__private::EnvName::Auto(parts) = &mut binding.name {
                    parts.insert(0, #env_prefix.to_string());
                }
                bindings.push(binding);
            }
        }}
    } else if field.attrs.skip_env {
        quote! {}
    } else {
        let name_tokens = if let Some(exact) = &field.attrs.env {
            quote!(configlab::__private::EnvName::Exact(#exact.to_string()))
        } else {
            let auto = screaming(name);
            quote!(configlab::__private::EnvName::Auto(vec![#auto.to_string()]))
        };
        quote! {
            bindings.push(configlab::__private::EnvBinding {
                path: #path,
                name: #name_tokens,
            });
        }
    }
}

pub(crate) fn cli_tokens(field: &Info) -> proc_macro2::TokenStream {
    let name = &field.name;
    let path = quote!(configlab::__private::join(prefix, #name));

    if field.nested {
        let ty = &field.value_ty;
        quote! {{
            let child_prefix = #path;
            bindings.extend(<#ty as configlab::__private::Config>::cli_bindings(&child_prefix));
        }}
    } else if let Some(flag) = &field.attrs.cli {
        let boolean = type_name(&field.value_ty).as_deref() == Some("bool");
        quote! {
            bindings.push(configlab::__private::CliBinding {
                path: #path,
                flag: #flag.to_string(),
                boolean: #boolean,
            });
        }
    } else {
        quote! {}
    }
}

pub(crate) fn group_tokens(field: &Info) -> proc_macro2::TokenStream {
    let name = &field.name;
    if field.nested {
        let ty = &field.value_ty;
        let optional = field.optional;
        quote! {{
            let child_prefix = configlab::__private::join(prefix, #name);
            if #optional {
                groups.push(child_prefix.clone());
            }
            groups.extend(<#ty as configlab::__private::Config>::optional_groups(&child_prefix));
        }}
    } else {
        quote! {}
    }
}
