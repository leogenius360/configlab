# configlab-macros

Procedural derive companion for [ConfigLab](https://docs.rs/configlab).

Application code should depend on `configlab`, which re-exports the `Config`
derive when the default `derive` feature is enabled. This crate exists as a
separate package because Rust procedural macros must be compiled as a
`proc-macro` crate.

It is not intended to be used as a standalone configuration library.
