# ApexForge NightScript VS Code Extension

This extension delivers a first-class editing experience for ApexForge NightScript (`.afml`) projects powered by the `apexrc` compiler.

> Branding: The packaged `icon.png` is the official ApexForge logo copied verbatim from `assets/branding/apexforge_logo.png`. Never substitute another asset.

## Features

- **Syntax Highlighting** – Full TextMate grammar covering language keywords, async primitives, UI DSL calls (`ctx.text`, `ctx.button`, etc.), forge modules, and Flutter/engine APIs (SceneBuilder, FlutterEmbedder, PlatformChannel, ...).
- **Language Configuration** – Comment toggles, bracket pairs, auto-closing quotes/braces, and indentation rules tuned to `fun`, `struct`, `impl`, `widget`, and SceneBuilder blocks.
- **Snippets** – Quickly scaffold `fun`, `async fun`, `struct`, `impl`, UI layout blocks, async chaining (`async.then/.catch/.finally`), and SceneBuilder pipelines.
- **Completions** – Inline IntelliSense suggestions for the full AFNS keyword set, async runtime helpers, forge modules, and engine identifiers referenced in `README.md`, `ROADMAP.md`, `FLUTTER_INTEGRATION_ROADMAP.md`, and `async_Readme.md`.
- **Icon** – Custom `.afml` document icon for quick recognition in the explorer.

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
