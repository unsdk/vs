# vs

`vs` is a Rust workspace for a cross-platform runtime version manager inspired by `vfox`.

## Install from GitHub Releases

### macOS / Linux

Install the latest published `full` build to `~/.local/bin`:

```bash
curl -fsSL https://raw.githubusercontent.com/unsdk/vs/main/scripts/install.sh | bash
```

Install a specific release or variant:

```bash
curl -fsSL https://raw.githubusercontent.com/unsdk/vs/main/scripts/install.sh | bash -s -- --version v0.1.0 --variant lua
```

### Windows PowerShell

Install the latest published `full` build to `%LOCALAPPDATA%\Programs\vs\bin` and add it to the user `PATH`:

```powershell
irm https://raw.githubusercontent.com/unsdk/vs/main/scripts/install.ps1 | iex
```

Install a specific release or variant:

```powershell
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/unsdk/vs/main/scripts/install.ps1))) -Version v0.1.0 -Variant lua
```

Useful options:

- `--install-dir /path/to/bin`
- `--target <target-triple>`
- `--repo <owner/name>`

PowerShell equivalents:

- `-InstallDir <path>`
- `-Target <target-triple>`
- `-Repository <owner/name>`
- `-SkipPathUpdate`

Equivalent environment variables are also supported:

- `VS_INSTALL_DIR`
- `VS_INSTALL_VERSION`
- `VS_INSTALL_VARIANT`
- `VS_INSTALL_TARGET`
- `VS_RELEASE_REPOSITORY`

## Status

This repository provides the initial multi-crate implementation with:

- an CLI named `vs`
- config resolution for `.vs.toml`, `vs.toml`, `.vfox.toml`, `vfox.toml`, and common legacy files
- a local plugin registry model
- transactional local installs
- scope-aware `use`, `current`, `list`, `exec`, `activate`, and `migrate` flows
- a Lua-backed fixture plugin runtime and a typed native plugin contract crate

The design favors small modules, explicit crate boundaries, and testable services.

## Feature flags

`vs-cli` supports backend feature gating:

- `lua`
- `wasi`

Examples:

```bash
cargo build -p vs-cli --no-default-features --features lua
cargo build -p vs-cli --no-default-features --features wasi
cargo build -p vs-cli --no-default-features --features full
```

For the smallest binaries, use the dedicated size-first profile:

```bash
cargo build -p vs-cli --profile min-size --no-default-features
cargo build -p vs-cli --profile min-size --no-default-features --features wasi
cargo build -p vs-cli --profile min-size --no-default-features --features lua
```

The standard `release` profile now strips symbols and enables LTO. The `min-size`
profile additionally switches to `opt-level = "z"` and `panic = "abort"` for
size-focused distribution builds.

When the `lua` feature is enabled and `registry.address` is unset, `vs` defaults to the official vfox plugin registry at `https://version-fox.github.io/vfox-plugins`.

## Workspace crates

- `vs-cli`: CLI entrypoint and command parsing
- `vs-core`: application orchestration and use-case services
- `vs-config`: home, config, and version resolution
- `vs-registry`: registry persistence and plugin lookup
- `vs-installer`: transactional local installs and uninstall
- `vs-plugin-api`: shared plugin models and errors
- `vs-plugin-lua`: Lua-compatible plugin loader
- `vs-plugin-wasi`: native plugin contract and descriptor runtime
- `vs-plugin-sdk`: helper types for native plugin authors
- `vs-shell`: activation scripts and shell path planning
- `vs-test-support`: shared fixtures and temporary workspace helpers

## Quality gates

```bash
cargo fmt --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test
cargo test --doc
```
