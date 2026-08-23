# ConfigLab

**A configuration toolkit for Rust.**

ConfigLab is a derive-first toolkit for building typed application configuration
with deterministic resolution, explicit precedence, contextual selectors,
validation, provenance, and secret-safe diagnostics.

ConfigLab is independently owned and maintained by Dominic Maabobra Tuolong.

`0.1.0-alpha.1` is the first public alpha. The core model is intentionally
small and source-agnostic: files, environment variables, command-line
arguments, and inline documents are normalized into logical inputs before the
resolver applies precedence and merge semantics.

> **Alpha status:** the API is usable and heavily tested, but breaking changes
> may occur between alpha releases. See [Compatibility](docs/COMPATIBILITY.md).

## Quick start

```rust
use configlab::{args, env, path, Config, SecretRef};

#[derive(Debug, Config)]
struct AppConfig {
    #[config(default = 8080, env = "APP_PORT", cli = "port")]
    port: u16,

    #[config(default = "info", env = "APP_LOG_LEVEL")]
    log_level: String,

    #[config(env = "APP_DATABASE_PASSWORD")]
    database_password: Option<SecretRef>,
}

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let config = AppConfig::builder()
    .file(path("config.toml").optional())
    .environment(env())
    .arguments(args())
    .load()?;

println!("port = {}", config.port);
# Ok(())
# }
```

For Cargo:

```toml
[dependencies]
configlab = "0.1.0-alpha.1"
```

The `derive`, `toml`, `json`, and `yaml` features are enabled by default. The
runtime model remains usable without the derive feature.

## Source descriptors

Sources are ordinary owned descriptor values. Independent properties compose
without method-name combinations:

```rust
use configlab::{args, env, path, selector, Config};

# #[derive(Config)]
# struct AppConfig { #[config(default = 8080)] port: u16 }
# fn demo() -> Result<(), configlab::ConfigError> {
let loaded = AppConfig::builder()
    .context("environment", "production")
    .context("region", "west-africa")
    .file("base.toml")
    .file(
        path("production.toml")
            .layer("contextual")
            .optional()
            .when(selector!(environment = "production")),
    )
    .file(
        path("west-africa.toml")
            .layer("contextual")
            .optional()
            .when(selector!(
                environment = "production",
                region = "west-africa"
            )),
    )
    .environment(env().prefix("APP"))
    .arguments(args())
    .resolve()?;

println!("safe configuration: {}", loaded.redacted_value());
# Ok(())
# }
```

Inline documents use the same descriptor grammar:

```rust
use configlab::{json, Config};

# #[derive(Config)]
# struct AppConfig { #[config(default = 8080)] port: u16 }
# fn demo() -> Result<(), configlab::ConfigError> {
let config = AppConfig::builder()
    .inline(json("test.json", r#"{"port":9000}"#))
    .load()?;
# assert_eq!(config.port, 9000);
# Ok(())
# }
```

## Resolution model

ConfigLab keeps three concepts independent:

- **source** — where input physically came from;
- **selector** — when that input applies;
- **layer** — how much precedence the input has.

The conventional layer order is:

```text
defaults < base < contextual < local < environment < invocation < override
```

Selector specificity only decides between matching inputs **inside the same
layer**. A higher layer always beats a lower layer, even when the lower input
has a more-specific selector. Source type itself never determines precedence.

Applications that need another policy can declare one explicitly:

```rust
# use configlab::Config;
# #[derive(Config)] struct AppConfig { #[config(default = 1)] value: u8 }
# fn demo() -> Result<(), configlab::ConfigError> {
let builder = AppConfig::builder().layer_order([
    "defaults",
    "system",
    "user",
    "project",
    "environment",
    "invocation",
    "override",
])?;
# let _ = builder;
# Ok(())
# }
```

See [Resolution semantics](docs/RESOLUTION.md) for the exact selector, peer,
merge, removal, and provenance contracts.

## Runtime configuration

Startup-only applications keep the one-shot API. Long-running applications can
retain the same source descriptors in `LiveConfig<T>` and explicitly refresh
their current typed value:

```rust,ignore
let mut config = AppConfig::builder().file("config.toml").live()?;
let report = config.refresh()?;
println!("port = {} ({} changes)", config.port, report.changes().len());
```

Every refresh rereads all registered physical sources, runs the complete
resolver and typed validation pipeline, and replaces the current value only on
success. `RefreshReport` exposes redaction-aware value and provenance changes.
Invalid refreshes leave the current value untouched.

Applications acquire remote data through their own transport and submit complete
source-neutral inputs through `reconcile` or `refresh_with`. ConfigLab owns no
thread, timer, async runtime, network client, control plane, or external side
effect.

See [Runtime refresh](docs/RUNTIME_LIFECYCLE.md).

## Diagnostics and explainability

Use `.load()` when only the typed configuration is needed. Use `.resolve()`
when the application also needs diagnostics, provenance, redacted output, or an
explanation of why a setting has its effective value.

Machine integrations should use stable descriptive categories rather than
parsing human-readable text:

- `ConfigError::code()`;
- `ResolveError::code()`;
- `DiagnosticCode::as_str()`.

## Secrets

`Secret<T>`, `SecretRef`, sensitive metadata, provenance views, and source
descriptor `Debug` implementations are designed to reduce accidental secret
disclosure. They are **redaction tools**, not secure-memory containers or secret
store clients. See [Security](docs/SECURITY.md).

Secret wrappers remain sensitive when nested inside visible collections such as
`Vec<Secret<String>>`. Library-owned `EffectiveConfiguration` and
`ResolutionReport` `Debug` output omits raw configuration values; Serde
serialization remains an explicit raw-data operation rather than a logging API.

## Examples

The examples are real workspace consumers of the public API:

- [`examples/basic-app`](examples/basic-app) — minimal file/env/CLI loading;
- [`examples/cli-tool`](examples/cli-tool) — optional project config, lists,
  validation, and an optional secret reference;
- [`examples/contextual-service`](examples/contextual-service) — nested service
  configuration, contextual selectors, cross-section validation, provenance,
  and redacted diagnostics;
- [`examples/runtime-service`](examples/runtime-service) — repeated file
  refresh, invalid-update recovery, secret-safe changes, provenance, and
  optional-source removal;
- [`examples/platform-integration`](examples/platform-integration) — manual
  schemas, reusable components, logical-input adapters, merge/removal behavior,
  diagnostics, provenance, and operational verification without derive macros.

See [`examples/README.md`](examples/README.md) for the learning path.

## Project direction

ConfigLab is intended to grow into a broader configuration ecosystem while
keeping the resolver model deterministic and small. Candidate future areas
include reusable configuration components such as server, database, logging,
TLS, retry, and metrics definitions, plus schema/introspection and tooling.
Those are roadmap directions, not APIs promised by this alpha.

See [Ecosystem direction](docs/ECOSYSTEM.md).

## Development

The repository pins development verification to Rust `1.97.1`; the package MSRV
is the Rust `1.97` line.

```bash
./script/refresh-lock.sh
# Review Cargo.lock intentionally.
cargo fmt --all
./script/check.sh
```

`script/check.sh` covers architecture, formatting, tests, compile-fail docs,
Clippy with warnings denied, rustdoc, feature isolation, release compilation,
all examples, generated package archives, an independent packaged consumer,
and a non-threshold performance baseline.

## Documentation

- [API guide](docs/API.md)
- [Resolution semantics](docs/RESOLUTION.md)
- [Configuration model](docs/KERNEL_MODEL.md)
- [Runtime refresh](docs/RUNTIME_LIFECYCLE.md)
- [Diagnostics](docs/DIAGNOSTICS.md)
- [Security](docs/SECURITY.md)
- [Compatibility policy](docs/COMPATIBILITY.md)
- [Testing](docs/TESTING.md)
- [Dependencies](docs/DEPENDENCIES.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Ecosystem direction](docs/ECOSYSTEM.md)
- [Release process](docs/RELEASING.md)
- [Changelog](CHANGELOG.md)

## License

Licensed under the [BSD 3-Clause License](LICENSE).
