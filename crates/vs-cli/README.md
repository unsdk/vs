# vs-cli

`vs-cli` is the command dispatcher library behind the `vs` command: command parsing, shell completion generation, and user-facing output. The binary itself lives in the thin [`vs`](../vs) crate, which calls `vs_cli::run()`.

## Responsibilities

- define the CLI surface
- parse command arguments with `clap`
- delegate business logic to `vs-core`
- print human-readable command results
- expose hidden shell helper commands such as `__hook-env`
- expose `vs_cli::run()` as the shared entry point for the `vs` binary

## Features

- `lua`: enables Lua-backed plugins and the default vfox registry integration
- `wasi`: enables native WASI-style plugins

The `vs` binary crate forwards these feature flags. For the smallest
distributable binaries, build the `vs` crate with the workspace `min-size`
profile and only the backend features you need:

```bash
cargo build -p vs --profile min-size --no-default-features
cargo build -p vs --profile min-size --no-default-features --features wasi
cargo build -p vs --profile min-size --no-default-features --features lua
```

## Testing

Integration tests live in the `vs` crate (`crates/vs/tests/cli.rs`) and cover:

- registry refresh
- plugin add/install/use/current/exec flows
- project scope behavior
- migration from a legacy home
- English help output
