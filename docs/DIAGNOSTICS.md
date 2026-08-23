# Diagnostics, resolution information, provenance, and verification

The project deliberately keeps four forms of explanation separate because they
answer different questions.

## 1. Diagnostics: "Is the candidate definition-valid?"

`Diagnostic` is produced by kernel definition validation over the complete
candidate.

A diagnostic has:

- `DiagnosticSeverity` (`Error` or `Warning`);
- a stable `DiagnosticCode`;
- an optional primary path;
- all participating subject paths;
- a human-readable message.

Current codes are:

- `MissingRequired` (`missing_required`)
- `InvalidType` (`invalid_type`)
- `ConstraintViolation` (`constraint_violation`)
- `InvalidRepresentation` (`invalid_representation`)
- `RuleViolation` (`rule_violation`)
- `DeprecatedSetting` (`deprecated_setting`)

`DiagnosticCode::as_str()` returns the stable descriptive identifier in
parentheses. `ResolveError::code()` and `ConfigError::code()` provide the same
pattern for fail-fast resolver and facade failures. Human-readable `Display`
text is not a machine protocol and may improve independently.

`ConfigError::Resolve` retains a concrete `ResolveError` and exposes it through
`std::error::Error::source`, so downstream error-reporting crates preserve that
chain. Other facade variants intentionally retain redacted, stable strings
rather than raw parser/decoder errors. They therefore have no downcastable
source; exposing raw parser chains could reintroduce sensitive input details
that the facade deliberately omits.

An error blocks successful effective configuration. A warning does not.

`Resolver::resolve_report()` is the structured API. `Resolver::resolve()` is a
fail-fast compatibility convenience that maps the first blocking diagnostic to
a `ResolveError`.

### Validation phases

Diagnostics follow the model order:

```text
value -> section -> component -> application
```

If value validation produces an error, relationship rules are skipped. This
avoids secondary messages based on malformed or missing primitives.

## 2. Resolution information: "What normal event happened?"

`ResolutionInformation` is not an error/warning channel.

Examples:

- `InputIgnored` because an exact selector did not match;
- `DefaultUsed`;
- `DerivedDefaultUsed`;
- `ValueOverridden` by a higher explicit layer;
- `OptionalOmitted`;
- `RemovalNoop`.

These are useful in traces and support tooling even when the configuration is
perfectly valid.

`RemovalNoop` carries a structured `RemovalKind` rather than the removed key or
item value. Target payloads belong to raw input data, not normal resolution
information.

## 3. Provenance: "Where did this effective value come from?"

`EffectiveConfiguration::explain(path)` returns ordered `ProvenanceEntry`
records.

For replace-style values this shows the winning contribution plus shadowed or
prior contributions. For accumulated lists, each item carries its final index.
For `Deep`, nested leaves may have independent provenance. For
`CombineByKey`, provenance is recorded at the direct named-map entry boundary,
matching the policy's atomic key semantics.

A provenance entry includes its `InputTarget`, so a contribution can be traced
to a root input or to a specific composed component.

Sensitive values and secret references are redacted before they are stored in
public provenance. Sensitive collection structure is also treated as protected:
dynamic map keys, nested value paths, and per-item indices are not expanded into
public provenance. The setting-level source/layer/action remains explainable.

## 4. Operational verification: "Can this installation use it?"

Operational verification is a separate optional report. It answers questions
such as whether a certificate can be opened or a database can be reached.

A `VerificationReport` groups the individual checks, but it is never embedded
into `ResolutionReport`. A configuration may be definition-valid while an
operational check fails or was never performed. Operational checks therefore
never replace or mutate kernel definition diagnostics, and the kernel does not
impose a startup/refresh policy on their outcomes.

## Redaction rules

Redaction is enforced from definition metadata, not presentation convention.

A setting is redacted when either:

- `Disclosure::Sensitive`, or
- `Representation::SecretReference`.

The typed facade also uses `SecretRef` and `Secret<T>` to make `Debug` and
`Display` redacting by default.

`EffectiveConfiguration` and `ResolutionReport` have explicit safe `Debug`
implementations that omit raw effective values and detailed provenance. Their
Serde serialization remains a raw data interface and must not be used as a
logging shortcut.

Diagnostic messages and custom typed validators must not interpolate secret
material. The kernel's built-in errors report path/type/constraint information,
not the offending sensitive payload.

## Example

A successful resolution may conceptually contain:

```text
effective:
  server.port = 9090

diagnostics:
  WARNING old_logging_format is deprecated

information:
  server.port was overridden from base by invocation
  database.port used a derived default from database.engine

provenance:
  server.port <- invocation / --port
```

A blocking result instead has `effective = None` and one or more error
`Diagnostic` records.
