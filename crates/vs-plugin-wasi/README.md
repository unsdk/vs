# vs-plugin-wasi

`vs-plugin-wasi` provides the native plugin contract layer for `vs`.

## Responsibilities

- define the WIT contract used by native plugins
- load typed native plugin descriptors from `component.toml`
- expose a backend that implements the shared plugin runtime contract

## Current scope

The current build uses a descriptor-based native runtime so the rest of the workspace can treat native plugins uniformly while the WIT contract stays versioned in `wit/`.
