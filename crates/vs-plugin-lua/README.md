# vs-plugin-lua

`vs-plugin-lua` loads Lua-backed plugins for `vs`.

## Responsibilities

- read `metadata.lua`
- read Lua hook files under `hooks/`
- deserialize hook outputs into typed Rust structures
- expose a backend that implements the shared plugin runtime contract

## Fixture hook layout

This repository uses a fixture-friendly Lua layout:

- `metadata.lua`
- `hooks/available.lua`
- `hooks/pre_install.lua`
- `hooks/env_keys.lua`

The crate intentionally keeps the runtime small and testable.
