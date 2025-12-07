# fs_basic

Filesystem demo for ApexForge NightScript.

Commands:

- `apexrc build` — emit native ELF into `target/x86_64/debug/fs_basic`
- `apexrc run` — build + run `fun apex()`
- `apexrc build --target x86` — emit a 32-bit ELF (experimental)
- `apexrc clean` — remove local `target/`

Key APIs:

- `forge.fs.read_to_string`, `write_string`, `append_string`
- `forge.fs.metadata`, `read_dir`, `exists`, `is_file`, `is_dir`
- `forge.fs.ensure_dir`, `copy_file`, `copy_dir_recursive`, `move`, `remove_file`
