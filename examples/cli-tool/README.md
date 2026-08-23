# CLI tool example

A local build-tool configuration modeled as a real downstream workspace
package. It depends only on the public `configlab` API.

It demonstrates:

- explicit custom layer order;
- optional project configuration;
- generated environment and CLI bindings;
- strict argument handling with explicit shared-parser pass-through;
- ancestor-based project configuration discovery;
- typed defaults and validation;
- an appendable collection with explicit removal policy;
- an optional secret reference.

Run from the repository root:

```bash
cargo run -p example-cli-tool -- --jobs 8 --no-minify
```
