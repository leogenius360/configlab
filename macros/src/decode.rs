//! Generation of typed decoding code.

use crate::model::Info;
use quote::{format_ident, quote};

pub(crate) fn decode_tokens(field: &Info) -> proc_macro2::TokenStream {
    let ident = &field.ident;
    let name = &field.name;
    let path_ident = format_ident!("__{}_path", ident);
    let path = quote!(configlab::__private::join(prefix, #name));

    if field.nested && field.optional {
        let ty = &field.value_ty;
        quote! {
            #ident: {
                let #path_ident = #path;
                let marker = configlab::__private::join(&#path_ident, "__config_present");
                match effective
                    .get(&marker)
                    .and_then(configlab::__private::serde_json::Value::as_bool)
                {
                    Some(true) => Some(<#ty as configlab::__private::Config>::decode(
                        effective,
                        &#path_ident,
                    )?),
                    _ => None,
                }
            }
        }
    } else if field.nested {
        let ty = &field.value_ty;
        quote! {
            #ident: {
                let #path_ident = #path;
                <#ty as configlab::__private::Config>::decode(effective, &#path_ident)?
            }
        }
    } else if field.optional {
        let ty = &field.ty;
        quote! {
            #ident: {
                let #path_ident = #path;
                match effective.get(&#path_ident) {
                    Some(value) => configlab::__private::decode::<#ty>(value, &#path_ident)?,
                    None => None,
                }
            }
        }
    } else {
        let ty = &field.ty;
        quote! {
            #ident: {
                let #path_ident = #path;
                let value = effective.get(&#path_ident).ok_or_else(|| {
                    configlab::__private::ConfigError::Decode {
                        path: #path_ident.clone(),
                        message: "required value is missing".into(),
                    }
                })?;
                configlab::__private::decode::<#ty>(value, &#path_ident)?
            }
        }
    }
}
