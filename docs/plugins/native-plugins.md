# Native Plugins

The current native plugin layer is provided by `vs-plugin-wasi`.

## Current implementation model

This build uses a typed descriptor runtime backed by `component.toml` plus a versioned WIT contract in:

`crates/vs-plugin-wasi/wit/vs-plugin.wit`

## Descriptor layout

```toml
[plugin]
name = "deno"
description = "Fixture native plugin"
aliases = ["denojs"]
legacy_filenames = [".deno-version"]

[[versions]]
version = "1.40.5"
source = "packages/1.40.5"
note = "Current fixture release"

[[env]]
key = "DENO_HOME"
value = "{install_dir}"
```

## What the runtime reads

- static plugin metadata
- available versions
- install source paths
- environment key templates

## Design direction

The WIT file keeps the native contract explicit so the descriptor runtime can evolve toward a fuller component-hosting model without changing the rest of the workspace API.
