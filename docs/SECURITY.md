# Security model

## Secret boundary

The library treats secret **references** as configuration. It does not fetch,
store, rotate, or manage secret payloads.

`SecretRef` and `Secret<T>` redact `Debug` and `Display`. Definition metadata
also carries independent `Representation` and `Disclosure` classifications so
a value may be a secret reference and sensitive at the same time.

`Secret<T>` deliberately provides no equality operation. Ordinary `PartialEq`
on an arbitrary wrapped type cannot promise constant-time authentication;
applications must expose the value only at the relevant trust boundary and use
a domain-appropriate constant-time verifier. `SecretRef` retains equality
because it represents an identifier rather than secret payload material.

The derive facade treats a setting as sensitive when its syntactically visible
Rust type contains `Secret<T>` or `SecretRef` anywhere in the type tree. This
includes collection element/value positions such as `Vec<Secret<String>>` and
`BTreeMap<String, SecretRef>`. Because procedural macros cannot reliably see
through arbitrary Rust type aliases, an alias that hides the secret wrapper
must use `#[config(sensitive)]` explicitly.

## Redaction paths

Sensitive values are redacted from:

- public effective-configuration output;
- provenance/explanations;
- typed decode errors;
- kernel invalid-value errors surfaced through the facade.

Sensitive collections are treated as one disclosure unit in provenance. Their
dynamic map keys, nested keys, per-item indices, item values, and collection
shape are not expanded into public provenance paths. The setting-level
contribution remains visible with a redacted value so support tooling can still
explain which source/layer affected the setting.

Inline, environment, and argument source descriptors implement redacted
`Debug`; payload text/values/arguments are not emitted.

Redaction is a disclosure control, not secure-memory erasure. Descriptor-owned
strings and resolved secret references remain ordinary process memory for as
long as their owners live; the crate does not zeroize arbitrary strings or
claim protection against process-memory inspection.

`#[config(sensitive)]` protects library-generated output; it cannot change the
application type's own `Debug` or `Display` implementation. Prefer `Secret<T>`
or `SecretRef` when a value must remain redacted even if the containing Rust
struct is formatted by application code.

Serde serialization is a data interface, not a redacted logging interface.
`Secret<T>` serializes its wrapped value and `SecretRef` serializes its reference
identifier so configuration encoding/decoding can work. Do not serialize a typed
configuration directly into logs or support bundles when it contains sensitive
fields. Use `LoadedConfig::redacted_value()`, `explain()`, or other library
redaction paths for diagnostic output.

`Debug` for `EffectiveConfiguration` and `ResolutionReport` is library-owned and
log-safe: it deliberately omits raw effective values and detailed provenance
contents. Their Serde representations remain intentionally raw data interfaces.
This distinction is tested so adding a new raw field cannot silently turn the
normal `Debug` path into a secret-bearing log surface.

Selector/context values are descriptive metadata, not secret storage; do not
place credentials or tokens in context attributes.

Process environment and argument acquisition uses OS-native string values and
per-binding lookup. A non-Unicode value for a configured environment binding or
a non-Unicode process argument produces a controlled `ConfigError` that names
only the binding/argument position; malformed payload bytes are never rendered.
Unrelated malformed environment entries are never enumerated by ConfigLab.

Default `ResolutionLimits` bound document bytes, scalar bytes, argument/input/
operation counts, aggregate logical bytes, value nesting depth, and total value
nodes. These limits are
applied before parsing where possible and again at the logical resolver
boundary. Applications with generated configurations may supply reviewed
limits through `ConfigBuilder::limits` or `Resolver::with_limits`.

## Runtime configuration

External logical inputs are raw data interfaces. Their derived `Debug` and Serde
representations contain operations and must not be used as logging shortcuts.

Refresh change sets omit old/new values for sensitive settings and replace any
provenance values with `[REDACTED]`. Origins and logical-input IDs remain public
operational metadata and therefore must not contain credentials.

`LiveConfig` replaces its current typed value and provenance only after every
source is acquired and the complete resolution and validation pipeline succeeds.
Failure preserves the current value. ConfigLab cannot roll back arbitrary
application side effects; applications decide when to refresh and how consumers
react to the returned `RefreshReport`.

## Errors and identifiers

`ConfigError::code`, `ResolveError::code`, and `DiagnosticCode::as_str` expose
stable descriptive categories without embedding input values. Human-readable
messages remain free to improve without forcing callers to parse text. Error
codes do not weaken redaction boundaries.

## Parser errors

TOML/YAML parse errors deliberately avoid echoing complete source text. JSON
errors report line/column rather than source payload. Unknown-field diagnostics
name paths, not values.

## Unsafe code

The main library, proc-macro companion, and executable examples forbid unsafe
code. `script/architecture-check.sh` also rejects unsafe blocks in project
source.

## Dependency and supply-chain review

Direct dependency requirements allow compatible ecosystem updates while the
reviewed repository resolution remains pinned in `Cargo.lock`.
`serde_yaml_ng` is the selected YAML implementation,
but its parser stack currently includes `unsafe-libyaml`; enabling the `yaml`
feature therefore carries that transitive native-parser maintenance surface.
Projects that do not need YAML can disable the feature.

The repository does not claim that static source review is a vulnerability
audit. Before promotion or dependency refresh, review `Cargo.lock` and run an
advisory scanner such as `cargo audit` in an environment where that tool is
managed by the consuming organization. GitHub Actions and GitLab CI pin and run `cargo-audit` in
its separate security stage; `script/check.sh` deliberately does not
auto-install or auto-update external audit tooling during local compilation.

## Remaining responsibility

Secrets still exist in application memory when the application explicitly
stores them as ordinary values or calls `expose()`. This library is not secure
memory and is not a secret manager. Application-provided validation messages
must not interpolate secret material.
