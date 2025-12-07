# {{name}}

This project was scaffolded with `apexrc`. Common commands:

- `apexrc build` — verify sources and emit native ELF in `target/x86_64/debug/{{name}}`
- `apexrc run` — build and execute `fun apex()`
- `apexrc check` — fast syntax diagnostics
- `apexrc clean` — delete local `target/` artifacts
- `apexrc add forge.math` — add dependencies from the local `~/.apex` registry

Generate export metadata before publishing:

- Run `sdk/exports-gen --manifest Apex.toml --exports .afml/exports.toml --out .afml/exports.json`.
- Include `.afml/exports.json` inside published packages so consumers know which symbols are available.

All sources live under `src/`. The entry point is always `fun apex()` inside `src/main.afml`.
