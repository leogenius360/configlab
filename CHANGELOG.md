# Changelog

All notable public changes to ConfigLab are documented here.

The project follows [Semantic Versioning](https://semver.org/) while recognizing
that alpha releases may contain breaking changes. Breaking alpha changes are
called out explicitly in this file and in release notes.

## [0.1.0-alpha.1] - Unreleased

First public alpha of ConfigLab.

### Added

- derive-first typed configuration through `#[derive(Config)]`;
- deterministic resolution with explicit layer ordering, canonical application
  of compatible equal-precedence peers, and permutation-invariant complete
  provenance;
- exact-match contextual selectors and same-layer specificity;
- file, environment, command-line, inline-document, and source-neutral logical
  input support;
- TOML, JSON, and YAML document support behind Cargo features;
- replace, deep, append, prepend, and combine-by-key merge policies;
- explicit unset, clear, map-entry removal, and list-item removal semantics;
- definition defaults and lookup-based derived defaults;
- declarative requirements, constraints, component composition, and validation;
- structured diagnostics, provenance, resolution information, and operational
  verification reports;
- `ConfigBuilder::resolve_report()` and `ConfigResolutionReport<T>` for aggregate
  kernel and typed-validation diagnostics;
- reusable `ConfigPlan<T>` values that retain one validated resolver and
  reacquire registered physical sources on every resolution;
- `LiveConfig<T>` with transactional source refresh, complete external-input
  reconciliation, immediate validated replacement, optional-source removal, and
  redaction-aware value/provenance change reports;
- configurable `ResolutionLimits` for physical-source and logical-resolution
  work;
- `Secret<T>` and `SecretRef` redaction helpers, with `Secret<T>` deliberately
  omitting `PartialEq` and `Eq` because ordinary generic equality is unsuitable
  for authenticating secret material;
- recursive derive sensitivity for syntactically visible nested secret
  containers and coarse-grained provenance for sensitive collections;
- log-safe `Debug` implementations for raw effective/report structures while
  preserving explicit raw Serde data contracts;
- one selector-specificity rule for every merge policy, including append and
  prepend accumulation;
- structured removal no-op information and entry-removal provenance;
- controlled non-Unicode process environment/argument handling without payload
  echoing;
- schema ancestry-overlap rejection and exact mixed integer/float relationship
  comparisons across the full `i128`/`u128` range;
- duplicate-case/duplicate-target rejection for derived-default customization;
- stable descriptive error/diagnostic codes;
- property-based resolver-permutation and exact numeric-comparison invariants,
  plus regression coverage proving unknown builder unset targets fail with
  `ResolveError::UnknownSetting` rather than silently no-op;
- package-boundary verification against an independent renamed consumer with
  archive checks based on actual package files, and a portable architecture
  gate that does not require a separately executable `wc` binary;
- five public-API example applications covering basic, CLI, contextual, runtime
  refresh, and source-neutral platform integration workflows;
- GitHub Actions verification across Linux, Windows, and macOS plus a pinned
  RustSec advisory audit;
- GitHub issue forms, pull-request guidance, and weekly Cargo/GitHub Actions
  dependency update automation.

### Changed

- deep and combine-by-key application and peer-conflict detection share one
  structured merge traversal so their semantics cannot drift;
- `ConfigError::Resolve` exposes its concrete `ResolveError` through
  `std::error::Error::source`;
- schema validation rechecks deserialized structural invariants and schema
  prefixing is fallible;
- `ValueShape::Any` is restricted to replace semantics and whole-setting unset;
- argument sources reject unknown and duplicate configuration arguments by
  default, with explicit `ignore_unknown()` opt-out;
- optional-section marker names are reserved by the derive facade and marker
  removal is limited to generated paths;
- dynamic provenance path segments escape dots and backslashes;
- ordinary ecosystem dependencies use compatible requirements while the
  main/proc-macro version pairing remains exact;
- the derive macro supports consumers that rename the `configlab` dependency;
- logical resource limits bound aggregate metadata, selector, path, key, and
  string bytes supplied by advanced integrations;
- the project is licensed solely under BSD-3-Clause;
- generated pitch-deck outputs are published as release assets rather than
  retained in source history.
