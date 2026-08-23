# Contributing to ConfigLab

Thanks for helping improve ConfigLab.

Unless explicitly stated otherwise, contributions intentionally submitted for
inclusion in ConfigLab are licensed under the BSD 3-Clause License without
additional terms or conditions.

## Ownership and governance

Dominic Maabobra Tuolong is the sole current owner and maintainer of ConfigLab.
Accepting a contribution does not transfer project ownership or governance.

## Before opening a change

For bug fixes, include a minimal reproduction when practical. For API or
semantic changes, describe the user problem first; avoid proposing new resolver
semantics without explaining why the existing model is insufficient.

## Development setup

The repository verification toolchain is pinned by `rust-toolchain.toml`.
Generate or refresh the lockfile intentionally:

```bash
./script/refresh-lock.sh
```

Then run the complete gate:

```bash
cargo fmt --all
./script/check.sh
```

Do not weaken tests, architecture checks, redaction rules, or lint settings to
make a change pass.

## Design principles

- source type never determines resolver precedence;
- layer precedence beats selector specificity across layers;
- equal-precedence disagreement fails rather than depending on input order;
- omission, null, unset, clear, and targeted removal remain distinct;
- derive code generates metadata/decoding but does not implement resolution;
- secret-safe output is preferred by default for library-owned diagnostics;
- modules/features are preferred over new crates unless a technical or
  publication boundary requires another package.

## Tests and documentation

Every bug fix should have a regression test. Public API changes should include
rustdoc and guide updates in the same change. Resolver semantic changes require
updates to `docs/RESOLUTION.md`, `docs/ACCEPTANCE.md`, and `CHANGELOG.md`.

## Commit scope

Keep changes focused. Formatting-only churn, unrelated refactors, and semantic
changes should not be mixed in one commit when they can be reviewed separately.
