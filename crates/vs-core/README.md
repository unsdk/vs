# vs-core

`vs-core` is the application orchestration layer for `vs`.

## Responsibilities

- compose config, registry, installer, shell, and plugin services
- implement command use cases such as `install`, `use`, `current`, `exec`, and `migrate`
- keep command logic out of the CLI crate
- provide a stable application API for future frontends

## Design

Each use case lives in its own module under `src/service/`. The `App` type owns shared dependencies and helper methods, while the service modules add focused `impl App` methods for specific commands.

## Features

- `lua`: compiles the Lua plugin backend and enables the default vfox registry source
- `wasi`: compiles the native plugin backend
