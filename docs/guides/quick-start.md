# Quick Start

This guide walks through the shortest path from an empty home to a working `vs` installation.

## 1. Build the CLI

```bash
cargo build -p vs-cli
```

The binary is produced as `target/debug/vs` during development.

Backend-specific builds are also supported:

```bash
cargo build -p vs-cli --no-default-features --features lua
cargo build -p vs-cli --no-default-features --features wasi
cargo build -p vs-cli --no-default-features --features "lua,wasi"
```

For distribution builds where binary size matters more than build time, prefer:

```bash
cargo build -p vs-cli --profile min-size --no-default-features
cargo build -p vs-cli --profile min-size --no-default-features --features wasi
cargo build -p vs-cli --profile min-size --no-default-features --features lua
```

## 2. Choose a home directory

`vs` uses `~/.vs` by default. Its internal layout is kept close to `vfox`, but the default home path remains `vs`-native.

To override it:

```bash
export VS_HOME="$HOME/.vs"
```

## 3. Point `vs` at a registry index

This repository ships with a local fixture index:

```bash
vs config registry.address /absolute/path/to/fixtures/registry/index.json
vs update
```

In the current build, `registry.address` accepts either a registry base address or a direct local index JSON path.

When `vs` is built with the `lua` feature and `registry.address` is unset, `vs update` defaults to the official vfox plugin registry:

`https://version-fox.github.io/vfox-plugins`

If the local registry cache is still empty, commands such as `vs available`, `vs search`, and `vs add <name>` automatically bootstrap the registry index on first use.

## 4. Add a plugin

```bash
vs add nodejs
```

You can also add a plugin directly from a source directory:

```bash
vs add nodejs --source /absolute/path/to/plugin --backend lua
```

## 5. Install and activate a version

```bash
vs install nodejs@20.11.1
vs use nodejs@20.11.1 -g
```

Check the active version:

```bash
vs current nodejs
```

## 6. Run a command with the resolved environment

```bash
vs exec node
```

## 7. Enable shell activation

Example for Bash:

```bash
eval "$(vs activate bash)"
```

After activation, shell hooks can apply the currently resolved tool versions to the session automatically.
