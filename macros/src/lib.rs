#![forbid(unsafe_code)]

//! Procedural macro companion for ConfigLab (`configlab`).
//!
//! This crate exists only because Rust procedural macros must be compiled as a
//! dedicated `proc-macro` crate. Users normally depend on `configlab`; the main
//! package re-exports the `Config` derive behind its `derive` feature.

mod attrs;
mod bindings;
mod decode;
mod expand;
mod field;
mod model;
mod schema;
mod types;
mod validation;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

/// Derives the typed `configlab` contract for a named Rust struct.
#[proc_macro_derive(Config, attributes(config))]
pub fn derive_config(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand::expand(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
