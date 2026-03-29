# vs-cli

`vs-cli` provides the `vs` binary, command parsing, shell completion generation, and user-facing output.

## Responsibilities

- define the CLI surface
- parse command arguments with `clap`
- delegate business logic to `vs-core`
- print human-readable command results
- expose hidden shell helper commands such as `__hook-env`

## Binary

The crate publishes a single binary named `vs`.

## Features

- `lua`: enables Lua-backed plugins and the default vfox registry integration
- `wasi`: enables native WASI-style plugins

For the smallest distributable binaries, build with the workspace `min-size`
profile and only the backend features you need:

```bash
cargo build -p vs-cli --profile min-size --no-default-features
cargo build -p vs-cli --profile min-size --no-default-features --features wasi
cargo build -p vs-cli --profile min-size --no-default-features --features lua
```

## Testing

Integration tests in `tests/cli.rs` cover:

- registry refresh
- plugin add/install/use/current/exec flows
- project scope behavior
- migration from a legacy home
- English help output
