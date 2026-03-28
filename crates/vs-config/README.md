# vs-config

`vs-config` resolves `vs` home paths, reads user configuration, and merges tool version sources.

## Responsibilities

- resolve `VS_HOME` and legacy migration candidates
- read and write `config.yaml`
- parse `.vs.toml`, `vs.toml`, `.vfox.toml`, and `vfox.toml`
- parse common legacy files such as `.tool-versions` and `.nvmrc`
- resolve active tool versions using `Project > Session > Global > System`

## Key types

- `HomeLayout`
- `AppConfig`
- `ToolVersions`
- `ResolvedToolVersion`
- `Scope`
