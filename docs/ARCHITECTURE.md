# Architecture

`configlab` is one public runtime/library package plus one technical
procedural-macro companion required by Rust's `proc-macro` compilation model.
The architecture is organized by responsibility rather than by artificial Cargo
package boundaries.

## Dependency direction

```text
physical source descriptors / parsing
            ↓
      logical inputs
            ↓
definition → resolver → effective configuration
            ↑
 typed/derive metadata
            ↓
   typed decode/validation
            ↓
       LoadedConfig
            ↓
 optional LiveConfig refresh → current typed value + provenance
```

Critical boundaries:

- physical source code normalizes inputs but never decides precedence;
- the derive macro generates metadata/typed decoding but never resolves values;
- `ConfigBuilder` orchestrates source materialization and delegates semantic
  resolution to the one `Resolver`;
- `LoadedConfig` presents typed output and explainability but does not implement
  validation or merging;
- definition validity and operational verification remain separate concerns.

## Runtime modules

| Area | Responsibility |
| --- | --- |
| `src/definition/` | setting/application/component definitions and invariant validation |
| `src/resolution/` | context, layer order, logical inputs, merging, removals, provenance, diagnostics |
| `src/source/` | file/inline/environment/argument descriptors, parsing, normalization |
| `src/builder.rs` | typed orchestration only |
| `src/typed.rs` | derive-generated typed contract and bindings |
| `src/loaded.rs` | resolved typed value and explainability access |
| `src/plan.rs` | reusable typed plan that reacquires sources for each resolution |
| `src/runtime.rs` | refreshable typed value and redaction-aware change reporting |
| `src/secret.rs` | redacted secret wrappers |
| `src/error.rs` | facade error categories |
| `src/codec.rs` | serde value encode/decode helpers |

## Proc-macro companion

`/macros` contains only compile-time derive functionality. Attribute parsing,
field normalization, type inspection, schema generation, decoding, bindings,
validation, and expansion orchestration are split by reason to change.
Applications depend on `configlab`, not directly on the companion.

## Source semantics

Source mechanism is orthogonal to precedence. Descriptor defaults are facade
conventions only; the resolver sees explicit logical layer metadata. Exact
resolution rules are documented in [`RESOLUTION.md`](RESOLUTION.md).

## Repository layout guarantees

`script/architecture-check.sh` guards high-value structure:

- no alternate runtime-crate or dogfood-only repository layout;
- all maintenance scripts under `/script`;
- examples directly under `/examples`;
- one semantic `Resolver` implementation;
- no combinatorial source-method API;
- examples cannot reach the proc-macro package or hidden implementation internals;
- root `lib.rs` and macro `lib.rs` remain navigation/entrypoint surfaces;
- no TODO/unimplemented markers or project unsafe blocks.

The project intentionally does not enforce arbitrary line-count limits for every
module. A file should have one reason to change; cohesive algorithms may be
larger than simple descriptor modules.
