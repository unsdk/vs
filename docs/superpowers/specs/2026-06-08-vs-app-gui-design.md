# vs-app: GPUI desktop GUI for the vs version manager

Date: 2026-06-08
Status: Approved design (pending implementation plan)

## Summary

Add a new `vs-app` crate to the workspace: a cross-platform desktop GUI for the
`vs` runtime version manager, built with [`gpui`](https://crates.io/crates/gpui)
and [`gpui-component`](https://crates.io/crates/gpui-component). It drives the
existing `vs-core::App` API — the same orchestration layer the `vs` CLI uses — so
the GUI adds no business logic, only presentation and interaction.

The app provides **version management** (search / install / switch / uninstall
versions of a tool) and **plugin management** (add / remove / update plugins,
browse and refresh the registry). Configuration editing, home migration, and
self-upgrade are explicitly deferred.

## Goals

- A native desktop window that covers the daily `vs` workflow without the CLI.
- Reuse `vs-core::App` verbatim; no duplicated logic.
- Keep the codebase compliant with the workspace's strict lint gate.
- Cross-platform: macOS, Linux, Windows.

## Non-goals (deferred)

- Config key editing (`config`/`set_config_value`/`unset_config_value`).
- Home migration (`migrate`).
- Self-upgrade (`upgrade_self_to`/`self_upgrade_summary`).
- Shell activation / `hook-env` / `cd` helpers (these are shell-integration
  concerns, not GUI ones).
- Shipping the GUI as a prebuilt GitHub release artifact (v1 distributes via
  `cargo install vs-app` only).

## Decisions

| Decision | Choice | Rationale |
| --- | --- | --- |
| Package / binary name | `vs-app` (kebab) / binary `vs-app` | Matches `vs-cli`/`vs-core` convention; `vs` binary is the CLI and stays. |
| Workspace integration | Full member: strict workspace lints, published to crates.io, runs in CI | User decision; maximum consistency with the rest of the workspace. |
| Platforms | macOS + Linux + Windows | User decision; gpui's default features already include all three backends. |
| GUI framework | `gpui = "0.2"`, `gpui-component = "0.5"` | Both now published on crates.io, preserving the workspace's crates.io-only dependency model. |
| Threading | Run blocking `vs-core` calls on gpui's background executor | `vs-core::App` is synchronous and performs network/disk I/O; the UI thread must never block. |

## Scope: features → vs-core mapping

All operations call existing `vs_core::App` methods (synchronous,
`-> Result<_, CoreError>`).

### Version management (per selected tool)
- List installed: `installed_versions_for_plugin(name)` + `current_tool(name)`
  (to mark the active version).
- Switch version: `use_tool(name, version, scope, unlink=false)` where `scope`
  is the title-bar selection.
- Uninstall: `uninstall_plugin_version(name, version)` (surfaces
  `UninstallResult.auto_switched` in the toast).
- Search available: `search_versions(name, args)`.
- Install: `install_plugin_version(name, Some(version))`.

### Plugin management
- List added tools (sidebar): `added_plugins()` + `current_tool_statuses()`.
- Browse registry (Add modal → "From registry"): `available_plugins()`.
- Add from registry: `add_plugin(Some(name), None, None, None)`.
- Add from source (Add modal → "From source"): `add_plugin(None, Some(source),
  Some(backend), alias)`.
- Update one / all: `update_plugin(name)` / `update_all_plugins()`.
- Remove: `remove_plugin(name)`.
- Refresh registry (title bar): `update_registry()`.

### Scope selector
Title-bar control mapping to `UseScope::{Project, Global, Session}`, applied by
`use_tool`.

## UI layout (approved)

Two-pane window inside a `gpui-component` `Root` + `TitleBar`:

- **Title bar:** registry summary + Refresh action; global scope selector
  (Project / Global / Session).
- **Left sidebar:** filter input, **＋ Add** button (opens Add-tool modal), and
  the list of added tools each showing its current-version badge.
- **Right detail pane** (selected tool):
  - Header: tool name, homepage, current version; **Update plugin** /
    **Remove plugin** actions.
  - **Installed** section: each version row with *Use ▾* (scope-aware) and
    *Uninstall*; current version marked.
  - **Available** section: version search box → results, each with *Install*.
- **Add-tool modal:** two tabs — *From registry* (search + Add) and *From
  source* (git URL / local path, optional alias, backend selector, Add).
- **Bottom status / toast:** success and error notifications.

**Empty state:** when `added_plugins()` is empty, the sidebar shows a prompt to
add a tool via **＋ Add**.

## Architecture

Two layers, separating testable logic from gpui views. This is deliberate: it
keeps the bulk of the code unit-testable headlessly (important for CI) and
confines panic-prone UI code to a thin shell.

```
crates/vs-app/
  Cargo.toml
  src/
    main.rs        # gpui Application bootstrap: init theme, open window
    service.rs     # AppService { core: Arc<vs_core::App> } — typed wrappers, no gpui
    model.rs       # UI-agnostic view-model structs + pure transforms (testable)
    ui/
      mod.rs
      root.rs      # window root: titlebar, scope selector, toast host, layout
      sidebar.rs   # tool list + filter + Add button
      detail.rs    # installed + available sections, version actions
      add_tool.rs  # Add-tool modal (registry tab + source tab)
```

- **`service.rs`** — wraps `Arc<vs_core::App>`; each method returns plain data
  (`Vec<ToolRow>`, `Vec<VersionRow>`, etc.) or `Result`. No gpui types, no
  `unwrap`/`expect`. This is the seam the UI calls through.
- **`model.rs`** — display structs (`ToolRow`, `VersionRow`, `ScopeChoice`,
  `AddSource`) and pure functions that merge core results into rows (e.g. fold
  installed + current + available into a sorted version list with flags). Unit
  tests live here.
- **`ui/`** — gpui-component views. Each view owns a gpui `Entity` for its state,
  renders from `model` structs, and dispatches actions to `service` on the
  background executor.
- **`main.rs`** — creates the gpui `Application`, initializes the gpui-component
  theme/assets, builds `AppService` from `vs_core::App::from_env()`, opens the
  main window.

## Threading & data flow

`vs-core::App` calls block (network installs, registry refresh, version search).
The rule: **the main/UI thread never blocks.**

1. **Load** (startup or refresh): spawn `service` read calls on
   `cx.background_executor()`; on completion, hop back to the foreground via
   `cx.spawn` and `entity.update(cx, ...)` to populate rows.
2. **Mutating actions** (install / use / uninstall / add / remove / update /
   registry-refresh): set a per-row or per-action **busy** flag, spawn the
   blocking call on the background executor, then on completion update the
   affected lists and emit a toast (success or error). Errors are never
   panics — `CoreError` is formatted into the notification.
3. Concurrency: actions are independent per tool/version; the busy flag prevents
   duplicate concurrent triggers on the same target.

## Error handling

- `service` propagates `CoreError` with `?`.
- The `ui` layer converts every `Result::Err` into a gpui-component
  notification/toast; no operation panics the app.
- No `unwrap`/`expect` anywhere (enforced by the workspace lint gate).

## Dependencies

`crates/vs-app/Cargo.toml`:

- `vs-core = { workspace = true, default-features = false }` with `vs-app`
  features `default = ["full"]`, `full = ["lua", "wasi"]`, `lua = ["vs-core/lua"]`,
  `wasi = ["vs-core/wasi"]` (mirrors the `vs` crate so registry/plugin backends
  are available).
- `gpui = "0.2"` (default features — includes macOS/Linux/Windows backends).
- `gpui-component = "0.5"` with **minimal features** (no `webview`,
  `tree-sitter-languages`, `decimal`, or `inspector`) to keep the dependency
  tree and `--all-features` CI builds tractable.
- `anyhow` (workspace) for `main` ergonomics.
- `[lints] workspace = true`.

Root `Cargo.toml`: add `vs-app = { version = "0.0.2", path = "crates/vs-app",
default-features = false }` to `[workspace.dependencies]` for consistency.

## CI & release changes

- **`.github/workflows/ci.yml`** already matrices `ubuntu-latest`,
  `macos-latest`, `windows-latest`. Add an Ubuntu-only step to the `checks` job,
  before fmt/clippy/test, installing gpui's Linux system libraries:
  `libxcb-*`, `libxkbcommon-x11-dev`, `libwayland-dev`, `libssl-dev`,
  `libasound2-dev`, `libfontconfig-dev`, plus `vulkan`/`mesa` packages and
  `cmake`/`clang`. macOS and Windows need no extra system deps. (Exact package
  list to be finalized against the gpui-component Linux build docs during
  implementation.)
- **`scripts/github-actions/publish_crates.sh`**: append `vs-app` to the publish
  list (after `vs`, since it depends on `vs-core`).
- **Release / prebuilt-binary pipeline** (`build_binary.sh`, release matrix):
  **unchanged** — it builds `-p vs` (the CLI). The GUI is distributed via
  `cargo install vs-app` in v1. (The min-size/`panic=abort`/`cross` release
  profile is also a poor fit for a gpui binary, another reason to keep it out.)

## Testing strategy

- **Unit tests** in `model.rs` (and pure parts of `service.rs`) cover the
  row-merging/transform logic and run headlessly under `cargo test --workspace`
  on all three CI OSes. This is the real test coverage.
- **No `#[gpui::test]` window tests** in v1: they require a display backend and
  are flaky/heavy in CI. The view layer is kept thin precisely so little logic
  needs window-level testing.
- Manual verification: `cargo run -p vs-app` on macOS for the happy paths
  (add tool → install version → use → uninstall → remove plugin → refresh
  registry).

## Risks & mitigations

- **Strict lints vs gpui idioms:** gpui examples lean on `unwrap`. Mitigation:
  our code routes errors to toasts and uses `match`/`unwrap_or`; the testable
  layers stay panic-free by construction.
- **Linux CI compilation:** gpui needs system libs. Mitigation: the dedicated
  Ubuntu install step; finalize the package list against upstream docs.
- **`--all-features` blow-up:** gpui-component has heavy optional features.
  Mitigation: depend on it with minimal features and do not expose `vs-app`
  features that forward to them, so `clippy --all-features` stays bounded.
- **Cross-platform polish:** v1 targets functional parity, not per-OS visual
  perfection; Linux/Windows verified to compile + launch, deep polish deferred.

## Open items for implementation

- Finalize the exact Ubuntu apt package list for gpui.
- Confirm gpui-component's current API names for `Root`, `TitleBar`, modal,
  notification, and list/table widgets at the 0.5.x version pinned.
