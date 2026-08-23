# Performance model

The startup-only path adds no runtime coordination work. Runtime configuration
is explicit and in-process; it reuses the same resolution model without adding
a background service or async-runtime dependency.

## Runtime work

A one-shot builder load performs, in order:

1. definition construction/validation;
2. physical source reads/parsing;
3. source normalization to logical inputs;
4. selector filtering and deterministic resolution;
5. type/constraint/requirement diagnostics;
6. typed serde decoding;
7. application-specific typed validation.

`ConfigBuilder::plan()` performs definition construction and validation once
and stores the resulting validated `Resolver` in `ConfigPlan`. Reusing a plan,
including through `LiveConfig`, does not revalidate or reclone its immutable
schema and layer policy. It still runs definition-driven candidate diagnostics
on every resolution because those depend on the newly acquired values. The
validated derived-default dependency order is also retained by the resolver.

There is no runtime reflection, registry, plugin dispatch, background thread,
filesystem notification backend, or async runtime.

`ConfigPlan` reacquires registered physical sources on every resolution. Each
`LiveConfig` refresh resolves those sources plus the retained complete external
inputs, decodes and validates the typed value, and computes setting-level value
and provenance changes.
This intentionally favors correctness and bounded deterministic work over
incremental mutation or caching.

`LiveConfig` imposes no lock or shared-state primitive. Reading borrows the
current typed value; refresh requires exclusive mutable access. Applications
choose their own lock, actor, channel, or dependency-injection boundary when
cross-thread sharing is required.

## Allocation behavior

The canonical configuration tree is `serde_json::Value`. Logical inputs and
provenance are owned because they must survive long enough for deterministic
resolution/explanation. Source descriptors are ordinary structs and use no
trait-object dispatch. The only intentional dynamic dispatch in the facade is
for optional user-supplied typed validators (`Box<dyn Fn...>`).

Environment variables and process arguments are collected once when their
process-backed source is materialized. Deterministic `.from(...)` descriptors
are borrowed during materialization rather than cloning their entire payload.
Files are read once per resolution.

## Merge complexity

`Deep` recursively visits object fields that participate in the incoming value.
`CombineByKey` is intentionally shallower: it operates at direct named-map keys
and replaces a colliding entry as a unit. This both matches its semantic model
and avoids unnecessary recursive work for map entries.

## Compile-time derive cost

The proc macro executes at compile time and emits ordinary Rust implementations.
It does not introduce runtime reflection.

## Measurement baseline

`script/performance-baseline.sh` runs an ignored release-mode test that resolves
100 settings across multiple layers for repeated iterations and prints total and
per-resolution wall-clock timing. A second probe evaluates complete live
reconciliation through typed decoding and change-set construction for
the same setting count. They deliberately assert no threshold because machine
load and CI scheduling make absolute timing unsuitable as a correctness gate.

Performance changes should be justified by comparable measurements from this
baseline or a more focused benchmark. The project does not claim throughput or
latency guarantees yet, and it should not add caching or lifetime complexity
without evidence of a real bottleneck.
