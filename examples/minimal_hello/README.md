# minimal_hello

This project was scaffolded with `apexrc`. Common commands:

- `apexrc build` — verify sources and emit native ELF at `target/x86_64/debug/minimal_hello`
- `apexrc run` — build and execute `fun apex()`
- `apexrc check` — fast syntax diagnostics
- `apexrc clean` — delete local `target/` artifacts
- `apexrc add forge.math` — add dependencies from the local `~/.apex` registry

All sources live under `src/`. The entry point is always `fun apex()` inside `src/main.afml`.
