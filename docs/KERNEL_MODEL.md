# Kernel definition and resolution model

`configlab`'s definition/resolution modules are the single semantic authority for configuration resolution.
It does not know whether an input came from TOML, JSON, YAML, environment
variables, command-line arguments, tests, or another delivery adapter.

The kernel is intentionally data-driven. Definition variation is represented
by enums and structs rather than callbacks, plugins, or runtime reflection.

## Resolution boundary

The pipeline is:

```text
ResolutionRequest
    -> discovery / context establishment
    -> stable Context + LogicalInput[]
    -> deterministic resolution
    -> complete-candidate definition validation
    -> ResolutionReport
    -> optional operational verification
    -> optional live refresh/reconciliation (outside the kernel)
```

`ResolutionRequest` contains raw facts available before selection, such as a
working directory or profile argument. `Context` contains only attributes that
are stable before selector evaluation. A configuration input cannot provide a
context value that decides whether that same input applies.

## Logical inputs and targets

A `LogicalInput` owns:

- a stable input id;
- an explicit layer name;
- an `InputTarget` (`Root` or a composed component key);
- an exact-match `Selector`;
- optional deterministic peer order for accumulation;
- an `Origin`;
- ordered `Operation` values.

Component-targeted operations use paths relative to the component's root.
The resolver looks up that root from compiled application metadata. Unknown
component keys fail before ordinary value resolution.

Source mechanism never contributes precedence.

## Layers, selectors, and ambiguity

`LayerOrder` is the primary precedence axis. Higher layers always beat lower
layers. Inside one layer, an applicable input with a selector matching more
context attributes is more specific.

Specificity never crosses a layer boundary.

Two applicable inputs that affect the same replace-style value in the same
layer at equal specificity must agree. A disagreement is
`ResolveError::Ambiguous`; incidental file order, discovery order, map order,
or load timing is never used as a hidden tie-breaker.

For append/prepend accumulation, equal-layer/equal-specificity peers require an
explicit `LogicalInput::ordered(...)` value. Missing or duplicate peer order is
`ResolveError::UndefinedOrder`.

This ordering rule is applied only after selector specificity has selected the
winning peer tier. Less-specific append/prepend candidates are shadowed just as
they are for other merge policies; they cannot force an ordering error or
contribute items to the effective value.

## Setting definitions

Each `SettingSpec` describes a single effective setting path.

Independent setting paths may not overlap by ancestry. A schema cannot define
both `service` and `service.port`, in either insertion order, because those
settings would compete for the same canonical value-tree location. Such overlap
is rejected atomically during `Schema::insert`.

### Shape, semantic type, and combination are separate

`ValueShape` describes structure:

- `Scalar`
- `Object`
- `OrderedList`
- `NamedMap`
- `Any`

`ValueType` describes a scalar value or the element/value type of a collection:

- `Boolean`
- `Integer`
- `UnsignedInteger`
- `Float`
- `Text`
- `Duration`
- `ByteSize`
- `Path`
- `Address`
- `Enumeration`
- `Any`

`MergePolicy` independently describes cross-input combination:

- `Replace`
- `Deep`
- `Append`
- `Prepend`
- `CombineByKey` — combines direct named-map entries by key; a colliding key is an atomic entry and is not recursively deep-merged

The schema validator rejects invalid shape/policy combinations before a
resolver is constructed.

`Deep` and `CombineByKey` are intentionally different. `Deep` recursively
combines object children. `CombineByKey` operates at the direct map-entry
boundary: disjoint keys coexist, while a higher-precedence value for an
existing key replaces that entry as one unit. See `docs/RESOLUTION.md` for the
peer-compatibility and removal-operation matrix.

### Canonical scalar representation

The current kernel uses `serde_json::Value` as its internal transport-neutral
value tree. `Integer` and `UnsignedInteger` support the full `i128`/`u128`
range through serde_json arbitrary-precision numbers.

Numeric `<` / `<=` conditions preserve that width when comparing integer and
floating-point settings. Integer/integer comparisons are exact, and mixed
integer/`f64` ordering compares against the float's exact binary value rather
than first rounding the integer through `f64`.

`Duration`, `ByteSize`, `Path`, and `Address` are nominal text-like semantic
types at this stage. The kernel guarantees that the canonical value is text,
but deliberately does not impose one global duration syntax, byte-size syntax,
path grammar, or network-address grammar. Applications that require a specific
canonical syntax should add a constraint/validation rule or typed facade
validation until a language-neutral normalization contract is agreed.

### Requirements and conditions

`Requirement` is one of:

- `Optional`
- `Required`
- `RequiredWhen(Condition)`

`Condition` is intentionally small and non-executable:

- equality against a literal;
- presence;
- collection containment;
- numeric `<` / `<=` between settings;
- `All`, `Any`, and `Not` composition.

Conditions are validated against the definition before resolution begins.
Unknown paths, type-incompatible equality values, invalid containment targets,
and non-numeric numeric comparisons are definition errors.

The condition language is not a scripting language and performs no I/O.

### Defaults

`DefaultRule` supports:

- a fixed canonical value;
- a declarative lookup table keyed by another setting.

Lookup dependencies are topologically ordered. Unknown dependencies and cycles
are rejected during schema validation. Defaults are the lowest contributions;
ordinary inputs can replace or remove them using normal resolution semantics.

Facade customization rejects duplicate lookup-case keys and rejects configuring
the same derived-default target more than once. Encoding failures are reported
against the target configuration path rather than the lookup key.

### Removal permissions

Removal is not inferred from value shape. `RemovalPolicy` independently grants:

- whole-setting `unset`;
- collection `clear`;
- named-map entry removal;
- ordered-list matching-item removal.

Collection-specific mutation is opt-in. An unsupported operation returns
`ResolveError::RemovalNotAllowed`.

A removal targeting a nonexistent map entry or list item is a defined no-op and
is recorded as `ResolutionInformation::RemovalNoop` rather than treated as an
error. The information record carries only the structured removal kind, not the
target key or value.

### Representation and disclosure

These are independent axes:

`Representation`:

- `Ordinary`
- `ExternalReference`
- `SecretReference`

`Disclosure`:

- `Public`
- `Sensitive`

A secret reference is configuration; the secret payload itself is not. Secret
references and sensitive values are redacted when provenance/effective output
is exposed.

Reference representations currently require text-like scalar values. This
prevents a collection or arbitrary object from being mislabeled as a single
reference.

### Constraints and deprecation

Built-in constraints cover signed/unsigned/float ranges, non-empty values, and
string choice sets. Constraint/type incompatibilities and invalid defaults are
rejected when the definition is registered.

`Deprecation` is metadata on a setting. A present deprecated setting emits a
non-blocking structured warning and may name a replacement path.

## Components and applications

`ComponentDefinition` owns:

- a stable `DefinitionIdentity`;
- a component-local `Schema`;
- an optional declarative enablement condition.

`ApplicationDefinition` owns:

- an application identity;
- optional application-owned root settings;
- component compositions under non-overlapping paths;
- application-level validation rules.

Compilation prefixes component-local settings, conditions, rules, defaults,
and deprecation replacement paths into the application namespace. Component
roots cannot overlap each other or application-owned settings. Because these
definition structs are intentionally data-first and serializable, compilation
rechecks duplicate keys, path overlap, rule ownership, nested metadata, and
component identities even when callers construct the public structs directly
instead of using helper methods.

`EffectiveConfiguration::component_presence(key)` reports:

- `Absent` when the application does not compose the key;
- `Disabled` when composed but its enablement condition is false;
- `Enabled` otherwise.

Enablement does not automatically make every component field conditional.
Definitions explicitly use `RequiredWhen` for fields needed only while active;
this preserves the ability to declare unconditional component invariants.

## Validation order

Kernel validation runs against the complete candidate in this order:

1. value presence/null/type/representation/constraints;
2. section rules;
3. component rules;
4. application rules.

Relationship rules are not evaluated after a value-level error, preventing
cascading diagnostics from malformed values.

Blocking diagnostics result in `ResolutionReport::effective == None`.
Warnings preserve the effective configuration.

## Provenance

Provenance records:

- layer;
- selector specificity;
- logical input id;
- input target;
- physical/logical origin;
- action;
- redaction-safe contributed value;
- final item index for accumulated lists.

Less-specific same-layer candidates are retained as `Shadowed` provenance.
Append/prepend/remove operations maintain list-item provenance against final
indices. Deep/named-map combination records entry-level provenance. Sensitive
settings are the deliberate exception: provenance remains at the setting level
so dynamic map keys, nested paths, list indices, and collection shape are not
disclosed.

## Resolution information

Normal outcomes are not diagnostics. `ResolutionInformation` records events
such as:

- selector mismatch / ignored input;
- fixed or derived default use;
- higher-layer replacement;
- optional omission;
- removal no-op.

This keeps explainability separate from correctness problems.

## Operational verification

`VerificationReport` contains `VerificationCheck` / `VerificationResult` values
and remains separate from `ResolutionReport` and definition validation. It can
report whether this installation can actually use a valid configuration without
changing whether the definition itself is valid. The kernel deliberately does
not guess whether a failed operational check should block startup or refresh.

## Explicitly outside this kernel

The kernel deliberately does not implement:

- remote delivery or filesystem observation;
- revision history or rollback;
- plugin architectures;
- standardized schema packs;
- multi-tenant scope hierarchies;
- policy authority, caps, or locks;
- source acquisition and mutable runtime refresh.

The surrounding crate provides explicit source refresh and complete external
input reconciliation. Those APIs reuse the kernel and do not add transport or
control-plane semantics to it. Remote delivery and distributed coordination
remain outside this repository.
