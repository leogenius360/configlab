#![allow(dead_code)]

#[path = "facade/support.rs"]
mod support;

#[path = "facade/contextual_service.rs"]
mod contextual_service;

#[path = "facade/nesting.rs"]
mod nesting;

#[path = "facade/layers.rs"]
mod layers;

#[path = "facade/discovery.rs"]
mod discovery;

#[path = "facade/formats.rs"]
mod formats;

#[path = "facade/environment_arguments.rs"]
mod environment_arguments;

#[path = "facade/secrets.rs"]
mod secrets;

#[path = "facade/derive_metadata.rs"]
mod derive_metadata;

#[path = "facade/diagnostics.rs"]
mod diagnostics;

#[path = "facade/file_sources.rs"]
mod file_sources;
