# vs-cli

`vs-cli` provides the `vs` binary, command parsing, shell completion generation, and user-facing output.

## Responsibilities

- define the English-only CLI surface
- parse command arguments with `clap`
- delegate business logic to `vs-core`
- print human-readable command results
- expose hidden shell helper commands such as `__hook-env`

## Binary

The crate publishes a single binary named `vs`.

## Testing

Integration tests in `tests/cli.rs` cover:

- registry refresh
- plugin add/install/use/current/exec flows
- project scope behavior
- migration from a legacy home
- English help output
