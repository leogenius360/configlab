//! Top-level orchestration of `Config` derive expansion.

use crate::attrs::parse_struct_attrs;
use crate::bindings::{cli_tokens, env_tokens, group_tokens};
use crate::decode::decode_tokens;
use crate::field::field_info;
use crate::schema::schema_tokens;
use crate::validation::validate_tokens;
use proc_macro_crate::{FoundCrate, crate_name};
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, spanned::Spanned};

pub(crate) fn expand(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new(
            input.generics.span(),
            "Config derive currently supports non-generic structs",
        ));
    }

    let ident = input.ident;
    let runtime = match crate_name("configlab").map_err(|error| {
        syn::Error::new(
            ident.span(),
            format!("could not locate configlab dependency: {error}"),
        )
    })? {
        FoundCrate::Itself => quote!(::configlab),
        FoundCrate::Name(name) => {
            let name = format_ident!("{}", name.replace('-', "_"));
            quote!(::#name)
        }
    };
    let struct_attrs = parse_struct_attrs(&input.attrs)?;
    let fields = match input.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => fields.named,
            _ => {
                return Err(syn::Error::new(
                    ident.span(),
                    "Config requires named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new(
                ident.span(),
                "Config can only be derived for structs",
            ));
        }
    };

    let infos = fields
        .iter()
        .map(field_info)
        .collect::<syn::Result<Vec<_>>>()?;
    let schema = infos.iter().map(schema_tokens);
    let decode = infos.iter().map(decode_tokens);
    let env = infos.iter().map(env_tokens);
    let cli = infos.iter().map(cli_tokens);
    let validate = infos.iter().map(validate_tokens);
    let groups = infos.iter().map(group_tokens);
    let root_validate = struct_attrs.validate.map(|path| {
        quote! {
            if let Err(error) = #path(self) {
                issues.push(configlab::__private::ValidationIssue::new(error.to_string()));
            }
        }
    });

    Ok(quote! {
        const _: () = {
        use #runtime as configlab;

        impl configlab::__private::Config for #ident {
            fn schema(
                prefix: &str,
            ) -> Result<
                configlab::__private::Schema,
                configlab::__private::ConfigError,
            > {
                let mut schema = configlab::__private::Schema::new();
                #(#schema)*
                Ok(schema)
            }

            fn decode(
                effective: &configlab::__private::EffectiveConfiguration,
                prefix: &str,
            ) -> Result<Self, configlab::__private::ConfigError> {
                Ok(Self { #(#decode),* })
            }

            fn env_bindings(prefix: &str) -> Vec<configlab::__private::EnvBinding> {
                let mut bindings = Vec::new();
                #(#env)*
                bindings
            }

            fn cli_bindings(prefix: &str) -> Vec<configlab::__private::CliBinding> {
                let mut bindings = Vec::new();
                #(#cli)*
                bindings
            }

            fn optional_groups(prefix: &str) -> Vec<String> {
                let mut groups = Vec::new();
                #(#groups)*
                groups
            }

            fn validate(&self) -> Vec<configlab::__private::ValidationIssue> {
                let mut issues = Vec::new();
                #(#validate)*
                #root_validate
                issues
            }
        }
        };
    })
}
