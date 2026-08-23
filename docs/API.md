# Public API guide

The primary API is derive-first and intentionally small. Advanced definition and
logical-input types remain available for integrations, but ordinary applications
should not need to construct the resolver manually.

## Builder entry point

`Config::builder()` is the single typed entry point:

```rust
AppConfig::builder()
    .file("config.toml")
    .environment(env())
    .arguments(args())
    .load()?;
```

There are three one-shot terminal operations:

- `.load()` resolves, validates, decodes, and returns only `T`;
- `.resolve()` performs the same work but returns `LoadedConfig<T>` with
  provenance, diagnostics, warnings, stable context, redacted effective output,
  and the validated schema.
- `.resolve_report()` preserves all blocking kernel diagnostics and typed
  validation issues in `ConfigResolutionReport<T>` instead of reducing them to
  the first compatibility error.

`load` has one meaning in the typed API: the terminal operation that returns
the decoded configuration.

## Reusable plans and refreshable values

`.plan()` compiles schema, source descriptors, validators, context, and layer
policy into `ConfigPlan<T>`. The plan does not acquire physical sources until
resolution; every `resolve()` call rereads them.

`.live()` resolves an initial value and returns `LiveConfig<T>`:

```rust,ignore
let mut config = AppConfig::builder()
    .file("config.toml")
    .environment(env())
    .live()?;

let report = config.refresh()?;
println!("{} setting(s) changed", report.changes().len());
```

`refresh()` rereads every registered physical source. `reconcile(inputs)` and
`refresh_with(inputs)` additionally replace the complete external logical-input
contribution. Resolution, typed validation, value replacement, and provenance
replacement are transactional: failure preserves the current value and retained
external inputs.

See [`RUNTIME_LIFECYCLE.md`](RUNTIME_LIFECYCLE.md) for refresh, reconciliation,
change-reporting, source-removal, and concurrency contracts.

## File source

A bare path is a required file in the conventional `base` layer:

```rust
.file("config.toml")
```

Use `path(...)` when the source needs independent modifiers:

```rust
.file(
    path("production.toml")
        .layer("contextual")
        .optional()
        .when(selector!(environment = "production"))
        .order(10),
)
```

`optional()` only changes missing-file behavior. It never changes precedence.

## Environment source

```rust
.environment(
    env()
        .layer("environment")
        .prefix("ORDERS")
        .separator("__")
)
```

Exact `#[config(env = "NAME")]` bindings are never modified by prefix or
separator settings. Tests can avoid process-global mutation:

```rust
.environment(env().from([("ORDERS_PORT", "9090")]))
```

Process-backed environment sources look up only the generated bindings through
the OS-native environment API. ConfigLab therefore does not enumerate unrelated
environment entries, preserves platform-native lookup behavior, and returns a
controlled `ConfigError::Environment` if a configured binding contains a
non-Unicode value. Deterministic `.from(...)` maps remain ordinary
case-sensitive Rust maps.

## Arguments source

```rust
.arguments(args().layer("invocation"))
```

Tests can supply deterministic arguments:

```rust
.arguments(args().from(["--port", "9090"]))
```

Argument parsing is strict by default: unknown flags, unexpected positional
arguments, and duplicate configuration paths are errors. Applications that
share `argv` with another parser must opt into pass-through behavior explicitly
with `args().ignore_unknown()`.

Process arguments are acquired as OS strings and converted explicitly. A
non-Unicode argument returns `ConfigError::Argument` naming only its 1-based
position after the executable name; ConfigLab does not render malformed
argument payload bytes into the error.

## Inline source

```rust
.inline(toml("test.toml", "port = 9090"))
.inline(json("test.json", r#"{"port":9090}"#).layer("contextual"))
```

Inline descriptor `Debug` output redacts the document text.

## Layers and selectors

Use `layer_order(...)` only when the conventional precedence policy is not the
application's intended policy. Layer names are validated before source I/O, so
a misspelled layer is an error even when an associated file is optional and
missing.

Selectors decide applicability and same-layer specificity; they do not outrank
higher layers. See [`RESOLUTION.md`](RESOLUTION.md) for the normative rules.

## Resource limits

`ResolutionLimits::default()` bounds physical documents and scalars as well as
logical input count, operation count, aggregate logical bytes, value depth, and
value nodes. Override it
only with an explicitly reviewed policy:

```rust
# use configlab::{Config, ResolutionLimits};
# #[derive(Config)] struct App { #[config(default = 1)] value: u16 }
# fn demo() -> Result<(), configlab::ConfigError> {
let limits = ResolutionLimits {
    max_document_bytes: 32 * 1024 * 1024,
    ..ResolutionLimits::default()
};
let builder = App::builder().limits(limits)?;
# let _ = builder;
# Ok(())
# }
```

## Explainability and safe output

```rust
let loaded = AppConfig::builder()
    .file("config.toml")
    .resolve()?;

let provenance = loaded.explain("server.port");
let safe = loaded.redacted_value();
```

Do not use generic serde serialization of a typed configuration as a logging
interface when it may contain sensitive values. `redacted_value()` is the
library-owned safe output path.

`Debug` for low-level `EffectiveConfiguration` and `ResolutionReport` is also a
safe library-owned presentation surface: raw values and detailed provenance are
omitted. Serde serialization of those types intentionally remains raw data.

Secret wrappers nested inside visible collection types automatically make the
whole setting sensitive, for example `Vec<Secret<String>>` and
`BTreeMap<String, SecretRef>`. If a Rust type alias hides the wrapper from the
derive input, add `#[config(sensitive)]` explicitly.

## Stable error categories

`Display` is for humans. Code should use stable descriptive categories:

```rust
match error.code() {
    "unknown_layer" => { /* configuration declaration problem */ }
    "parse" => { /* physical source syntax problem */ }
    _ => {}
}
```

Kernel diagnostics expose `DiagnosticCode::as_str()` for the same reason.

## Raw logical inputs

`ConfigBuilder::input(LogicalInput)` remains available for advanced integrations
that already produce source-neutral inputs. Such inputs still pass through the
same resolver; they do not bypass precedence, selector, merge, validation, or
redaction semantics.

Low-level path-tree mutation helpers are internal implementation details. Use
`EffectiveConfiguration::get`, `LoadedConfig::explain`, and the definition/
logical-input APIs rather than depending on resolver internals.

Dynamic object and map keys in provenance use backslash escaping so literal
dots cannot collide with structural path separators. Use
`provenance_child_path(parent, key)` when constructing a dynamic explanation
path.
