# control_flow_if

This project exercises conditional branching in the native backend. Common commands:

- `apexrc build` — build a native ELF at `target/x86_64/debug/control_flow_if`
- `apexrc run` — build (native by default) and execute `fun apex()`
- `apexrc check` — fast syntax diagnostics
- `apexrc clean` — delete local `target/` artifacts
- `apexrc add forge.math` — add dependencies from the local `~/.apex` registry

All sources live under `src/`. The entry point is always `fun apex()` inside `src/main.afml`.
