//! Generation of typed post-decode validation hooks.

use crate::model::Info;
use quote::quote;

pub(crate) fn validate_tokens(field: &Info) -> proc_macro2::TokenStream {
    let ident = &field.ident;
    let name = &field.name;
    let nested = if field.nested && field.optional {
        quote! {
            if let Some(value) = &self.#ident {
                for issue in configlab::__private::Config::validate(value) {
                    issues.push(issue.prefixed(#name));
                }
            }
        }
    } else if field.nested {
        quote! {
            for issue in configlab::__private::Config::validate(&self.#ident) {
                issues.push(issue.prefixed(#name));
            }
        }
    } else {
        quote! {}
    };
    let own = field.attrs.validate.as_ref().map(|validator| {
        quote! {
            if let Err(error) = #validator(&self.#ident) {
                issues.push(configlab::__private::ValidationIssue::at(
                    #name,
                    error.to_string(),
                ));
            }
        }
    });
    quote! {
        #nested
        #own
    }
}
