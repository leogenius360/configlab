# Contextual service example

A composed service configuration using the full local/static model through the
public facade.

It demonstrates:

- server, database, logging, metrics, TLS, and alerting sections;
- production and regional selectors;
- specificity within the `contextual` layer;
- optional local/contextual files;
- environment and invocation overrides;
- conditional requirements;
- secret and sensitive-value redaction;
- section and application validation;
- safe effective-configuration output;
- setting provenance, normal resolution information, and diagnostics;
- complete external-input reconciliation with immediate typed-value and
  provenance updates.

Run from the repository root:

```bash
cargo run -p example-contextual-service -- --port 9090
```
