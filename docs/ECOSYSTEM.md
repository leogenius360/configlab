# Ecosystem direction

ConfigLab is intentionally named as a toolkit rather than only as a resolver.
The current package establishes the deterministic configuration model on which
broader tooling can be built.

## Current scope

The first public alpha provides:

- typed derive-first definitions;
- source normalization;
- deterministic resolution;
- validation and component composition;
- provenance and diagnostics;
- redaction helpers;
- file/environment/argument/inline source descriptors.
- reusable resolution plans and explicit in-process source refresh;
- complete source-neutral external reconciliation.

## Candidate future areas

The project may grow into reusable configuration components such as:

```rust,ignore
use configlab::types::{DatabaseConfig, LoggingConfig, ServerConfig};
```

Potential component families include server/listener, database pools, logging,
TLS, retries, metrics, health checks, and common runtime limits. Such components
must remain framework-neutral where practical and should be modules/features
before becoming separate crates unless independent publication is justified.

Other candidate areas include schema/introspection, configuration testing,
command-line inspection tools, migration helpers, and framework integrations.

## Explicit non-promises

This roadmap is directional, not a compatibility promise. The following remain
out of the current package contract:

- remote configuration transports and servers;
- background filesystem notification services;
- async resolution;
- plugin systems;
- automatic secret-store fetching;
- multi-tenant policy/control planes.

New capabilities should be justified by real adoption feedback and must not
weaken the deterministic local resolution model.
