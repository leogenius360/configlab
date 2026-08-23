# Basic application example

This is the smallest realistic consumer of `configlab`.

It demonstrates:

- typed defaults;
- one optional configuration file;
- environment-variable overrides;
- command-line overrides;
- an optional `SecretRef`;
- the descriptor API (`path`, `env`, `args`).
- equivalent TOML, JSON, and YAML inline documents in its executable tests.

Run from the repository root:

```bash
cargo run -p example-basic-app
```

No configuration file is required because `config.toml` is registered as
optional.
