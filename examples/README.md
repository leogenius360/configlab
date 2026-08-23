# ConfigLab executable examples

Every example is a standalone workspace package that depends only on
ConfigLab's public API. Together they cover every supported product workflow;
tests are part of the examples so advertised behavior remains executable.

## Learning path

1. **basic-app** — the smallest startup-only application;
2. **cli-tool** — layered project, environment, and invocation configuration;
3. **contextual-service** — a nested production service with selectors,
   validation, secrets, and explainability;
4. **runtime-service** — complete source refresh, reconciliation, validation,
   change-reporting, and recovery lifecycle;
5. **platform-integration** — source-neutral schemas and reusable components
   without derive macros or format features.

## Supported workflow matrix

| Workflow | Concrete example |
| --- | --- |
| Typed defaults and one optional local file | `basic-app` |
| TOML, JSON, and YAML through one schema | `basic-app` tests |
| Environment and strict command-line overrides | `basic-app`, `cli-tool` |
| Explicit unknown-argument pass-through for a shared parser | `cli-tool` tests |
| Project-root discovery from a nested working directory | `cli-tool` tests |
| Custom layer order and source-independent precedence | `cli-tool` |
| Boolean CLI flags and application validation | `cli-tool` |
| Appendable collections and secret references | `cli-tool` |
| Nested service configuration and optional sections | `contextual-service` |
| Context selectors and same-layer specificity | `contextual-service` |
| Conditional requirements and cross-section validation | `contextual-service` |
| Redacted effective output, provenance, diagnostics, and normal information | `contextual-service` |
| Reusable plans and reviewed resource limits | `runtime-service` |
| Re-reading configured physical sources on demand | `runtime-service` |
| Complete external input replacement and removal | `runtime-service` |
| Runtime validation rejection with unchanged current values | `runtime-service` |
| Redaction-aware runtime changes and provenance reporting | `runtime-service` |
| Failed-refresh recovery and provenance-only changes | `runtime-service` |
| Hand-written serializable schemas without derive | `platform-integration` |
| Reusable components and application composition | `platform-integration` |
| Raw logical-input provider adapters and component targets | `platform-integration` |
| Fixed and lookup-derived defaults | `platform-integration` |
| Append, combine-by-key, and explicit removal semantics | `platform-integration` |
| Aggregated conditional/deprecation diagnostics | `platform-integration` |
| Operational verification separate from configuration validity | `platform-integration` |
| Minimal build with ConfigLab default features disabled | `platform-integration` |

## Run everything

From the repository root:

```bash
cargo run -p example-basic-app
cargo run -p example-cli-tool -- --jobs 8 --no-minify
cargo run -p example-contextual-service -- --port 9090
cargo run -p example-runtime-service
cargo run -p example-platform-integration
cargo test --workspace --all-features --all-targets
```

The repository release gate compiles, tests, and executes all five examples.
The runtime example uses a temporary copy of its fixture and cleans it up.

## Deliberate non-examples

ConfigLab does not currently provide a remote control plane, secret manager,
distributed rollout coordinator, durable revision database, background watcher
thread, automatic component side effects, or remote transport. The
examples do not simulate those as if they were supported product behavior.
