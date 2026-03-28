# Repository Guidelines

## Project Structure & Module Organization
`vs` is a Rust 2024 workspace (`Cargo.toml`) with crates under `crates/`. Keep boundaries tight: `vs-cli` owns argument parsing, commands, TUI/output, and the `vs` binary; `vs-core` owns orchestration and use cases; leaf crates such as `vs-config`, `vs-registry`, `vs-installer`, and `vs-shell` provide focused services. Follow the existing file discipline: one CLI command per file in `crates/vs-cli/src/command/`, one service per file in `crates/vs-core/src/service/`. Shared test helpers live in `crates/vs-test-support`, fixtures in `fixtures/`, and longer-form docs in `docs/`.

## Build, Test, and Development Commands
- `cargo build -p vs-cli`: build the default CLI at `target/debug/vs`.
- `cargo run -p vs-cli -- --help`: run the CLI locally.
- `cargo build -p vs-cli --no-default-features --features lua`: test a backend-specific build. Swap `lua` for `wasi` or use `"lua,wasi"`.
- `cargo fmt --all --check`: enforce formatting.
- `cargo clippy --all-targets --all-features --locked -- -D warnings`: run the same lint gate as CI.
- `cargo test --workspace`: run unit and integration tests.
- `cargo test --doc --workspace`: run doctests.

## Coding Style & Naming Conventions
Use standard Rust formatting with 4-space indentation and keep modules small. The workspace denies broad Clippy issues plus `unwrap_used`, `expect_used`, `redundant_clone`, and `needless_collect`, so prefer explicit error propagation with `Result`. Use `snake_case` for modules, functions, and files; `UpperCamelCase` for types; and descriptive command/service filenames such as `install.rs`, `available.rs`, or `use_tool.rs`.

## Testing Guidelines
Prefer focused unit tests close to the code with `#[cfg(test)]`, and add integration coverage in `crates/vs-cli/tests/` for end-to-end CLI flows. Reuse `vs-test-support` and `fixtures/` instead of creating ad hoc test data. Match the existing naming style: `resolve_home_should_prefer_vs_home`, `cli_should_migrate_from_a_legacy_home`.

## Commit & Pull Request Guidelines
Recent history follows Conventional Commits: `feat:`, `fix:`, `refactor:`, `test:`, and automated `release:` commits. Keep subjects imperative and scoped to one change. PRs should explain the behavior change, note any feature-flag impact, list validation commands run, and update `README.md` or `docs/` when CLI behavior, configuration, or plugin flows change. Include terminal output or screenshots when prompts or user-visible CLI formatting change.
