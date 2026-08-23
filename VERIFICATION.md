# Verification status — ConfigLab 0.1.0-alpha.1

ConfigLab uses one repeatable quality gate for local development, GitHub
Actions, GitLab CI, and release preparation.

## Required toolchain

- package MSRV: Rust 1.97;
- repository verification toolchain: Rust 1.97.1;
- rustfmt and Clippy components installed through `rust-toolchain.toml`.

## Gate

Run:

```bash
./script/refresh-lock.sh
cargo fmt --all
./script/check.sh
```

`script/check.sh` verifies:

1. repository architecture rules;
2. formatting;
3. workspace tests with all features and targets;
4. normal and compile-fail doctests;
5. Clippy with warnings denied;
6. rustdoc with warnings denied;
7. supported feature-isolation combinations;
8. optimized release compilation;
9. all canonical examples;
10. both generated `.crate` archives and an independent packaged consumer;
11. the non-threshold release-mode performance baseline;
12. release metadata and public identity.

The separate CI advisory job runs `cargo audit --deny warnings` against the
reviewed lockfile.

## Acceptance inventory

The current alpha.1 development source carries:

- 142 library and integration tests, including generated properties;
- 11 executable example tests across five public-API applications;
- 7 proc-macro unit tests;
- 2 passing doctests and 4 passing compile-fail doctests;
- 2 ignored release-mode performance probes.

The category breakdown and security contracts are documented in
`docs/ACCEPTANCE.md`.

## Verified locally — 2026-08-16

Rust 1.97.1 completed the following checks on Windows:

- formatting;
- all 160 normally executable tests;
- all 6 doctests;
- Clippy for the complete workspace with `-D warnings`;
- rustdoc for the complete workspace with `-D warnings`;
- all 11 supported feature-isolation builds;
- an optimized all-feature library build;
- all five canonical applications;
- both release-mode performance probes;
- package creation for `configlab` and `configlab-macros`;
- compilation and execution of an independent consumer using the extracted
  packages and a renamed `configlab` dependency;
- architecture and release invariants, including one semantic resolver,
  synchronized versions, required public files, and absence of old project
  identities.

The main package contains 96 files and is 109.3 KiB compressed. The macro
package contains 15 files and is 11.5 KiB compressed. Each package contains the
single BSD 3-Clause license.

The canonical `script/check.sh` completed locally through Git Bash. The
architecture count checks use POSIX `awk` rather than depending on this host's
unusable bundled `wc.exe`. Publication still requires the hosted CI matrix and
the separate advisory-audit job to exit successfully on the pushed commit; no
local advisory scanner was installed for this verification.
