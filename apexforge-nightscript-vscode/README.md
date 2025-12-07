# ApexForge NightScript VS Code Extension

**Author:** Natiq Mammadov — ApexForge · https://github.com/Natiqmammad

This extension delivers a first-class editing experience for ApexForge NightScript (`.afml`) projects powered by the `apexrc` compiler.

> Branding: The packaged `icon.png` is the official ApexForge logo copied verbatim from `assets/branding/apexforge_logo.png`. Never substitute another asset.

## Features

- **Syntax Highlighting** – Full TextMate grammar covering language keywords, async primitives, UI DSL calls (`ctx.text`, `ctx.button`, etc.), forge modules, and Flutter/engine APIs (SceneBuilder, FlutterEmbedder, PlatformChannel, ...).
- **Language Configuration** – Comment toggles, bracket pairs, auto-closing quotes/braces, and indentation rules tuned to `fun`, `struct`, `impl`, `widget`, and SceneBuilder blocks.
- **Snippets** – Quickly scaffold `fun`, `async fun`, `struct`, `impl`, UI layout blocks, async chaining (`async.then/.catch/.finally`), and SceneBuilder pipelines.
- **Completions** – Inline IntelliSense suggestions for the full AFNS keyword set, async runtime helpers, forge modules, and engine identifiers referenced in `README.md`, `ROADMAP.md`, `FLUTTER_INTEGRATION_ROADMAP.md`, and `async_Readme.md`.
- **Icon** – Custom `.afml` document icon for quick recognition in the explorer.
- **`apexrc check` Diagnostics** – The extension runs `apexrc check` automatically on file open/save (or via the **“ApexForge: Run apexrc check”** command) and surfaces parser/lexer errors inline through VS Code’s Problems panel. Diagnostics appear without having to build/run the project, similar to Rust Analyzer’s `cargo check`.
- **Forge-aware snippets** – Templates and completions cover `forge.fs`, `forge.net`, `forge.db`, `forge.async`, `forge.log`, Flutter `ctx.*` widgets, and the generic helper patterns that ship in `examples/`.

### Keyword coverage

Completions include every forge primitive listed in the main README:

- `forge.fs`: `read_to_string`, `read_bytes`, `write_string`, `write_bytes`, `append_*`, `create_dir(_all)`, `remove_dir(_all)`, `copy_file`, `move`, `metadata`, `exists`, `is_file`, `is_dir`, `read_dir`, `ensure_dir`, `read_lines`, `write_lines`, `copy_dir_recursive`, `join`, `dirname`, `basename`, `extension`, `canonicalize`.
- `forge.net`: `tcp_connect`, `tcp_listen`, `tcp_accept`, `tcp_send`, `tcp_recv`, `tcp_shutdown`, `tcp_set_nodelay`, timeout setters, `tcp_peer_addr`, `tcp_local_addr`, plus the full UDP family (`udp_bind`, `udp_send_to`, `udp_recv_from`, `udp_set_broadcast`, etc.) and cleanup helpers (`close_socket`, `close_listener`).
- `forge.db`: `db.open`, `db.exec`, `db.query`, `db.begin`, `db.commit`, `db.rollback`, `db.get`, `db.set`, `db.del`, `db.close`.
- `forge.async`/UI: all `async.*` helpers, `forge.log.*`, and `ctx.*` widget builders for Flutter-like layouts.

### apexrc Integration

- Uses the workspace folder as the execution root and streams all output into the dedicated “ApexForge apexrc” output channel.
- Errors are parsed into VS Code diagnostics so caret/highlight positions match the CLI output.
- The path to the CLI and its arguments can be customized with the following settings:
  - `apexforge.apexrcPath` (default `apexrc`)
  - `apexforge.apexrcCheckArgs` (default `["check"]`)

If the check fails, you get both an inline error and the full CLI text to help triage issues before ever running `apexrc build`.

### FFI exports awareness

The extension also documents the `.afml/exports.json` workflow. `apexrc install` indexes the exports metadata into `target/vendor/.index.json`, and native libraries are placed under `.afml/lib/<triplet>/`. The runtime now loads Rust exports with `libloading` (with stub fallbacks when symbols are absent), and `apexrc doctor` surfaces any native loading problems before you run your app. The Diagnostics view keeps you aware of missing exports without running `apexrc run`.

## Getting Started

1. Copy the `apexforge-nightscript-vscode` directory into your workspace.
2. Run `vsce package` (or `npm install && npm run compile` if you prefer TypeScript tooling).
3. Install the generated `.vsix` in VS Code (`Extensions → ... → Install from VSIX`).
4. Open any `.afml` file or `apexrc` project – the extension activates automatically.

## Requirements

- VS Code `>= 1.85.0`
- `apexrc` compiler (for running/building your projects)

## Known Limitations

- Completions are keyword-oriented (no full AST awareness yet).
- Formatting is delegated to future `apexrc` formatter support; hook your formatter via `afns.afml-formatter` if available.

## Contributing

1. Fork this repository.
2. Update the grammar/snippets/completion keywords as the AFNS spec evolves.
3. Open a PR with before/after screenshots covering new syntax.

## License

MIT

---

**Created by Natiq Mammadov — ApexForge**  
GitHub: https://github.com/Natiqmammad
