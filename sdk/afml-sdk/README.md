# ApexForge AFML SDK

The AFML SDK provides helpers to declare exported symbols from `.afml` libraries. Place an `.afml/exports.toml` beside your `Apex.toml` describing each export. Run the helper script (or `sdk/exports-gen`) before packaging so `.afml/exports.json` is generated and can be consumed by the runtime/publish pipeline.

Example `.afml/exports.toml`:

```toml
[exports]
name = "hello"
signature = "fn hello(str) -> str"

[exports]
type = "HelloResult"
[exports.fields]
- name = "message"
  ty = "str"
```
