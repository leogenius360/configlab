#![forbid(unsafe_code)]

//! ConfigLab is a derive-first configuration toolkit for Rust applications,
//! built around one deterministic resolution engine.
//!
//! The crate separates physical sources (files, environment, arguments)
//! from resolution semantics (context, selectors, explicit layers, merge
//! policies, defaults, removals, validation, provenance, and diagnostics).
//! Source type never determines precedence in the resolver. The common path is
//! [`Config::builder`] followed by source descriptors and either
//! [`ConfigBuilder::load`] or [`ConfigBuilder::resolve`].
//!
//! # Basic use
//!
//! ```no_run
//! use configlab::{args, env, Config, SecretRef};
//!
//! #[derive(Debug, Config)]
//! struct AppConfig {
//!     #[config(default = 8080, env = "APP_PORT")]
//!     port: u16,
//!     #[config(env = "APP_DATABASE_PASSWORD")]
//!     database_password: SecretRef,
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = AppConfig::builder()
//!     .file("config.toml")
//!     .environment(env())
//!     .arguments(args())
//!     .load()?;
//! println!("port = {}", config.port);
//! # Ok(())
//! # }
//! ```

extern crate self as configlab;

mod builder;
mod codec;
mod definition;
mod error;
mod limits;
mod loaded;
mod plan;
mod report;
mod resolution;
mod runtime;
mod secret;
mod source;
mod typed;

pub use builder::ConfigBuilder;
pub use definition::*;
pub use error::ConfigError;
pub use limits::ResolutionLimits;
pub use loaded::LoadedConfig;
pub use plan::ConfigPlan;
pub use report::{ComponentPresence, ConfigResolutionReport, ResolutionReport};
pub use resolution::*;
pub use resolution::{LogicalInput as Input, Operation as InputOperation};
pub use runtime::*;
pub use secret::{Secret, SecretRef};
pub use source::{
    ArgumentsSource, EnvironmentSource, FileSource, InlineSource, args, env, json, path, toml, yaml,
};
pub use typed::{CliBinding, Config, EnvBinding, EnvName, Severity, ValidationIssue};

#[cfg(feature = "derive")]
pub use configlab_macros::Config;

/// Creates an exact selector with concise key/value syntax.
#[macro_export]
macro_rules! selector {
    () => {
        $crate::Selector::any()
    };
    ($($key:ident = $value:expr),+ $(,)?) => {
        $crate::Selector::exact(vec![$((stringify!($key).to_string(), ($value).to_string())),+])
    };
}

/// Internals used by generated derive implementations.
///
/// This module is not a stable user-facing API. It is public only because a
/// procedural macro expands in the downstream crate and therefore must reach
/// these symbols through the public crate path.
#[doc(hidden)]
pub mod __private {
    pub use crate::codec::{decode, encode, join};
    pub use crate::definition::*;
    pub use crate::error::ConfigError;
    pub use crate::resolution::{EffectiveConfiguration, ResolutionInformation};
    pub use crate::typed::{CliBinding, Config, EnvBinding, EnvName, ValidationIssue};
    pub use serde_json;
}
