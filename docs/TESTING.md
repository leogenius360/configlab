# Testing and verification

`configlab` treats tests as executable semantic contracts rather than as
implementation-detail coverage.

## Test layers

- `tests/kernel/` validates definitions, components, diagnostics, requirements,
  security metadata, value types, removal permissions, and operational
  verification.
- `tests/resolution/` validates precedence, selectors, peer compatibility,
  merge/removal semantics, defaults/null behavior, accumulation, and
  provenance.
- `tests/facade/` validates derive-generated metadata, physical source
  normalization, discovery, formats, nesting, diagnostics, and secret-safe
  facade behavior.
- source-module unit tests exercise OS-string conversion directly, including
  platform-specific malformed Unicode cases that cannot be represented by the
  deterministic string-only descriptor APIs.
- `tests/source_api.rs` validates the descriptor grammar and source-payload
  redaction.
- `tests/runtime.rs` validates source reacquisition, transactional refresh,
  complete external reconciliation, optional-source removal, retained inputs,
  provenance-only changes, and redacted sensitive changes.
- the five packages under `/examples` are genuine public-API consumers and
  carry small use-case-focused tests.
- `macros/src/field.rs` contains focused unit tests for targeted derive
  diagnostic messages.
- `tests/performance_baseline.rs` contains ignored resolver and runtime
  release-mode measurement probes; they have no pass/fail timing thresholds.
- property tests generate bounded JSON object trees, permute logical inputs,
  and require complete resolution results (including provenance or structured
  errors) to remain identical for commutative merge policies and explicitly
  ordered accumulation. Numeric properties exercise antisymmetry, transitivity,
  and exact-range mixed integer/float comparison across generated values.

## Compiler-facing tests

The crate-level rustdoc includes `compile_fail` examples for derive misuse.
These intentionally test that unsupported attributes, contradictory field
roles, and invalid `required_when` placement are rejected at compile time.
They run as part of `cargo test -p configlab --doc`.

## Deterministic gate

`script/check.sh` is the authoritative repository verification command. It
requires the pinned Rust 1.97.1 development toolchain and a reviewed `Cargo.lock`,
then runs:

1. architecture checks;
2. formatting check;
3. all workspace tests and targets;
4. doctests, including compile-fail cases;
5. Clippy with warnings denied;
6. rustdoc with warnings denied;
7. the supported feature matrix;
8. release-mode compilation;
9. all canonical examples;
10. packaged-consumer verification;
11. the non-threshold release-mode performance baseline.

The runtime integration suite specifically exercises invalid file and external
updates, current-value preservation, repeated physical-source acquisition,
complete external replacement/removal, provenance-only changes, sensitive
redaction, and optional-file recovery/removal.

`script/refresh-lock.sh` is intentionally separate because dependency mutation
must not happen during verification.

GitHub Actions and GitLab CI run the complete gate on the MSRV toolchain, compiler-facing checks
on current stable Rust, complete gates on protected Windows and macOS shell
runners, and a pinned RustSec advisory scan. Release pipelines must not skip or
allow-fail the platform or advisory jobs.

## Packaging boundary

`script/package-check.sh` packages the proc-macro companion and main crate,
extracts the generated `.crate` archives, validates the package contents, and
builds/runs a fresh temporary consumer against those extracted packages. The
consumer declares the package MSRV (`1.97`), while repository verification stays
pinned to Rust 1.97.1. The main package intentionally ships its integration-test
sources but excludes workspace-only examples, scripts, build output, and the
proc-macro source tree. Package-content checks operate on archive **file**
entries rather than harmless directory entries and print any offending file
paths. This catches workspace-only path assumptions and missing packaged files
that ordinary workspace tests cannot.

## Performance measurements

`script/performance-baseline.sh` runs the ignored performance probes in release
mode and prints comparable wall-clock measurements for 100-setting kernel
resolution and live-configuration reconciliation. The numbers are observational
only; no correctness gate uses a timing threshold.
