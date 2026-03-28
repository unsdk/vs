# Plugin Overview

`vs` supports two plugin tracks.

## Lua plugins

Lua plugins are useful when you want a small, data-driven plugin with scriptable metadata and hook files.

See:

- [Lua Plugins](./lua-plugins.md)

## Native plugins

Native plugins use the `vs-plugin-wasi` contract layer and are intended for typed, non-Lua plugin integrations.

See:

- [Native Plugins](./native-plugins.md)

## Shared concepts

Both plugin tracks expose the same conceptual capabilities:

- list available versions
- describe how to install a version
- emit environment keys for an installed runtime
- parse legacy version file contents
