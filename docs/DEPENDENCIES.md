# Toolchain and dependency policy

## Rust

The package MSRV is Rust **1.97.0** (`rust-version = "1.97"`), edition 2024.
The repository development/verification toolchain is pinned separately to Rust
**1.97.1** in `rust-toolchain.toml`, with rustfmt and Clippy enabled.

This distinction is intentional: Cargo defines `rust-version` as the minimum
supported Rust version, whereas `rust-toolchain.toml` selects the toolchain used
to develop and verify this repository. Patch-level verification therefore does
not artificially raise the published MSRV.

## Direct runtime dependencies

Direct requirements declare the tested minimum compatible versions:

- serde `1.0.229`
- serde_json `1.0.151` with `arbitrary_precision`
- toml `1.1.4`
- serde_yaml_ng `0.10.0`
- proc-macro2 `1.0.107`
- proc-macro-crate `3.4.0`
- quote `1.0.47`
- syn `3.0.3`

Compatible updates are intentionally permitted for ordinary ecosystem
dependencies. `Cargo.lock` retains the exact reviewed repository resolution,
while downstream applications remain free to unify SemVer-compatible versions.
The runtime-to-`configlab-macros` dependency remains exact because generated
code and its runtime support must evolve together.

`toml` and `serde_yaml_ng` are optional format dependencies. The proc-macro
stack belongs only to the `/macros` companion and is activated for the main
package through the optional `derive` feature.

The first public alpha adds no dependency solely for diagnostics testing.
Compiler-failure coverage uses rustdoc `compile_fail` cases plus focused
proc-macro unit tests.

`proptest` is a development-only dependency used for generated resolver
permutation and numeric-comparison invariants. It is not part of ConfigLab's
runtime dependency graph or published API.

## Lockfile discipline

`./script/refresh-lock.sh` is the intentional dependency-update operation.
Normal verification uses `--locked` and must never mutate dependency resolution.
Review `Cargo.lock` after every refresh before committing or releasing it.

## Package dependency boundary

The main package records both the local `path` and exact package `version` for
`configlab-macros`. Local development uses the path; Cargo normalizes the
packaged manifest to the exact version dependency expected after publication.
`script/package-check.sh` patches the extracted unpublished companion locally so
that the packaged pair can be tested before either crate exists in a registry.

Both `configlab-macros` and `configlab` are publishable packages. The companion
crate must be published first because the main crate depends on its exact
version. Before publication, `script/package-check.sh` patches the extracted
companion locally so the packaged pair can be verified without requiring either
crate to exist in the registry. The manifests provide canonical docs.rs and
GitHub repository URLs.

The main source package intentionally includes its integration tests. Cargo
recognizes those files as package targets, so shipping them keeps the normalized
package manifest self-consistent and avoids warnings about omitted test targets.
Workspace examples, scripts, build output, and the companion crate source remain
outside the main `.crate`.

## Security review

Dependency refresh and advisory review are separate concerns. The deterministic
compiler gate does not auto-install or auto-update advisory tooling. For
promotion, review `Cargo.lock` and run the organization's managed Rust advisory
scanner separately.

The optional YAML stack includes the transitive `unsafe-libyaml` parser. Users
that do not need YAML can build without the `yaml` feature.
