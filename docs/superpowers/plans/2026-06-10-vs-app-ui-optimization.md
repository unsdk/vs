# vs-app UI Optimization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a real install progress bar, scrollable regions, a resizable + collapsible layout with a modal Add-tool, interaction polish, and inline failure feedback to the `vs-app` GUI.

**Architecture:** A backward-compatible optional progress callback (`ProgressFn`) is threaded through `vs-installer` → `vs-core` → `vs-app`; the GUI runs installs on the background executor and streams byte progress to a live `Progress` bar via an async channel. All other changes are contained in the `vs-app` UI layer (single-entity `RootView`): `h_resizable` two-pane, collapsible sidebar, styled scrollbars, a centered modal overlay for Add-tool, per-action loading buttons, status tags, empty states, and a dismissible inline `Alert` for failures.

**Tech Stack:** Rust 2024, gpui 0.2.2, gpui-component 0.5.1, `smol` channels (already in the gpui dependency tree). Workspace strict lint gate: deny `clippy::all`, `unwrap_used`, `expect_used`, `redundant_clone`, `needless_collect`, `rust-2018-idioms`, `unused-qualifications`; no `#[allow]`.

**Spec:** `docs/superpowers/specs/2026-06-10-vs-app-ui-optimization-design.md`

**Commit timestamps (REQUIRED):** every commit must be timestamped after 18:24 on 2026-06-09, continuing the existing incrementing-minute scheme. This plan's tasks use minutes :36 through :46. Each commit MUST export `GIT_AUTHOR_DATE` and `GIT_COMMITTER_DATE` (e.g. `"2026-06-09T18:36:00"`) and end the message with the `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>` trailer.

**gpui/gpui-component API note:** Where a builder below doesn't compile, the exact signature is in the vendored source at `~/.cargo/registry/src/index.crates.io-*/gpui-{0.2.2,component-0.5.1}/src/`. Match it and keep the documented behavior. Known-uncertain spots are flagged inline with a fallback.

---

## Task 1: Thread progress callback through `vs-installer`

Define a `ProgressFn` type alias in `vs-installer` (the lowest crate in the install path) and thread an optional callback through `download_bytes` → `materialize_artifact` → `Installer::install`. When a callback is supplied, skip the indicatif console bar and call the callback per chunk; when absent, behavior is unchanged.

**Files:**
- Modify: `crates/vs-installer/src/lib.rs` (add `ProgressFn` type + export; update in-file test callers of `install`)
- Modify: `crates/vs-installer/src/install.rs` (`download_bytes`, `materialize_artifact`, `Installer::install`)

- [ ] **Step 1: Add the `ProgressFn` type alias in `crates/vs-installer/src/lib.rs`**

Near the top of `crates/vs-installer/src/lib.rs` (after the existing `use`/module declarations, before or after `pub use`), add:

```rust
/// Reports download progress as `(bytes_downloaded_so_far, total_bytes_if_known)`.
///
/// Invoked synchronously on whatever thread runs the download. Optional — when
/// `None` the installer falls back to its console progress bar.
pub type ProgressFn<'a> = dyn Fn(u64, Option<u64>) + 'a;
```

Then ensure it is publicly reachable. If `lib.rs` re-exports install items (e.g. `pub use install::Installer;`), add `ProgressFn` to the crate root exports so `crate::ProgressFn` works. (It is defined at the crate root here, so `crate::ProgressFn` is already valid for `install.rs`.)

- [ ] **Step 2: Update `download_bytes` in `crates/vs-installer/src/install.rs`**

Replace the existing `download_bytes` method (currently lines ~286-319) with:

```rust
    fn download_bytes(
        &self,
        url: &str,
        headers: &std::collections::BTreeMap<String, String>,
        progress: Option<&crate::ProgressFn<'_>>,
    ) -> Result<Vec<u8>, InstallerError> {
        let client = self.http_client()?;
        let mut request = client.get(url);
        for (key, value) in headers {
            request = request.header(key, value);
        }
        let response = request
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|error| InstallerError::Download(error.to_string()))?;
        let total_size = response.content_length();
        // When a progress callback is supplied (GUI), skip the console bar to
        // avoid stray stderr output; otherwise keep the indicatif bar (CLI).
        let progress_bar = match progress {
            Some(_) => None,
            None => Some(create_download_progress_bar(total_size)),
        };
        let mut response = response;
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 8192];

        loop {
            let read = response
                .read(&mut buffer)
                .map_err(|error| InstallerError::Download(error.to_string()))?;
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if let Some(bar) = progress_bar.as_ref() {
                bar.inc(read as u64);
            }
            if let Some(callback) = progress {
                callback(bytes.len() as u64, total_size);
            }
        }

        if let Some(bar) = progress_bar.as_ref() {
            bar.finish_and_clear();
        }
        Ok(bytes)
    }
```

- [ ] **Step 3: Update `materialize_artifact` to accept and forward the callback**

In `crates/vs-installer/src/install.rs`, change the `materialize_artifact` signature (currently ~line 168) to add the `progress` parameter, and forward it on the `InstallSource::Url` branch:

```rust
    fn materialize_artifact(
        &self,
        artifact: &InstallArtifact,
        version_root: &Path,
        is_main: bool,
        progress: Option<&crate::ProgressFn<'_>>,
    ) -> Result<ArtifactPlacement, InstallerError> {
```

and within the `InstallSource::Url { url, headers } => { ... }` arm, change the download call from `self.download_bytes(url, headers)?` to:

```rust
                let bytes = self.download_bytes(url, headers, progress)?;
```

(Leave the rest of that arm and the other match arms unchanged.)

- [ ] **Step 4: Update `Installer::install` to accept and forward the callback**

In `crates/vs-installer/src/install.rs`, change the public `install` signature (currently ~line 86) and its two `materialize_artifact` calls:

```rust
    pub fn install(
        &self,
        plan: &InstallPlan,
        progress: Option<&crate::ProgressFn<'_>>,
    ) -> Result<InstalledRuntime, InstallerError> {
```

Update the main + additions calls:

```rust
        let main = self.materialize_artifact(&plan.main, &staged_install, true, progress)?;
        let mut additions = Vec::new();
        for artifact in &plan.additions {
            additions.push(self.materialize_artifact(artifact, &staged_install, false, progress)?);
        }
```

- [ ] **Step 5: Update `Installer::install` callers inside `crates/vs-installer/src/lib.rs`**

The crate's own tests call `.install(&plan)`. Find each (around lines 83, 107, 124 — search `rg "\.install\(&" crates/vs-installer/src/lib.rs`) and change each to pass `None`:

```rust
        // before: let result = installer.install(&plan);
        // after:
        let result = installer.install(&plan, None);
```

Apply the same `, None` addition to every `.install(&plan...)` call in that test module.

- [ ] **Step 6: Build and test `vs-installer`**

Run: `cargo test -p vs-installer && cargo clippy -p vs-installer --all-targets -- -D warnings`
Expected: compiles, existing tests pass, no warnings. (The download-callback path is mechanical plumbing; its numeric logic is unit-tested in `vs-app` Task 3 via `progress_percent`.)

- [ ] **Step 7: Commit**

```bash
cd /Users/admin/Documents/i/vs
git add crates/vs-installer/src/
GIT_AUTHOR_DATE="2026-06-09T18:36:00" GIT_COMMITTER_DATE="2026-06-09T18:36:00" git commit -m "$(cat <<'EOF'
feat(vs-installer): thread optional download progress callback

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Thread progress through `vs-core` and update all callers

Add the optional callback to `App::install_plugin_version`, re-export `ProgressFn` from `vs-core`, and update every caller across the workspace to pass `None` (CLI/exec/tests), preserving today's behavior off the GUI.

**Files:**
- Modify: `crates/vs-core/src/lib.rs` (re-export `ProgressFn`)
- Modify: `crates/vs-core/src/service/install.rs` (`install_plugin_version`)
- Modify: `crates/vs-core/src/service/exec.rs` (caller → `None`)
- Modify: `crates/vs-core/src/service/use_tool.rs` (test caller → `None`)
- Modify: `crates/vs-core/src/lib.rs` (test callers → `None`)
- Modify: `crates/vs-cli/src/install.rs`, `crates/vs-cli/src/tui.rs` (callers → `None`)
- Modify: `crates/vs-app/src/service.rs` (existing `install` caller → `None`)

- [ ] **Step 1: Re-export `ProgressFn` from `crates/vs-core/src/lib.rs`**

Add `vs_installer::ProgressFn` to vs-core's public exports. In `crates/vs-core/src/lib.rs`, near the existing `pub use` block, add:

```rust
pub use vs_installer::ProgressFn;
```

(Confirm `vs_installer` is the crate name used in vs-core's deps; if it is imported under a different path, match it. Check `crates/vs-core/Cargo.toml` for the `vs-installer` dependency.)

- [ ] **Step 2: Update `install_plugin_version` in `crates/vs-core/src/service/install.rs`**

Change the signature and the `self.installer.install(...)` call:

```rust
    pub fn install_plugin_version(
        &self,
        plugin_name: &str,
        version: Option<&str>,
        progress: Option<&crate::ProgressFn<'_>>,
    ) -> Result<InstalledVersion, CoreError> {
```

and:

```rust
        let runtime = self.installer.install(&plan, progress)?;
```

(Leave the rest of the function — registry resolve, version select, post_install rollback — unchanged.)

- [ ] **Step 3: Update the `exec.rs` caller**

In `crates/vs-core/src/service/exec.rs` (~line 62), change:

```rust
        // before: self.install_plugin_version(plugin_name, Some(version))?
        self.install_plugin_version(plugin_name, Some(version), None)?
```

- [ ] **Step 4: Update vs-core test callers**

In `crates/vs-core/src/lib.rs` (tests at ~lines 60, 273, 351, 352) and `crates/vs-core/src/service/use_tool.rs` (~line 368), add `, None` to each `install_plugin_version(...)` call. Search with `rg "install_plugin_version" crates/vs-core` to find them all. Example:

```rust
        // before: app.install_plugin_version("nodejs", Some("20.11.1"))?;
        app.install_plugin_version("nodejs", Some("20.11.1"), None)?;
```

- [ ] **Step 5: Update vs-cli callers**

In `crates/vs-cli/src/install.rs` (~lines 41, 86) and `crates/vs-cli/src/tui.rs` (~line 51), add `, None`:

```rust
        // install.rs line ~41:
        let installed = app.install_plugin_version(&plugin, Some(&version), None)?;
        // install.rs line ~86:
        match app.install_plugin_version(&plugin, Some(&version), None) {
        // tui.rs line ~51:
        app.install_plugin_version(plugin, Some(&selected.version), None)?
```

- [ ] **Step 6: Update the existing `vs-app` service caller**

In `crates/vs-app/src/service.rs` (the `install` method, ~line 75), change:

```rust
    /// Install a specific version (no progress reporting).
    pub fn install(&self, name: &str, version: &str) -> Result<InstalledVersion, CoreError> {
        self.core.install_plugin_version(name, Some(version), None)
    }
```

- [ ] **Step 7: Build and test the workspace**

Run: `cargo build --workspace && cargo test -p vs-core && cargo clippy --workspace --all-targets -- -D warnings`
Expected: compiles, vs-core tests pass, no warnings. (`cargo run -p vs -- --help` still works; CLI install path unchanged — indicatif bar intact because callers pass `None`.)

- [ ] **Step 8: Commit**

```bash
cd /Users/admin/Documents/i/vs
git add crates/vs-core/src/ crates/vs-cli/src/ crates/vs-app/src/service.rs
GIT_AUTHOR_DATE="2026-06-09T18:37:00" GIT_COMMITTER_DATE="2026-06-09T18:37:00" git commit -m "$(cat <<'EOF'
feat(vs-core): expose optional install progress callback

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: `vs-app` progress helper + `install_with_progress` service method

Add a pure, unit-tested percentage helper to `model.rs` and a progress-aware install method to `AppService`.

**Files:**
- Modify: `crates/vs-app/src/model.rs` (add `progress_percent` + tests)
- Modify: `crates/vs-app/src/service.rs` (add `install_with_progress`)

- [ ] **Step 1: Write the failing test for `progress_percent`**

Add to the `#[cfg(test)] mod tests` block in `crates/vs-app/src/model.rs`:

```rust
    #[test]
    fn progress_percent_computes_clamped_ratio_or_none() {
        assert_eq!(progress_percent(50, Some(200)), Some(25.0));
        assert_eq!(progress_percent(200, Some(200)), Some(100.0));
        // Over-report clamps to 100.
        assert_eq!(progress_percent(300, Some(200)), Some(100.0));
        // Unknown or zero total → None (caller shows a spinner).
        assert_eq!(progress_percent(10, None), None);
        assert_eq!(progress_percent(10, Some(0)), None);
    }
```

- [ ] **Step 2: Run the test to confirm it fails**

Run: `cargo test -p vs-app progress_percent`
Expected: FAIL — "cannot find function `progress_percent`".

- [ ] **Step 3: Implement `progress_percent`**

Add to `crates/vs-app/src/model.rs` (above the `#[cfg(test)]` block, with the other pure functions):

```rust
/// Compute a `0.0..=100.0` percentage from downloaded/total bytes.
///
/// Returns `None` when the total is unknown or zero, signalling the UI to show
/// an indeterminate spinner instead of a determinate bar.
pub fn progress_percent(done: u64, total: Option<u64>) -> Option<f32> {
    match total {
        Some(total) if total > 0 => {
            Some((done as f32 / total as f32 * 100.0).clamp(0.0, 100.0))
        }
        _ => None,
    }
}
```

Then re-export it from `crates/vs-app/src/lib.rs` alongside the other `model` re-exports (append `progress_percent` to the `pub use model::{...}` list) so it stays referenced and lint-clean.

- [ ] **Step 4: Run the test to confirm it passes**

Run: `cargo test -p vs-app progress_percent`
Expected: PASS.

- [ ] **Step 5: Add `install_with_progress` to `crates/vs-app/src/service.rs`**

Add this method to the `impl AppService` block, right after the existing `install` method:

```rust
    /// Install a specific version, reporting download progress via `on_progress`
    /// as `(downloaded_bytes, total_bytes_if_known)`.
    pub fn install_with_progress(
        &self,
        name: &str,
        version: &str,
        on_progress: &vs_core::ProgressFn<'_>,
    ) -> Result<InstalledVersion, CoreError> {
        self.core
            .install_plugin_version(name, Some(version), Some(on_progress))
    }
```

- [ ] **Step 6: Build and lint**

Run: `cargo test -p vs-app && cargo clippy -p vs-app --all-targets -- -D warnings`
Expected: all tests pass (including `progress_percent`), no warnings.

- [ ] **Step 7: Commit**

```bash
cd /Users/admin/Documents/i/vs
git add crates/vs-app/src/model.rs crates/vs-app/src/service.rs crates/vs-app/src/lib.rs
GIT_AUTHOR_DATE="2026-06-09T18:38:00" GIT_COMMITTER_DATE="2026-06-09T18:38:00" git commit -m "$(cat <<'EOF'
feat(vs-app): add progress_percent helper and install_with_progress

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: `run_action` refactor — per-action `pending` set + inline error state

Replace the global `busy: u32` counter with a `pending: HashSet<String>` keyed by action, add `last_error` state with a `set_error` helper, route `deferred_error` through it, and render a dismissible inline `Alert` banner under the title bar. This delivers the failure-feedback feature and lays the foundation for per-button loading (Task 9).

**Files:**
- Modify: `crates/vs-app/src/ui/root.rs` (state, `run_action`, `set_error`, `deferred_error`, `render`, `render_title_bar`)
- Modify: `crates/vs-app/src/ui/detail.rs` (pass action keys to `run_action`)
- Modify: `crates/vs-app/src/ui/add_tool.rs` (pass action keys to `run_action`)

- [ ] **Step 1: Update imports and struct in `root.rs`**

At the top of `crates/vs-app/src/ui/root.rs`, add the `HashSet` and `Alert` imports:

```rust
use std::collections::HashSet;
```

and with the other `gpui_component` imports:

```rust
use gpui_component::alert::Alert;
```

Replace the `busy` field in the `RootView` struct with `pending` and add `last_error`:

```rust
    // remove: pub(crate) busy: u32,
    /// Action keys for in-flight operations (e.g. "install:nodejs:21.6.0").
    pub(crate) pending: HashSet<String>,
    /// Last error message, shown as a dismissible inline banner. None = hidden.
    pub(crate) last_error: Option<String>,
```

In `RootView::new`, replace `busy: 0,` in the initializer with:

```rust
            pending: HashSet::new(),
            last_error: None,
```

- [ ] **Step 2: Rewrite `run_action` to use the `pending` set + `set_error`**

Replace the entire `run_action` method in `root.rs` with (note the new `key: String` parameter):

```rust
    /// Run a blocking `service` operation off the UI thread under action `key`
    /// (so its button can show a loading state), toast the outcome, surface any
    /// error inline, then refresh the detail + sidebar lists.
    pub(crate) fn run_action<F>(
        &mut self,
        key: String,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
        op: F,
    ) where
        F: FnOnce(AppService) -> Result<String, CoreError> + Send + 'static,
    {
        let service = self.service.clone();
        self.pending.insert(key.clone());
        cx.spawn_in(window, async move |this, cx| {
            let result = cx.background_spawn(async move { op(service) }).await;
            this.update_in(cx, |this, window, cx| {
                this.pending.remove(&key);
                match result {
                    Ok(msg) => window.push_notification(
                        Notification::from((NotificationType::Success, SharedString::from(msg))),
                        cx,
                    ),
                    Err(err) => {
                        let msg = format!("{err}");
                        this.set_error(msg.clone());
                        window.push_notification(
                            Notification::from((NotificationType::Error, SharedString::from(msg))),
                            cx,
                        );
                    }
                }
                this.reload_detail(cx);
                this.reload_tools(cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
```

- [ ] **Step 3: Add `set_error` and route `deferred_error` through it**

Replace the existing `deferred_error` method with these two methods:

```rust
    /// Record an error message: log it and surface it as an inline banner.
    pub(crate) fn set_error(&mut self, message: String) {
        eprintln!("vs-app: {message}");
        self.last_error = Some(message);
    }

    /// Surface a `CoreError` from a load path as an inline banner.
    pub(crate) fn deferred_error(&mut self, err: CoreError, _cx: &mut Context<'_, Self>) {
        self.set_error(format!("{err}"));
    }
```

(`deferred_error` now takes `&mut self`; its callers in `reload_tools`/`reload_detail`/`load_registry`/`run_search` already invoke it as `this.deferred_error(err, cx)` where `this` is `&mut Self`, so they keep compiling.)

- [ ] **Step 4: Render the inline `Alert` banner in `render`**

Replace the `Render` impl's outer `div` chain so the banner appears between the title bar and the body:

```rust
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(self.render_title_bar(window, cx))
            .when_some(self.last_error.clone(), |el, msg| {
                el.child(
                    div().px_3().py_1().child(
                        Alert::error("vs-error", msg).on_close(cx.listener(
                            |this, _ev, _window, cx| {
                                this.last_error = None;
                                cx.notify();
                            },
                        )),
                    ),
                )
            })
            .child(body)
```

(If `Alert::on_close` does not accept a `cx.listener(...)` closure shape, capture `let entity = cx.entity();` before building the `Alert` and use `move |_ev, _window, cx| { entity.update(cx, |this, cx| { this.last_error = None; cx.notify(); }); }` instead — verify against the vendored `alert.rs`. `when_some` is gpui's `FluentBuilder` combinator; if absent, use `if let Some(msg) = self.last_error.clone() { ... }` to build the child conditionally.)

- [ ] **Step 5: Update `render_title_bar`'s busy hint to use `pending`**

In `render_title_bar`, change the busy hint and give the refresh button its action key:

```rust
            // busy hint:
            .when(!self.pending.is_empty(), |el| el.child(div().child("working…")))
```

and update the refresh button's click to pass a key:

```rust
                        Button::new("refresh-registry")
                            .label("Refresh registry")
                            .on_click(cx.listener(|this, _ev, window, cx| {
                                this.run_action("refresh".to_string(), window, cx, |svc| {
                                    svc.refresh_registry()
                                        .map(|n| format!("Registry refreshed: {n} plugins"))
                                });
                            })),
```

- [ ] **Step 6: Pass action keys at every `run_action` call site in `detail.rs`**

In `crates/vs-app/src/ui/detail.rs`, update each `run_action` call to pass a key built from the already-cloned name/version:

Use button (in `render_installed_row`):

```rust
                        move |this, _ev, window, cx| {
                            let (n, v) = (use_name.clone(), use_version.clone());
                            let key = format!("use:{n}:{v}");
                            this.run_action(key, window, cx, move |svc| {
                                svc.use_version(&n, &v, scope)
                                    .map(|iv| format!("Now using {}@{}", iv.plugin, iv.version))
                            });
                        },
```

Uninstall button:

```rust
                        move |this, _ev, window, cx| {
                            let (n, v) = (uninstall_name.clone(), uninstall_version.clone());
                            let key = format!("uninstall:{n}:{v}");
                            this.run_action(key, window, cx, move |svc| {
                                svc.uninstall(&n, &v).map(|res| {
                                    if res.removed {
                                        match res.auto_switched {
                                            Some(sw) => format!(
                                                "Uninstalled {n}@{v}; auto-switched to {sw}"
                                            ),
                                            None => format!("Uninstalled {n}@{v}"),
                                        }
                                    } else {
                                        format!("{n}@{v} was not installed")
                                    }
                                })
                            });
                        },
```

Install button (in `render_available_row`):

```rust
                    .on_click(cx.listener(move |this, _ev, window, cx| {
                        let (n, v) = (install_name.clone(), install_version.clone());
                        let key = format!("install:{n}:{v}");
                        this.run_action(key, window, cx, move |svc| {
                            svc.install(&n, &v)
                                .map(|iv| format!("Installed {}@{}", iv.plugin, iv.version))
                        });
                    })),
```

Update plugin button (in `render_detail` header):

```rust
                                    .on_click(cx.listener(move |this, _ev, window, cx| {
                                        let n = update_name.clone();
                                        let key = format!("update:{n}");
                                        this.run_action(key, window, cx, move |svc| {
                                            svc.update_plugin(&n).map(|()| format!("Updated {n}"))
                                        });
                                    })),
```

Remove plugin button:

```rust
                                    .on_click(cx.listener(move |this, _ev, window, cx| {
                                        let n = remove_name.clone();
                                        this.selected = None;
                                        let key = format!("remove:{n}");
                                        this.run_action(key, window, cx, move |svc| {
                                            svc.remove_plugin(&n).map(|removed| {
                                                if removed {
                                                    format!("Removed {n}")
                                                } else {
                                                    format!("{n} was not present")
                                                }
                                            })
                                        });
                                    })),
```

- [ ] **Step 7: Pass action keys at every `run_action` call site in `add_tool.rs`**

In `crates/vs-app/src/ui/add_tool.rs`, registry-add button:

```rust
                            .on_click(cx.listener(move |this, _ev, window, cx| {
                                let n = add_name.clone();
                                this.show_add = false;
                                let key = format!("add:{n}");
                                this.run_action(key, window, cx, move |svc| {
                                    svc.add(AddSource::Registry { name: n.clone() })
                                        .map(|()| format!("Added {n}"))
                                });
                            })),
```

Add-from-source button:

```rust
                    .on_click(cx.listener(move |this, _ev, window, cx| {
                        let source = this.add_source_input.read(cx).value().to_string();
                        let alias_raw = this.add_alias_input.read(cx).value().to_string();
                        let alias = if alias_raw.trim().is_empty() {
                            None
                        } else {
                            Some(alias_raw)
                        };
                        if source.trim().is_empty() {
                            return;
                        }
                        this.show_add = false;
                        let backend = this.add_backend;
                        let key = format!("add-source:{source}");
                        this.run_action(key, window, cx, move |svc| {
                            svc.add(AddSource::Source {
                                source: source.clone(),
                                alias,
                                backend,
                            })
                            .map(|()| format!("Added from {source}"))
                        });
                    })),
```

- [ ] **Step 8: Build, lint, run**

Run: `cargo build -p vs-app && cargo clippy -p vs-app --all-targets -- -D warnings`
Expected: compiles, no warnings (no dead `busy`; `pending` read by the title-bar hint, `last_error` read by the banner).
Then: `( cargo run -p vs-app & P=$!; sleep 25; kill $P 2>/dev/null )` — reaches the event loop without panic. Trigger no action needed yet; just confirm clean launch.

- [ ] **Step 9: Commit**

```bash
cd /Users/admin/Documents/i/vs
git add crates/vs-app/src/ui/
GIT_AUTHOR_DATE="2026-06-09T18:39:00" GIT_COMMITTER_DATE="2026-06-09T18:39:00" git commit -m "$(cat <<'EOF'
feat(vs-app): per-action pending tracking and inline error banner

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Resizable two-pane + collapsible sidebar

Wrap the sidebar/detail body in a draggable `h_resizable` split and add a collapse toggle. (The Add-tool branch stays as-is here; Task 8 converts it to a modal overlay.)

**Files:**
- Modify: `crates/vs-app/src/ui/root.rs` (state, `new`, `render` body, collapsed strip)
- Modify: `crates/vs-app/src/ui/sidebar.rs` (collapse toggle in header)

- [ ] **Step 1: Add layout state to `RootView`**

In `crates/vs-app/src/ui/root.rs`, add imports:

```rust
use gpui_component::resizable::{ResizableState, h_resizable, resizable_panel};
```

(Verify these names against `~/.cargo/registry/src/index.crates.io-*/gpui-component-0.5.1/src/resizable/`. If the panel constructor is `h_resizable_panel()` instead of `resizable_panel()`, use that. If state is created differently, adjust Step 2.)

Add fields to `RootView`:

```rust
    pub(crate) resizable_state: Entity<ResizableState>,
    pub(crate) sidebar_collapsed: bool,
```

- [ ] **Step 2: Initialize the new fields in `RootView::new`**

In `RootView::new`, before building `Self { ... }`, create the resizable state:

```rust
        let resizable_state = ResizableState::new(cx);
```

(Verify the constructor. Common alternatives if that fails: `ResizableState::new(window, cx)`, or `cx.new(|_| ResizableState::default())`. The field type is `Entity<ResizableState>`.)

Add to the `Self { ... }` initializer:

```rust
            resizable_state,
            sidebar_collapsed: false,
```

- [ ] **Step 3: Rebuild the `render` body with the resizable split**

In the `Render` impl, replace the `body` computation with a three-way branch (add panel / collapsed / resizable):

```rust
        let body = if self.show_add {
            self.render_add_panel(window, cx).into_any_element()
        } else if self.sidebar_collapsed {
            div()
                .flex()
                .flex_1()
                .child(self.render_sidebar_collapsed(cx))
                .child(self.render_detail(window, cx))
                .into_any_element()
        } else {
            h_resizable("vs-main")
                .with_state(&self.resizable_state)
                .child(
                    resizable_panel()
                        .size(px(220.0))
                        .size_range(px(160.0)..px(420.0))
                        .child(self.render_sidebar(window, cx)),
                )
                .child(resizable_panel().child(self.render_detail(window, cx)))
                .into_any_element()
        };
```

(Verify `h_resizable(id).with_state(&entity).child(panel)` and `resizable_panel().size(px).size_range(range).child(body)` against the vendored example. If `with_state` isn't the binding method, use the one the example shows — e.g. state may be passed to `h_resizable` directly. If `.child(body)` on a panel is `.body(body)` instead, use that. Keep the two panels + sizing behavior.)

- [ ] **Step 4: Add the collapsed-strip renderer to `root.rs`**

Add this method in an `impl RootView` block:

```rust
    /// Thin strip shown when the sidebar is collapsed; expands on click.
    fn render_sidebar_collapsed(&mut self, cx: &mut Context<'_, Self>) -> impl IntoElement {
        use gpui_component::button::Button;
        div()
            .w(px(36.0))
            .flex()
            .flex_col()
            .items_center()
            .p_1()
            .child(
                Button::new("expand-sidebar")
                    .label("›")
                    .on_click(cx.listener(|this, _ev, _window, cx| {
                        this.sidebar_collapsed = false;
                        cx.notify();
                    })),
            )
    }
```

- [ ] **Step 5: Add the collapse toggle to the sidebar header in `sidebar.rs`**

In `crates/vs-app/src/ui/sidebar.rs`, in the header row (the `div().flex().gap_1()` holding the filter `Input` and `＋ Add` button), prepend a collapse button as the first child:

```rust
                div()
                    .flex()
                    .gap_1()
                    .child(
                        Button::new("collapse-sidebar")
                            .label("‹")
                            .on_click(cx.listener(|this, _ev, _window, cx| {
                                this.sidebar_collapsed = true;
                                cx.notify();
                            })),
                    )
                    .child(div().flex_1().child(Input::new(&self.filter_input)))
                    .child(
                        Button::new("add-tool")
                            .label("＋ Add")
                            .on_click(cx.listener(|this, _ev, window, cx| {
                                this.open_add_tool(window, cx);
                            })),
                    ),
```

- [ ] **Step 6: Build, lint, run**

Run: `cargo build -p vs-app && cargo clippy -p vs-app --all-targets -- -D warnings`
Expected: compiles, no warnings.
Then: `( cargo run -p vs-app & P=$!; sleep 25; kill $P 2>/dev/null )` — launches without panic.

- [ ] **Step 7: Commit**

```bash
cd /Users/admin/Documents/i/vs
git add crates/vs-app/src/ui/
GIT_AUTHOR_DATE="2026-06-09T18:40:00" GIT_COMMITTER_DATE="2026-06-09T18:40:00" git commit -m "$(cat <<'EOF'
feat(vs-app): resizable two-pane and collapsible sidebar

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Scrollable regions

Give the sidebar tool list and the detail INSTALLED/AVAILABLE sections their own scroll regions so long lists stay inside the window.

**Files:**
- Modify: `crates/vs-app/src/ui/sidebar.rs`
- Modify: `crates/vs-app/src/ui/detail.rs`

- [ ] **Step 1: Make the sidebar tool list scroll**

In `crates/vs-app/src/ui/sidebar.rs`, add the import:

```rust
use gpui_component::scroll::ScrollableElement;
```

Restructure `render_sidebar` so the header (collapse + filter + Add) stays fixed and only the TOOLS list scrolls. Move the `TOOLS` label and the `.children(...)` tool rows into a scrollable child:

```rust
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap_2()
            .p_2()
            .child(/* the header row from Task 5, unchanged */)
            .child(
                div()
                    .id("sidebar-scroll")
                    .flex_1()
                    .overflow_y_scrollbar()
                    .child(div().text_xs().child("TOOLS"))
                    .children(visible.into_iter().enumerate().map(|(ix, tool)| {
                        // ... unchanged tool-row closure from current code ...
                    })),
            )
```

(Keep the existing tool-row closure body verbatim. `overflow_y_scrollbar()` comes from `ScrollableElement`. The scroll container needs an `.id(...)` — add `"sidebar-scroll"`. If `overflow_y_scrollbar` requires the element be `Stateful` first, the `.id()` provides that. Verify against `scroll/scrollable.rs`; fallback to gpui core `.overflow_y_scroll().track_scroll(&handle)` with a stored `ScrollHandle` if the styled wrapper proves awkward.)

Note the outer column changes width from `.w(px(210.0))` to `.w_full()` because Task 5's `resizable_panel().size(...)` now owns the width.

- [ ] **Step 2: Make the detail version sections scroll**

In `crates/vs-app/src/ui/detail.rs`, add the import:

```rust
use gpui_component::scroll::ScrollableElement;
```

Restructure `render_detail` so the tool header and the AVAILABLE search box stay pinned, and the INSTALLED + AVAILABLE rows live in a single scrollable region. Wrap the four section children (`"INSTALLED"` label, `installed_rows`, `"AVAILABLE"` label, search box, `available_rows`) like this — keep the header `.child({ ... update/remove ... })` outside the scroll area, and put the rest inside:

```rust
        div()
            .flex_1()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .child(/* the tool-name + Update/Remove header block, unchanged */)
            .child(
                div()
                    .id("detail-scroll")
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .overflow_y_scrollbar()
                    .child(div().text_xs().child("INSTALLED"))
                    .children(installed_rows)
                    .child(div().text_xs().child("AVAILABLE"))
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .child(div().flex_1().child(Input::new(&self.search_input)))
                            .child(
                                Button::new("search").label("Search").on_click(
                                    cx.listener(|this, _ev, _window, cx| this.run_search(cx)),
                                ),
                            ),
                    )
                    .children(available_rows),
            )
```

(Same `overflow_y_scrollbar()` + `.id()` notes as Step 1.)

- [ ] **Step 3: Build, lint, run**

Run: `cargo build -p vs-app && cargo clippy -p vs-app --all-targets -- -D warnings`
Expected: compiles, no warnings.
Then: `( cargo run -p vs-app & P=$!; sleep 25; kill $P 2>/dev/null )` — launches without panic; with many tools/versions the lists scroll inside their panes.

- [ ] **Step 4: Commit**

```bash
cd /Users/admin/Documents/i/vs
git add crates/vs-app/src/ui/
GIT_AUTHOR_DATE="2026-06-09T18:41:00" GIT_COMMITTER_DATE="2026-06-09T18:41:00" git commit -m "$(cat <<'EOF'
feat(vs-app): scrollable sidebar and detail sections

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Live install progress bar

Add a `progress` field + `InstallProgress` struct, a `run_install` method that streams byte progress over a channel, and a `Progress`/`Spinner` in the Available row during install.

**Files:**
- Modify: `Cargo.toml` (workspace dep `smol`)
- Modify: `crates/vs-app/Cargo.toml` (add `smol`)
- Modify: `crates/vs-app/src/ui/root.rs` (`InstallProgress`, `progress` field, `run_install`)
- Modify: `crates/vs-app/src/ui/detail.rs` (call `run_install`, render the bar)

- [ ] **Step 1: Add the `smol` dependency**

In the root `Cargo.toml` `[workspace.dependencies]`, add (align the major with gpui's `smol`, which is `2`):

```toml
smol = "2"
```

In `crates/vs-app/Cargo.toml` `[dependencies]`, add:

```toml
smol.workspace = true
```

- [ ] **Step 2: Add `InstallProgress` + `progress` field in `root.rs`**

Add the struct near `AddTab` in `crates/vs-app/src/ui/root.rs`:

```rust
/// Live state for an in-flight install download.
#[derive(Clone, Debug)]
pub(crate) struct InstallProgress {
    pub(crate) key: String,
    pub(crate) done: u64,
    pub(crate) total: Option<u64>,
}
```

Add the field to `RootView`:

```rust
    pub(crate) progress: Option<InstallProgress>,
```

Initialize in `RootView::new`'s `Self { ... }`:

```rust
            progress: None,
```

- [ ] **Step 3: Add the `run_install` method to `root.rs`**

Add inside an `impl RootView` block (the channel sender feeds the foreground drain; when the install future finishes, the sender drops, closing the channel and ending the drain loop):

```rust
    /// Install a version, streaming download progress into `self.progress`.
    pub(crate) fn run_install(
        &mut self,
        name: String,
        version: String,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let key = format!("install:{name}:{version}");
        self.pending.insert(key.clone());
        self.progress = Some(InstallProgress {
            key: key.clone(),
            done: 0,
            total: None,
        });
        let (tx, rx) = smol::channel::unbounded::<(u64, Option<u64>)>();

        // Foreground drain: apply progress events to state until the channel closes.
        cx.spawn(async move |this, cx| {
            while let Ok((done, total)) = rx.recv().await {
                if this
                    .update(cx, |this, cx| {
                        if let Some(p) = this.progress.as_mut() {
                            p.done = done;
                            p.total = total;
                        }
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();

        // Background install: report bytes via the channel, then finalize.
        let service = self.service.clone();
        let install_name = name;
        let install_version = version;
        cx.spawn_in(window, async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    service.install_with_progress(
                        &install_name,
                        &install_version,
                        &|done, total| {
                            let _ = tx.try_send((done, total));
                        },
                    )
                })
                .await;
            this.update_in(cx, |this, window, cx| {
                this.pending.remove(&key);
                this.progress = None;
                match result {
                    Ok(iv) => window.push_notification(
                        Notification::from((
                            NotificationType::Success,
                            SharedString::from(format!("Installed {}@{}", iv.plugin, iv.version)),
                        )),
                        cx,
                    ),
                    Err(err) => {
                        let msg = format!("{err}");
                        this.set_error(msg.clone());
                        window.push_notification(
                            Notification::from((NotificationType::Error, SharedString::from(msg))),
                            cx,
                        );
                    }
                }
                this.reload_detail(cx);
                this.reload_tools(cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
```

- [ ] **Step 4: Render the progress bar in the Available row (`detail.rs`)**

In `crates/vs-app/src/ui/detail.rs`, add imports:

```rust
use gpui_component::Sizable;
use gpui_component::progress::Progress;
use gpui_component::spinner::Spinner;

use crate::model::progress_percent;
```

Change the Install button's click handler in `render_available_row` to call `run_install` (replacing the `run_action` install call from Task 4):

```rust
                    .on_click(cx.listener(move |this, _ev, window, cx| {
                        this.run_install(install_name.clone(), install_version.clone(), window, cx);
                    })),
```

Still in `render_available_row`, compute the per-row progress (this method takes `&self`, so it can read `self.progress`) just after the existing `let install_version = row.version;`:

```rust
        let progress_key = format!("install:{name}:{version}", name = name, version = install_version);
        let row_progress = self
            .progress
            .as_ref()
            .filter(|p| p.key == progress_key)
            .map(|p| progress_percent(p.done, p.total));
```

Then add a progress element below the row. Wrap the existing row `div(...)` in an outer column and append the bar when installing:

```rust
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                // ... the existing `div().id(("available", ix)) ... .child(Button...)` row ...
            )
            .when_some(row_progress, |el, pct| match pct {
                Some(value) => el.child(div().w_full().child(Progress::new().value(value))),
                None => el.child(Spinner::new().with_size(gpui_component::Size::Small)),
            })
```

(Verify `Progress::new().value(f32)` and `Spinner::new().with_size(Size::Small)` against the vendored `progress.rs`/`spinner.rs`. `gpui_component::Size` may be re-exported at the crate root as `gpui_component::Size`. If `when_some` is unavailable, use an `if let Some(pct) = row_progress { ... }` to build the child.)

- [ ] **Step 5: Build, lint, run**

Run: `cargo build -p vs-app && cargo clippy -p vs-app --all-targets -- -D warnings`
Expected: compiles, no warnings.
Then: `( cargo run -p vs-app & P=$!; sleep 25; kill $P 2>/dev/null )` — launches without panic. Manually: installing a version shows a live bar that advances and disappears on completion (verified during Task 10 acceptance).

- [ ] **Step 6: Commit**

```bash
cd /Users/admin/Documents/i/vs
git add Cargo.toml Cargo.lock crates/vs-app/Cargo.toml crates/vs-app/src/ui/
GIT_AUTHOR_DATE="2026-06-09T18:42:00" GIT_COMMITTER_DATE="2026-06-09T18:42:00" git commit -m "$(cat <<'EOF'
feat(vs-app): live install progress bar via streamed download progress

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Add-tool as a centered modal overlay

Stop letting Add-tool take over the body; render it as a centered card over a dimmed backdrop, with the two-pane content preserved underneath.

**Files:**
- Modify: `crates/vs-app/src/ui/root.rs` (`render` stacks the overlay)
- Modify: `crates/vs-app/src/ui/add_tool.rs` (`render_add_modal` wrapper; scroll registry list; drop full-body sizing)

- [ ] **Step 1: Restructure `render` to layer the modal**

In `crates/vs-app/src/ui/root.rs`, change `render` so `body` no longer branches on `show_add`, the root is `.relative()`, and the modal is an overlay child appended last:

```rust
impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let body = if self.sidebar_collapsed {
            div()
                .flex()
                .flex_1()
                .child(self.render_sidebar_collapsed(cx))
                .child(self.render_detail(window, cx))
                .into_any_element()
        } else {
            h_resizable("vs-main")
                .with_state(&self.resizable_state)
                .child(
                    resizable_panel()
                        .size(px(220.0))
                        .size_range(px(160.0)..px(420.0))
                        .child(self.render_sidebar(window, cx)),
                )
                .child(resizable_panel().child(self.render_detail(window, cx)))
                .into_any_element()
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .relative()
            .child(self.render_title_bar(window, cx))
            .when_some(self.last_error.clone(), |el, msg| {
                el.child(
                    div().px_3().py_1().child(
                        Alert::error("vs-error", msg).on_close(cx.listener(
                            |this, _ev, _window, cx| {
                                this.last_error = None;
                                cx.notify();
                            },
                        )),
                    ),
                )
            })
            .child(body)
            .when(self.show_add, |el| el.child(self.render_add_modal(window, cx)))
    }
}
```

- [ ] **Step 2: Add `render_add_modal` in `add_tool.rs`**

In `crates/vs-app/src/ui/add_tool.rs`, add imports:

```rust
use gpui::px;
use gpui_component::ActiveTheme;
```

Add the modal wrapper method (it reuses `render_add_panel` as the card body):

```rust
    pub(crate) fn render_add_modal(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl IntoElement {
        div()
            .absolute()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::rgba(0x0000_0088))
            .occlude()
            .child(
                div()
                    .w(px(560.0))
                    .max_h(px(520.0))
                    .bg(cx.theme().background)
                    .rounded(px(8.0))
                    .shadow_lg()
                    .p_4()
                    .child(self.render_add_panel(window, cx)),
            )
    }
```

(Verify `cx.theme().background` via `gpui_component::ActiveTheme`; if unavailable, fall back to `gpui::rgb(0x1e1e_1e)`. `.occlude()` blocks click-through to the body. `.absolute().size_full()` covers the `.relative()` root from Step 1. The existing "Close" button inside `render_add_panel` dismisses the modal.)

- [ ] **Step 3: Drop full-body sizing + scroll the registry list in `render_add_panel`/`render_add_registry`**

Since the panel now lives inside a fixed-size card, remove the `.flex_1()` from `render_add_panel`'s outer `div` (keep `.flex().flex_col().gap_2().p_4()` — or drop `.p_4()` too since the card already pads; keep `.gap_2()`).

In `render_add_registry`, make the results list scroll within the card. Add `use gpui_component::scroll::ScrollableElement;` at the top of `add_tool.rs`, and wrap the rows:

```rust
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(Input::new(&self.add_filter_input))
            .child(
                div()
                    .id("add-registry-scroll")
                    .max_h(px(320.0))
                    .overflow_y_scrollbar()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(rows),
            )
```

- [ ] **Step 4: Build, lint, run**

Run: `cargo build -p vs-app && cargo clippy -p vs-app --all-targets -- -D warnings`
Expected: compiles, no warnings.
Then: `( cargo run -p vs-app & P=$!; sleep 25; kill $P 2>/dev/null )` — launches; ＋ Add opens a centered modal over the dimmed two-pane, Close dismisses it.

- [ ] **Step 5: Commit**

```bash
cd /Users/admin/Documents/i/vs
git add crates/vs-app/src/ui/
GIT_AUTHOR_DATE="2026-06-09T18:43:00" GIT_COMMITTER_DATE="2026-06-09T18:43:00" git commit -m "$(cat <<'EOF'
feat(vs-app): render Add-tool as a centered modal overlay

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: Interaction polish — loading buttons, variants/icons, status tag, empty states

**Files:**
- Modify: `crates/vs-app/src/ui/detail.rs`
- Modify: `crates/vs-app/src/ui/sidebar.rs`

- [ ] **Step 1: Imports for variants, icons, and tag in `detail.rs`**

Add to `crates/vs-app/src/ui/detail.rs`:

```rust
use gpui_component::button::ButtonVariants;
use gpui_component::tag::Tag;
use gpui_component::{Icon, IconName, Size};
```

(`ButtonVariants` enables `.primary()`/`.danger()`; `Sizable` is already imported from Task 7 for `.with_size`. Confirm `IconName::Delete`, `IconName::Search` exist — they do; if `Icon`/`Size`/`IconName` live at a different path, adjust per the vendored `icon.rs`.)

- [ ] **Step 2: Per-action loading + variants/icons on the detail buttons**

In `render_installed_row`, compute keys up front and apply loading + variants:

```rust
        let use_key = format!("use:{name}:{}", row.version);
        let uninstall_key = format!("uninstall:{name}:{}", row.version);
        let use_loading = self.pending.contains(&use_key);
        let uninstall_loading = self.pending.contains(&uninstall_key);
```

Apply to the Use button: `.loading(use_loading).disabled(use_loading)`. Apply to the Uninstall button: `.danger().icon(Icon::new(IconName::Delete)).loading(uninstall_loading).disabled(uninstall_loading)`.

In `render_available_row`, the install key already exists as `progress_key`; add `let install_loading = self.pending.contains(&progress_key);` and apply to the Install button: `.primary().loading(install_loading).disabled(already || install_loading)`.

In the `render_detail` header, for the Update/Remove buttons read pending too (these are built in `render_detail` which has `&mut self`, so compute before the closures):

```rust
        let update_loading = self.pending.contains(&format!("update:{name}"));
        let remove_loading = self.pending.contains(&format!("remove:{name}"));
```

Apply `.loading(update_loading).disabled(update_loading)` to Update plugin, and `.danger().icon(Icon::new(IconName::Delete)).loading(remove_loading).disabled(remove_loading)` to Remove plugin.

- [ ] **Step 3: Replace the current-version text with a `Tag`**

In `render_installed_row`, replace the `label` string logic and the left child. Instead of `format!("{}  ✓ current", row.version)`, render the version plus a tag:

```rust
        let version_text = row.version.clone();
        let is_current = row.current;
        // left child:
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(div().child(version_text))
            .when(is_current, |el| {
                el.child(Tag::success().with_size(Size::Small).child("current"))
            })
```

Use this `div` as the row's first `.child(...)` (replacing `div().child(label)`). Remove the now-unused `label` binding.

- [ ] **Step 4: Empty states**

In `render_available_row`'s caller `render_detail`: after building `available_rows`, if `self.available.is_empty()` and a tool is selected, push a hint row. Simplest: in `render_detail`, change the `.children(available_rows)` inside the scroll region to also handle empty — before the loop, if `available.is_empty()`, set `available_rows` to a single hint element:

```rust
        // after the available_rows loop:
        if available_rows.is_empty() {
            available_rows.push(
                div()
                    .py_1()
                    .text_xs()
                    .child("No versions yet — type a query and press Search.")
                    .into_any_element(),
            );
        }
```

(Note: `render_available_row` returns `impl IntoElement + use<>`; to mix it with a plain `div` in the same `Vec`, collect `available_rows` as `Vec<AnyElement>` by calling `.into_any_element()` on each row. Update the loop accordingly: `available_rows.push(self.render_available_row(ix, &name, row, cx).into_any_element());` and declare `let mut available_rows: Vec<gpui::AnyElement> = Vec::new();`. Do the same for the empty-tools sidebar case below if needed.)

In `crates/vs-app/src/ui/sidebar.rs`, handle the no-tools case: when `self.tools.is_empty()`, render a prompt instead of the (empty) list. Inside the scrollable region, after the `TOOLS` label:

```rust
                    .when(self.tools.is_empty(), |el| {
                        el.child(
                            div()
                                .p_2()
                                .text_xs()
                                .child("No tools yet — click ＋ Add to add one."),
                        )
                    })
```

- [ ] **Step 5: Build, lint, run**

Run: `cargo build -p vs-app && cargo clippy -p vs-app --all-targets -- -D warnings`
Expected: compiles, no warnings. Watch for: unused `label` binding removed; `AnyElement` import (`gpui::AnyElement`) added where mixing element types in a `Vec`.
Then: `( cargo run -p vs-app & P=$!; sleep 25; kill $P 2>/dev/null )` — launches without panic.

- [ ] **Step 6: Commit**

```bash
cd /Users/admin/Documents/i/vs
git add crates/vs-app/src/ui/
GIT_AUTHOR_DATE="2026-06-09T18:44:00" GIT_COMMITTER_DATE="2026-06-09T18:44:00" git commit -m "$(cat <<'EOF'
feat(vs-app): loading buttons, danger variants, status tags, empty states

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: Full verification gate + manual acceptance

**Files:** none (verification + any fmt cleanup only)

- [ ] **Step 1: Format and run the exact CI gate**

```bash
cd /Users/admin/Documents/i/vs
cargo fmt --all
cargo fmt --all --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --workspace
cargo test --doc --workspace
```
Expected: fmt clean, clippy exit 0, all tests pass (including `progress_percent` and the `vs-app` model/service tests).

- [ ] **Step 2: Confirm the CLI is unaffected**

```bash
cargo run -p vs -- --help
```
Expected: the `vs` CLI help prints unchanged. (Optionally, an actual `vs install` still shows the indicatif console bar — the `None` callback path.)

- [ ] **Step 3: Manual GUI acceptance (`cargo run -p vs-app`)**

Confirm each:
  - Window opens; sidebar lists tools (or the empty-state prompt). Drag the divider to resize; collapse/expand the sidebar with `‹`/`›`.
  - Long tool/version lists scroll within their panes.
  - ＋ Add opens a centered modal over a dimmed backdrop; registry list scrolls; Add closes it and the tool appears in the sidebar.
  - Select a tool → INSTALLED (current marked with a green "current" tag) + AVAILABLE.
  - Search → Install: the clicked button shows loading, a live progress bar advances, a success toast fires, the version moves to Installed, and the bar disappears.
  - Use / Uninstall / Update / Remove: buttons show loading; danger styling on Uninstall/Remove; toasts fire; Remove clears the selection.
  - Trigger a failure (e.g. install a bogus version, or disconnect network): an inline red `Alert` banner appears under the title bar and persists until dismissed, alongside the toast.
  - The window stays responsive during a real install (work is off the UI thread).

- [ ] **Step 4: Final commit (if fmt or cleanup changed anything)**

```bash
cd /Users/admin/Documents/i/vs
git add -A
GIT_AUTHOR_DATE="2026-06-09T18:45:00" GIT_COMMITTER_DATE="2026-06-09T18:45:00" git commit -m "$(cat <<'EOF'
chore(vs-app): rustfmt and verification cleanups for UI optimization

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)" || echo "nothing to commit"
```

- [ ] **Step 5: Integration handoff**

The branch `feat/vs-app-gui` now contains the UI optimization. Use the
`superpowers:finishing-a-development-branch` skill to decide integration
(merge / PR / keep). The CI Linux gpui-deps step (added in the prior plan) must
remain for the Ubuntu check to pass.

---

## Notes for the implementer

- **Vendored source is the API reference.** When a gpui/gpui-component builder
  here doesn't compile, the exact signature is in
  `~/.cargo/registry/src/index.crates.io-*/gpui-{0.2.2,component-0.5.1}/src/`.
  Match it and keep the documented behavior. The flagged risky spots:
  `h_resizable`/`resizable_panel`/`ResizableState` construction, the
  `overflow_y_scrollbar` scroll wrapper (may need an `.id()`), `Progress::value`,
  `Spinner::with_size`, `Tag::success().child(..)`, `Alert::error(..).on_close(..)`,
  `cx.theme().background` (`ActiveTheme`), `IconName` variant names (no
  `Download`/`Refresh` — use confirmed variants or label-only), and `when_some`.
- **smol version.** Align the `smol` major with the one gpui already pulls in
  (check `cargo tree -p gpui | rg smol`) to avoid a duplicate-crate clippy/build
  issue. Use that exact major in `[workspace.dependencies]`.
- **No `#[allow]`.** Route errors to the inline banner/toast; no `unwrap`/`expect`.
  If a single glue line genuinely cannot satisfy clippy, surface it for review
  rather than relaxing the workspace lint config.



