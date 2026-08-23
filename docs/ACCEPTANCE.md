# Acceptance contract — ConfigLab 0.1.0-alpha.1

The first public alpha is accepted only when the deterministic resolver
semantics, derive facade, source descriptor API, security boundaries, package
boundary, and canonical examples all pass the repository quality gate.

## Behavioral inventory

The current alpha.1 development source contains 153 runtime/example behavioral
tests on a single host platform:

- 30 kernel/definition integration tests;
- 39 resolver integration tests, including generated permutation properties;
- 48 typed-facade integration tests;
- 7 source-descriptor API tests;
- 10 live-refresh and external-reconciliation tests;
- 8 root-library unit tests (6 exact/generated numeric-comparison tests plus one
  platform-selected non-Unicode environment test and one platform-selected
  non-Unicode argument test);
- 11 canonical example tests.

Additionally:

- 7 proc-macro unit tests (5 diagnostic tests plus 2 Syn 3 type-walker
  compatibility tests);
- 2 compiling rustdoc examples;
- 4 compile-fail rustdoc contracts (3 derive contracts plus secret-wrapper
  equality misuse);
- 2 ignored-by-default resolver/runtime performance probes run explicitly in
  release mode.

## Resolver invariants

The acceptance suite protects at least these contracts:

- source type never determines precedence;
- layer precedence outranks selector specificity across layers;
- selector specificity only compares matching peers inside one layer;
- append/prepend apply selector specificity before peer ordering, so
  less-specific accumulating inputs are shadowed rather than accumulated;
- equal-precedence disagreement fails loudly rather than depending on input
  order;
- complete results, including provenance, remain invariant when compatible
  logical inputs are permuted;
- `Deep` recursively merges object fields;
- `CombineByKey` combines named-map entries by direct key and replaces a
  colliding entry as one unit;
- identical equal-precedence `Unset`/`Clear` operations are idempotent;
- map-entry removals commute; distinct list-item removals commute; duplicate
  same-value list-item removals are ambiguous because multiplicity matters;
- omission, null, unset, clear, and targeted removals remain distinct;
- definition defaults and lookup-based derived defaults preserve provenance;
- duplicate derived-default cases/target customizations fail rather than
  silently overwriting earlier declarations;
- schema setting paths cannot overlap by ancestry;
- mixed integer/float relationship rules preserve exact full-width integer
  ordering rather than rounding integers through `f64`;
- representation and disclosure are separate concerns;
- operational verification remains separate from definition validity.

## Typed facade

The derive facade must remain a metadata/decoding layer over the resolver. It
must not reimplement precedence, selector, or merge logic.

The descriptor grammar remains compositional:

```rust,ignore
.file(path("config.toml").optional().layer("base").when(selector))
.environment(env().prefix("APP"))
.arguments(args())
```

No combinatorial `optional_file_when`/`file_in` style API should return.

## Runtime refresh

Acceptance requires one reusable semantic plan for one-shot and refreshable
resolution. Each plan resolution rereads registered physical sources. Successful
refreshes immediately replace the complete typed value and provenance; invalid
refreshes preserve the current value and retained external inputs. Complete
external reconciliation supports removal, provenance-only changes remain
visible, and sensitive change values and provenance are redacted.

## Security

Acceptance requires secret-safe behavior for library-owned formatting and
reports, including nested/list/map cases. Secret wrappers are redaction helpers,
not secure-memory containers. Serialization remains an explicit data operation
and is not treated as a logging-safe representation.

Syntactically visible `Secret<T>` / `SecretRef` wrappers inside collection types
make the complete setting sensitive. Sensitive collection provenance remains at
the setting boundary and does not expose dynamic map keys, nested paths,
per-item indices, or collection shape. `EffectiveConfiguration` and
`ResolutionReport` `Debug` output must omit raw effective values while their
Serde data contract remains explicitly raw.

Process environment/argument acquisition must not panic on non-Unicode OS
values. Bound malformed values return controlled errors that identify only the
binding or argument position and never echo malformed payload bytes.

## Distribution

The generated `configlab-macros` and `configlab` packages must be independently
assembled. An extracted consumer must compile and run against those packaged
contents with the macro package patched locally before publication.

The main crate package must not leak workspace-only `examples/`, `script/`,
`target/`, or `macros/` directories.

The leakage gate evaluates actual archive files, not directory entries, and
prints the exact offending paths when it fails.

## Documentation and warnings

The gate requires:

- no compiler warnings;
- Clippy with `-D warnings`;
- rustdoc with `-D warnings`;
- compile-fail derive documentation contracts;
- current README/API/security/resolution/compatibility/release documentation.
