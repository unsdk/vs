# vs

`vs` is the binary crate for the cross-platform runtime version manager inspired by `vfox`. It is a thin entry point that delegates to the [`vs-cli`](../vs-cli) library, which owns argument parsing, commands, TUI/output, and shell completion.

## Install

```bash
cargo install vs
```

## Build

```bash
cargo build -p vs
cargo build -p vs --no-default-features --features lua
cargo build -p vs --no-default-features --features wasi
cargo build -p vs --no-default-features --features full
```

Size-optimized builds:

```bash
cargo build -p vs --profile min-size --no-default-features
cargo build -p vs --profile min-size --no-default-features --features wasi
cargo build -p vs --profile min-size --no-default-features --features lua
```
