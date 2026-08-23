//! Normalized derive input model shared by generation stages.

use syn::{Expr, Type};

#[derive(Default)]
pub(crate) struct StructAttrs {
    pub(crate) validate: Option<syn::Path>,
}

#[derive(Default)]
pub(crate) struct FieldAttrs {
    pub(crate) default: Option<Expr>,
    pub(crate) default_with: Option<syn::Path>,
    pub(crate) env: Option<String>,
    pub(crate) env_prefix: Option<String>,
    pub(crate) sensitive: bool,
    pub(crate) required: bool,
    pub(crate) required_when: Option<String>,
    pub(crate) validate: Option<syn::Path>,
    pub(crate) merge: Option<String>,
    pub(crate) cli: Option<String>,
    pub(crate) rename: Option<String>,
    pub(crate) nested: bool,
    pub(crate) value: bool,
    pub(crate) skip_env: bool,
    pub(crate) nullable: bool,
    pub(crate) value_type: Option<String>,
    pub(crate) removal: Option<String>,
    pub(crate) external_reference: bool,
    pub(crate) deprecated: Option<String>,
}

pub(crate) struct Info {
    pub(crate) ident: syn::Ident,
    pub(crate) ty: Type,
    pub(crate) value_ty: Type,
    pub(crate) name: String,
    pub(crate) attrs: FieldAttrs,
    pub(crate) optional: bool,
    pub(crate) nested: bool,
    pub(crate) description: Option<String>,
}
