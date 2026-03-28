# Workspace Layout

`vs` is organized as a multi-crate workspace so each subsystem has a focused boundary.

## Crates

- `vs-cli`: binary, argument parsing, completions, and output
- `vs-core`: orchestration and command use cases
- `vs-config`: config files, home detection, and version resolution
- `vs-registry`: registry index and local plugin entries
- `vs-installer`: transactional install staging and receipts
- `vs-plugin-api`: shared plugin models and traits
- `vs-plugin-lua`: Lua plugin backend
- `vs-plugin-wasi`: native plugin contract layer
- `vs-plugin-sdk`: author-facing helpers for native plugins
- `vs-shell`: activation scripts, links, and path helpers
- `vs-test-support`: fixtures and temp helpers

## Dependency flow

The intended dependency direction is:

1. leaf crates such as `vs-config`, `vs-registry`, `vs-installer`, `vs-shell`, and plugin crates
2. `vs-core` composes those leaves
3. `vs-cli` depends on `vs-core`

## Backend feature flags

The CLI and core crates expose matching backend features:

- `lua`
- `wasi`

Only backends compiled into the current build are accepted at runtime. Unsupported plugin types return an explicit error instead of failing later during plugin loading.

## File-size discipline

The workspace favors many small modules over a few oversized files:

- one command per file in `vs-cli`
- one use case per file in `vs-core/src/service`
- separate models, loaders, stores, and helpers in the leaf crates
