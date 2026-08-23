# Runtime refresh

ConfigLab supports long-running applications through one mutable typed value,
explicit source refresh, and source-neutral reconciliation. It does not model a
control plane, deployment proposal, application adoption decision, or historical
snapshot store.

## Model

```text
registered physical sources ─┐
                             ├─ acquire → normalize → resolve → validate
complete external inputs ────┘                              │
                                                            ├─ failure: keep current value
                                                            └─ success: replace value and provenance
                                                                          │
                                                                  RefreshReport
```

`LiveConfig<T>` owns:

- a reusable `ConfigPlan<T>`;
- the current validated `LoadedConfig<T>`;
- the complete external logical inputs from the last successful reconciliation.

It does not own a thread, timer, filesystem watcher, network client, async
runtime, rollout controller, or component-restart policy.

## Construction

```rust,ignore
let mut config = AppConfig::builder()
    .file(path("base.toml").layer("base"))
    .file(path("local.toml").layer("local").optional())
    .environment(env())
    .arguments(args())
    .live()?;
```

`live()` compiles the schema and source policy, acquires all registered sources,
resolves and validates the initial value, and returns an error if no valid
initial configuration can be produced.

`ConfigBuilder::plan()` performs only the compilation step. `ConfigPlan::resolve`
and `ConfigPlan::resolve_with_inputs` reacquire physical sources on every call.

## Physical refresh

```rust,ignore
let report = config.refresh()?;
if report.changed() {
    for change in report.changes().changes() {
        println!("{}: {:?}", change.path(), change.kind());
    }
}
```

`refresh()` rereads every registered physical source. A changed or removed
optional file, changed process environment binding, or other current source
state therefore participates in one complete resolution.

Inline documents, provided environment maps, provided argument vectors, and
direct builder overrides are retained descriptor values. They participate in
every refresh but naturally remain unchanged unless a new plan is built.

## External reconciliation

Remote transport belongs to the application:

```rust,ignore
let remote_document = api_client.fetch_configuration().await?;
let inputs = adapter::normalize(remote_document)?;
let report = config.reconcile(inputs)?;
```

`reconcile(inputs)` treats `inputs` as the complete current external
contribution. It rereads registered physical sources in the same transaction.
On success, those inputs are retained for later `refresh()` calls. Supplying an
empty collection removes every external contribution.

`refresh_with(inputs)` is an alias that emphasizes the combined physical refresh
and external reconciliation. `clear_external()` is the explicit convenience for
reconciling an empty external contribution.

ConfigLab does not attach revisions, provider identities, polling, authentication,
retry, streaming, caching, or durability semantics. Integrations express source
identity through each `LogicalInput`'s stable ID and `Origin`.

## Transactional replacement

A refresh performs these steps before mutating the current value:

1. acquire every registered source;
2. normalize all source values into logical inputs;
3. enforce resource limits;
4. resolve selectors, layers, peers, merge policies, removals, and defaults;
5. decode the complete typed value;
6. run definition-driven candidate diagnostics and application validation;
7. calculate redaction-aware value and provenance changes.

Only a completely successful result replaces the current `LoadedConfig<T>` and
external input set. Parse, I/O, resource-limit, resolution, decode, or validation
failure returns `ConfigError` and leaves both unchanged.

The immutable schema and layer policy were already validated when the
`ConfigPlan` was compiled. Refresh reuses that validated resolver; it does not
repeat schema validation or cache source data/effective results.

This all-or-nothing assignment is an internal safety property, not a public
staged publication lifecycle.

## Changes and provenance

`RefreshReport` contains a lexical `ChangeSet`. Each `ConfigurationChange`
records:

- setting path and optional owning component;
- `Added`, `Removed`, `Modified`, or `ProvenanceChanged`;
- previous and current values when the setting is not sensitive;
- previous and current provenance traces with sensitive values redacted.

An equal value with a different origin is a `ProvenanceChanged` event. This lets
applications record that an environment value replaced a file value even when
the decoded value remained equal.

`LiveConfig::loaded()` exposes current diagnostics, resolution information,
redacted output, and `LoadedConfig::explain(path)` just like one-shot resolution.
ConfigLab keeps current provenance but does not retain an unbounded history;
applications may log or persist each `RefreshReport`.

## Precedence

Refresh does not introduce another precedence system. The ordinary resolver
continues to compare:

```text
explicit layer rank
→ selector specificity inside one layer
→ explicit peer order for order-sensitive accumulation
→ ambiguity error when equal peers conflict
```

Source registration order never silently chooses a scalar winner. Applications
declare precedence through named layers and may replace the conventional layer
order through `ConfigBuilder::layer_order`.

## Access and concurrency

`LiveConfig::value()` and `Deref` borrow the current `T`. `refresh` and
`reconcile` require `&mut self`, so Rust prevents a caller from retaining a
reference across replacement.

ConfigLab deliberately does not impose a shared-state primitive. An application
that needs cross-thread access may place `LiveConfig<T>` behind its chosen lock,
actor, channel, or dependency-injection boundary. That application also decides
when to call `refresh` and how downstream components react to returned changes.

## Source removal and recovery

A missing required file fails refresh. A missing optional file contributes no
inputs, so values previously supplied by that file are recomputed from remaining
sources and defaults. A partial or invalid write fails safely; a later refresh
can recover after the writer completes.

## Out of scope

ConfigLab runtime refresh does not provide:

- HTTP, gRPC, database, or message-bus clients;
- a remote configuration server or control plane;
- background file watching or scheduling;
- distributed rollout or revision coordination;
- application component restart or preparation;
- automatic external side effects;
- historical configuration storage;
- secure-memory erasure or secret-store retrieval.
