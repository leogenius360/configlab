# Runtime service example

A self-contained long-running service simulation covering ConfigLab's
refreshable configuration model. It uses a temporary optional file, so running
the example never modifies the repository.

It demonstrates:

- initial construction through `ConfigBuilder::live`;
- rereading every registered physical source through `LiveConfig::refresh`;
- immediate replacement after complete resolution and validation;
- parse and typed-validation failures preserving the current value;
- redaction-aware value and provenance changes;
- current per-setting provenance through `LoadedConfig::explain`;
- complete optional-source removal restoring lower-layer values.

Run from the repository root:

```bash
cargo run -p example-runtime-service
```

The example drives refresh explicitly. Production hosts can call the same method
from their own timer, event loop, filesystem notification integration, admin
endpoint, or test harness. ConfigLab owns no watcher thread or async runtime.
