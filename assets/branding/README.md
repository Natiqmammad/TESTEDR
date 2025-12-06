# ApexForge Branding

- `apexforge_logo.png` — canonical and immutable ApexForge logo. Copy (never edit) this file wherever branding is required.
- `logo_config.toml` — metadata consumed by build scripts, documentation, and tooling to locate the official asset. Treat this file as the single source of truth for logo location and checksum.

Usage rules:
- The compiler (`apexrc`) prints the canonical path on startup.
- VS Code extension packaging copies this PNG verbatim as its icon.
- All docs should embed the logo via `![ApexForge Official Logo](assets/branding/apexforge_logo.png)`.

Any alternative or modified logos are strictly forbidden.
