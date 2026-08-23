# Platform integration example

A source-neutral integration for platform and library authors. This package
disables ConfigLab's default features and uses no derive macro or document
format parser.

It demonstrates:

- manually declared, serializable setting schemas;
- reusable server, database, and notification component definitions;
- application composition and cross-component validation;
- fixed and lookup-derived defaults;
- exact contextual selectors;
- component-targeted logical inputs from external adapters;
- append and combine-by-key merge behavior;
- explicit map-entry removal;
- disabled/enabled component state;
- aggregated diagnostics for an invalid conditional requirement;
- deprecation warnings;
- secret-reference redaction and provenance;
- operational verification kept separate from definition validity.

Run from the repository root:

```bash
cargo run -p example-platform-integration
```

This is the model a remote provider, framework adapter, deployment controller,
or non-file configuration integration should use: normalize external data into
`LogicalInput` values and let the one deterministic resolver own semantics.
