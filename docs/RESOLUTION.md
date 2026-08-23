# Resolution semantics

This document is the normative implementation guide for precedence, peer
compatibility, merge behavior, removals, and provenance in `configlab`.
Physical source type is deliberately absent from these rules: files,
environment variables, command-line arguments, inline documents, and advanced
logical inputs all normalize to the same source-neutral model before
resolution.

## Resolution order

Resolution uses three independent concepts:

1. **Selector applicability** — an input applies only when every exact selector
   attribute matches stable context.
2. **Selector specificity inside one layer** — more matching selector attributes
   win over less-specific candidates in the same layer.
3. **Layer precedence** — after same-layer specificity is resolved, a higher
   declared layer always outranks a lower layer.

Selector specificity never jumps across layers. Source type never creates
precedence.

## Equal-precedence peers

Two contributions are peers when they target the same setting in the same layer
at the same selector specificity. Peers must either be provably compatible or
resolution fails with `ResolveError::Ambiguous`.

Compatibility is intentionally based on the operation's semantics, not merely
on whether the operation enum variant happens to match.

### Set peers

- `Replace`: values must be identical.
- `Deep`: disjoint object leaves may combine recursively; a differing value at
  the same leaf is ambiguous.
- `CombineByKey`: disjoint direct map keys may combine; the same direct key must
  contain an identical value. Nested data inside one key is one atomic entry and
  is **not** recursively merged.
- `Append` / `Prepend`: equal-specificity peers require explicit unique order,
  because order is part of the effective value.

### Removal peers

For non-accumulating settings:

- `Unset` + `Unset` is compatible and idempotent.
- `Clear` + `Clear` is compatible and idempotent.
- `RemoveMapEntry` peers are compatible for both identical and disjoint keys;
  map-entry removal is idempotent and commutative.
- `RemoveListItem` peers for distinct values are compatible because those
  removals commute.
- duplicate `RemoveListItem` peers for the same value are ambiguous. Removing
  one matching item is multiplicity-sensitive when duplicate list values exist,
  so two peer declarations must not silently consume two occurrences.
- different removal kinds conflict with one another.
- any equal-precedence `Set` mixed with a removal conflicts.

For `Append` / `Prepend`, the existing explicit-order rule remains authoritative
for multiple equal-specificity peer contributions because accumulation order
may be observable.

## Merge policies

`ValueShape::Any` is deliberately limited to `Replace`. Structural merging and
collection-specific removal require an explicit shape so definition validation
can guarantee that every accepted operation is executable.

### Replace

The winning applicable value replaces the previous value as one unit.

### Deep

`Deep` recursively combines object fields. Higher layers overlay lower layers
recursively, while equal-precedence conflicting leaves are ambiguous.

Example:

```text
base:       { database: { host: "db", port: 5432 } }
contextual: { database: { port: 6432 } }
result:     { database: { host: "db", port: 6432 } }
```

### CombineByKey

`CombineByKey` operates at the **direct named-map entry boundary**.

Different keys coexist:

```text
base:       { console: { format: "text" } }
contextual: { audit:   { format: "json" } }
result:     { console: { format: "text" }, audit: { format: "json" } }
```

A higher-precedence value for an existing key replaces that key's value as one
unit:

```text
base:       { console: { enabled: true, format: "text" } }
contextual: { console: { format: "json" } }
result:     { console: { format: "json" } }
```

The missing `enabled` field does not survive. Applications that want recursive
composition inside the value must model that value with its own definition or
use `Deep` at the appropriate object boundary.

### Append and Prepend

These policies accumulate ordered-list items, but they use the same selector
specificity rule as every other merge policy. Within one layer, only applicable
contributions at the maximum selector specificity participate; less-specific
contributions are retained as `Shadowed` provenance and do not contribute
items. Multiple winning equal-specificity peers require explicit unique
`order` values so the resolver never derives list order from source discovery
or iteration order.

Accumulation still occurs across explicit layers. `Append` adds the winning
contribution for each layer after lower-layer material; `Prepend` places the
winning contribution for each higher layer before lower-layer material.

## Omission, null, unset, clear, and item removal

These are distinct states/operations:

- **omission**: the input contributes nothing;
- **null**: an explicit value allowed only by nullable replace semantics;
- **unset**: removes the complete setting and suppresses its current value;
- **clear**: retains an empty collection;
- **remove map entry**: removes one named-map key;
- **remove list item**: removes one matching ordered-list value.

Removal operations are rejected unless the setting's `RemovalPolicy` explicitly
permits them.

## Provenance

Provenance follows the same semantic boundary as merging:

- replace settings record setting-level contributions;
- deep-merged objects record nested leaf provenance;
- combine-by-key maps record provenance at each direct map entry;
- append/prepend lists record per-item provenance and maintain item indices
  across prepend/removal operations.

For `Disclosure::Sensitive` settings, provenance is intentionally coarser than
the ordinary structural boundary: the resolver records setting-level
contributions only. It does not create dynamic named-map key paths, nested
object paths, or per-item list indices, because those structures may themselves
disclose sensitive information. Contributed values are redacted before
provenance reaches public explainability APIs.

Dynamic object and named-map keys escape `\\` as `\\\\` and `.` as `\\.` in
provenance paths. `provenance_child_path` applies this encoding for callers of
`explain`.

## Resolution information vs diagnostics

Expected resolution outcomes such as selector misses, defaults, higher-layer
replacement, and no-op removals are `ResolutionInformation`, not diagnostics.

No-op removal information stores a structured `RemovalKind` (`map_entry` or
`list_item`) rather than interpolating the target map key/list value into a
human string. This keeps normal resolution information useful without creating
an accidental payload-disclosure surface.

Diagnostics describe definition/validation problems and have error or warning
severity. Operational verification remains a separate report from definition
validity.
