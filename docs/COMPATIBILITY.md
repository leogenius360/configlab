# Compatibility policy

ConfigLab is currently an alpha project. This document defines what users can
expect from `0.1.0-alpha.*` releases and what the project intends to stabilize.

## Alpha releases

Breaking changes may occur between alpha releases when they materially improve
correctness, safety, naming, or long-term API coherence. Every intentional
breaking change must be documented in `CHANGELOG.md` and release notes.

Within one published alpha version, the package contents are immutable. Fixes
are released as a new prerelease version rather than replacing an existing
crate.

## Semantic contracts

The resolver semantics documented in `RESOLUTION.md` are treated more
conservatively than convenience APIs. Changes to precedence, selectors, peer
ambiguity, merge/removal behavior, defaults, provenance, or redaction require:

1. an explicit design rationale;
2. executable regression tests;
3. documentation updates; and
4. a changelog entry.

Source type must not silently become a precedence mechanism.

## Public API

Public Rust items may still change during alpha. The project prefers removing
or renaming questionable APIs before `0.1.0` rather than accumulating permanent
compatibility aliases.

Generated proc-macro internals under `configlab::__private` are not public API
even though they must be `pub` for downstream macro expansion.

## Crate naming and derive expansion

The `Config` derive resolves the runtime crate through Cargo package metadata,
so consumers may rename the `configlab` dependency. The packaged-consumer gate
uses a renamed dependency to keep this contract executable.

## Error text and codes

Human-readable `Display` text may improve between releases. Integrations should
use `ConfigError::code()`, `ResolveError::code()`, and `DiagnosticCode::as_str()`
when they need stable categories.

Descriptive codes are intended to be substantially more stable than wording,
but may still evolve during alpha when a category is proven misleading.

## MSRV

The package currently declares Rust `1.97` as its minimum supported Rust
version. Repository verification is pinned to Rust `1.97.1`.

During the alpha series this is an intentional current-stable support policy,
not a claim that every implementation detail requires Rust 1.97. Supporting an
older compiler means selecting an explicit candidate and running the complete
feature, example, and packaged-consumer gate on it before lowering
`rust-version`; successful ad hoc compilation with `--ignore-rust-version` is
not sufficient evidence.

An MSRV increase must be intentional, documented in `CHANGELOG.md`, and verified
by the package-consumer gate. During alpha it may occur in a prerelease; after
`0.1.0`, the project will define a stricter MSRV-change policy before making such
a change.

## Cargo features

Supported feature combinations are exercised by `script/check.sh`. Removing a
feature, changing its meaning, or making a previously optional runtime
dependency unconditional is considered a compatibility change and must be
documented.

## Runtime refresh

The `LiveConfig` refresh/reconciliation API is experimental during the alpha
series. The invariants documented in `RUNTIME_LIFECYCLE.md`—explicit precedence,
physical-source reacquisition, invalid updates preserving current state,
complete validated replacement, retained external inputs, and redacted
sensitive changes—are treated as semantic contracts. Convenience names and
external adapter shapes may still change before `0.1.0` based on real-service
use.
