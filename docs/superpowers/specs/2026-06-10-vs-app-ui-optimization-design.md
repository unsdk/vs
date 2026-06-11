# vs-app UI optimization: progress, scrolling, layout, interactions, failure feedback

Date: 2026-06-10
Status: Approved design (pending implementation plan)

## Summary

Refine the `vs-app` GUI (gpui + gpui-component desktop frontend for the `vs`
version manager) along five axes the user requested:

1. **Real progress bar** for install/download operations.
2. **Scrollable regions** for the tool list and version sections.
3. **Layout redesign** — resizable two-pane, collapsible sidebar, modal Add-tool.
4. **Interaction polish** — per-action loading, icons/variants, status tags, empty states.
5. **Failure feedback** — inline dismissible `Alert` banner in addition to toasts.

Most work lives in the `vs-app` UI layer. The progress bar is the only feature
that crosses crate boundaries: it threads a **backward-compatible optional
progress callback** through the existing (already byte-aware) download loop in
`vs-installer` and `vs-core`. `vs-cli` is unaffected (passes `None`).

## Goals

- Determinate progress (real %) for version installs, surfaced live in the GUI.
- No off-window overflow: long lists scroll within their pane.
- A layout that is comfortable to use: draggable split, collapsible sidebar,
  and an Add-tool flow that does not hijack the whole window.
- Clear per-action feedback (loading, success, failure) that never panics.
- Keep the workspace's strict lint gate green; no `unwrap`/`expect`; no `#[allow]`.

## Non-goals (deferred)

- Determinate progress for `add_plugin` / plugin-source archive downloads — these
  show an indeterminate button spinner in v1 (the determinate bar is scoped to
  version installs, the heavy download path).
- Using gpui-component's `Dialog` widget for the Add-tool modal — we render a
  custom overlay (backdrop + centered card) to avoid depending on `Dialog`'s
  body API. Adopting `Dialog` is optional later polish.
- Cancelling an in-flight install (no cancel token in `vs-core`).
- Virtualized lists (`List`/`Table` delegates) — plain scrollable columns are
  sufficient at expected list sizes; revisit if lists grow large.

## Confirmed API facts (from vendored sources)

gpui 0.2.2 / gpui-component 0.5.1 (verified during brainstorming):

- **Progress:** `gpui_component::progress::Progress::new().value(f32 /*0-100*/).bg(color)`.
- **Spinner:** `gpui_component::spinner::Spinner::new().with_size(...).color(...)`.
- **Scroll:** gpui core `Div::overflow_y_scroll()` / `.track_scroll(&ScrollHandle)`;
  gpui-component `ScrollableElement::overflow_y_scrollbar()` (styled scrollbar).
- **Resizable:** `gpui_component::resizable::{h_resizable(id), resizable_panel()}`;
  `ResizablePanel::body(..).min_size(px).max_size(px)`; group tracks a
  `ResizableState`.
- **Tab/Divider/Tag/Alert/Badge:** all present (`Tag::success()`, `Alert::error(id, msg)`
  with `.title()`, `.on_close(..)`, `.visible(..)`).
- **Button:** `.loading(bool)`, `.disabled(bool)`, `.icon(impl Into<Icon>)`,
  `.primary()/.danger()/.ghost()`, `.tooltip(..)`. `IconName` includes
  `Download`, `Plus`, `Delete`, `Search`, `Check`, etc.
- **Notification:** already used; `NotificationType::{Info,Success,Warning,Error}`.

vs install path (verified):

- `vs-installer::install::Installer::download_bytes(url, headers)` already reads
  `Content-Length` (`response.content_length()`) and loops over 8 KB chunks,
  calling `progress_bar.inc(read)` on an indicatif bar. The byte-level data for a
  determinate bar already exists; it is just hardwired to the console.
- `App::install_plugin_version(plugin_name, version)` is fully blocking with no
  progress hook today.

## Feature 1 — Real progress bar (cross-crate)

### Callback type

In `vs-core` (e.g. `lib.rs` or a small `progress.rs`), define:

```rust
/// Reports download progress: (bytes_downloaded, total_bytes_if_known).
pub type ProgressFn<'a> = dyn Fn(u64, Option<u64>) + 'a;
```

Public methods take `progress: Option<&ProgressFn<'_>>`. The callback is invoked
synchronously on whatever thread runs the install; it carries no `Send` bound
itself (the GUI's *future* is `Send` because the channel sender it captures is
`Send`, not because the callback crosses threads).

### vs-installer changes

- `download_bytes(&self, url, headers, progress: Option<&ProgressFn<'_>>)`:
  - When `progress` is `Some`: do **not** create the indicatif bar; after each
    `read`, call `progress(downloaded_so_far, total_size)`.
  - When `progress` is `None`: keep the current indicatif bar verbatim.
- `materialize_artifact(.., progress: Option<&ProgressFn<'_>>)`: forward to
  `download_bytes` on the `InstallSource::Url` path.
- `Installer::install(&self, plan, progress: Option<&ProgressFn<'_>>)`: forward
  to each `materialize_artifact`.

### vs-core changes

- `install_plugin_version(&self, plugin_name, version, progress: Option<&ProgressFn<'_>>)`
  passes `progress` into `self.installer.install(&plan, progress)`.
- Update all in-crate callers to pass `None` (or thread through where natural).

### vs-cli changes

- `crates/vs-cli/src/install.rs`: pass `None` to `install_plugin_version`. CLI
  output is unchanged (indicatif still drives the console bar inside the
  installer).

### vs-app changes

- `AppService` gains:

```rust
pub fn install_with_progress(
    &self,
    name: &str,
    version: &str,
    on_progress: &ProgressFn<'_>,
) -> Result<InstalledVersion, CoreError> {
    self.core.install_plugin_version(name, Some(version), Some(on_progress))
}
```

  (The existing `install` keeps working by calling with `None`; either keep it
  for non-progress callers or route everything through the new method.)

- UI flow (in `root.rs`), replacing the install branch of `run_action`:

```rust
// pseudocode shape — exact gpui API verified at implementation time
let (tx, rx) = smol::channel::unbounded::<(u64, Option<u64>)>();
let key = format!("install:{name}:{version}");
self.pending.insert(key.clone());
self.progress = Some(Progress { key: key.clone(), done: 0, total: None });

// foreground drain loop
cx.spawn(async move |this, cx| {
    while let Ok((done, total)) = rx.recv().await {
        this.update(cx, |this, cx| {
            if let Some(p) = this.progress.as_mut() { p.done = done; p.total = total; }
            cx.notify();
        }).ok();
    }
}).detach();

// background install
cx.spawn_in(window, async move |this, cx| {
    let svc = service.clone();
    let result = cx.background_spawn(async move {
        svc.install_with_progress(&n, &v, &|done, total| { let _ = tx.try_send((done, total)); })
    }).await;
    this.update_in(cx, |this, window, cx| {
        this.pending.remove(&key);
        this.progress = None;
        match result {
            Ok(iv) => window.push_notification(success_toast(iv), cx),
            Err(err) => { this.set_error(format!("{err}")); window.push_notification(error_toast(&err), cx); }
        }
        this.reload_detail(cx);
        this.reload_tools(cx);
        cx.notify();
    }).ok();
}).detach();
```

  The exact channel type (`smol::channel`, `futures::channel::mpsc`, or gpui's
  re-export) is finalized against the vendored gpui async API during
  implementation; the shape (sync `try_send` from the blocking callback, async
  `recv` on the foreground) is the contract.

- Render: when `self.progress` is `Some` for the row/section being installed,
  show a `Progress::new().value(pct)` where
  `pct = total.map(|t| done as f32 / t as f32 * 100.0).unwrap_or(indeterminate)`.
  When `total` is `None`, show a `Spinner` instead of a 0% bar.

## Feature 2 — Scrollable regions

- Sidebar tool list: wrap the list column in `.overflow_y_scrollbar()` so it
  scrolls independently of the filter/Add header.
- Detail INSTALLED and AVAILABLE sections: the version rows live in a scrollable
  container (`.overflow_y_scrollbar()`), with the tool header and Available
  search box pinned above it.
- The Add-tool modal's registry list also scrolls within the card.

`ScrollHandle`/scrollbar state entities, if required by the chosen API, are held
on `RootView`. Prefer the gpui-component styled scrollbar; fall back to gpui
core `overflow_y_scroll()` if the styled wrapper proves awkward.

## Feature 3 — Layout redesign

### Two-pane resizable split

- Use `h_resizable("vs-main")` with two `resizable_panel()`s:
  - Left (sidebar): `min_size(px(160.0))`, default ~`px(220.0)`.
  - Right (detail): fills remaining space.
- A `ResizableState` entity is created in `RootView::new` and stored on the
  struct so drag positions persist across renders.

### Collapsible sidebar

- `sidebar_collapsed: bool` on `RootView`.
- A toggle button (chevron) in the sidebar header. When collapsed, the left
  panel renders a thin strip (just an expand chevron + maybe tool count badge);
  expanding restores the full sidebar. Collapse is independent of the drag width
  (collapsing overrides the rendered body; expanding restores the prior width).

### Modal Add-tool overlay

- `show_add` no longer swaps the body. Instead, when `show_add` is true, the
  top-level `render` stacks a **modal overlay** above the two-pane content:
  - A full-window absolutely-positioned semi-transparent backdrop
    (`div().absolute().inset_0().bg(rgba(0x00000088))`) that closes the modal on
    click (with `.occlude()` so clicks don't pass through).
  - A centered card (`div` with bg, rounded, shadow, max width/height) holding
    the existing Add-tool content (tabs, registry list with scroll, source form).
- The two-pane content remains mounted underneath (state preserved).

## Feature 4 — Interaction polish

- **Per-action loading.** Replace the single global `busy` counter's role for
  buttons with a `pending: HashSet<String>` keyed by action
  (`"install:<name>:<ver>"`, `"use:<name>:<ver>"`, `"uninstall:<name>:<ver>"`,
  `"update:<name>"`, `"remove:<name>"`, `"refresh"`). A button whose key is in
  `pending` renders `.loading(true).disabled(true)`. `run_action` inserts the key
  before spawning and removes it on completion. The title-bar busy hint can key
  off `!pending.is_empty()`.
- **Icons + variants.** `Install` → `.primary().icon(IconName::Download)`;
  `Uninstall`/`Remove plugin` → `.danger().icon(IconName::Delete)`;
  `Refresh registry` → `.icon(IconName::Refresh or Redo)`; `＋ Add` →
  `.icon(IconName::Plus)`. Icon-only buttons get `.tooltip(..)`.
- **Status tag.** The current version shows a `Tag::success().child("current")`
  instead of the inline "✓ current" text.
- **Empty states.** No added tools → centered prompt in the sidebar ("No tools
  yet — click ＋ Add"). Selected tool with no search performed → hint in
  Available ("Search to find installable versions"). Empty search results →
  "No versions match".

## Feature 5 — Failure feedback

- `last_error: Option<String>` on `RootView`.
- A helper `set_error(&mut self, msg: String)` stores the message (and is called
  from both action-failure paths and the load paths that currently `eprintln`
  via `deferred_error`).
- Render a dismissible inline banner under the title bar when `last_error` is
  `Some`: `Alert::error("vs-error", msg).on_close(|this, _, cx| { this.last_error = None; cx.notify(); })`.
- Toasts are retained for transient success/failure; the inline `Alert` persists
  until dismissed or replaced, so failures are not missed if a toast auto-hides.
- `deferred_error` is updated to call `set_error` (and may keep the `eprintln`
  for logging). No operation panics; all `CoreError`s format into the banner/toast.

## RootView state additions (summary)

```rust
// new fields on RootView
resizable_state: Entity<ResizableState>,   // or the verified state handle type
sidebar_collapsed: bool,
pending: std::collections::HashSet<String>, // in-flight action keys
progress: Option<InstallProgress>,          // { key: String, done: u64, total: Option<u64> }
last_error: Option<String>,
// (existing: service, tools, selected, installed, available, scope, *_input, show_add, add_*)
// `busy: u32` may be removed in favor of `pending`, or retained for the title-bar hint.
```

`InstallProgress` is a small local struct in `root.rs` (or `model.rs` if it
needs unit tests — the percentage computation `done/total → 0..=100` is a good
candidate for a pure tested helper).

## Files touched

| File | Change |
| --- | --- |
| `crates/vs-installer/src/install.rs` | progress callback param on `download_bytes`, `materialize_artifact`, `install` |
| `crates/vs-core/src/lib.rs` (or `progress.rs`) | `ProgressFn` type alias, re-export |
| `crates/vs-core/src/service/install.rs` | `install_plugin_version` gains progress param |
| `crates/vs-cli/src/install.rs` | pass `None` |
| `crates/vs-app/src/service.rs` | `install_with_progress` |
| `crates/vs-app/src/model.rs` | pure `progress_percent` helper + test (optional) |
| `crates/vs-app/src/ui/root.rs` | resizable layout, modal overlay host, inline alert, progress drain, pending set, last_error |
| `crates/vs-app/src/ui/sidebar.rs` | scroll, collapse toggle, empty state, icons, pending loading |
| `crates/vs-app/src/ui/detail.rs` | scroll, progress bar, tags, icons, danger variants, pending loading |
| `crates/vs-app/src/ui/add_tool.rs` | render inside modal card, scroll registry list |
| `crates/vs-app/src/ui/modal.rs` (new, optional) | reusable overlay wrapper |

## Testing strategy

- **Unit tests (headless, in `model.rs`):** the `progress_percent(done, total)`
  pure helper (e.g. `(50, Some(200)) → 25.0`, `(x, None) → indeterminate sentinel`).
- **`vs-installer`:** the progress callback is exercised by existing install
  tests if any download in tests; otherwise a focused test that the callback
  fires with monotonic `downloaded` values (using a local file/mock server) is
  desirable but optional — the core change is mechanical signature plumbing.
- **Compile + lint gate:** `cargo build --workspace`, `cargo clippy --all-targets
  --all-features --locked -- -D warnings`, `cargo test --workspace`,
  `cargo fmt --all --check` all green.
- **CLI regression:** `cargo run -p vs -- --help` and an install still show the
  indicatif bar (callback `None` path unchanged).
- **Manual GUI acceptance:** resize/collapse the sidebar; scroll long lists;
  install a version and watch the live % bar; trigger a failure (e.g. bad
  version) and confirm the inline `Alert` + toast; open/close the Add modal.

## Risks & mitigations

- **Cross-crate signature churn:** mitigated by making the callback `Option<&..>`
  and defaulting all non-GUI callers to `None` — no behavior change off the GUI.
- **Indicatif vs callback double-output:** mitigated by skipping the indicatif
  bar entirely when a callback is supplied.
- **gpui async channel API drift:** the (sync `try_send` → async `recv`) contract
  is fixed; the concrete channel type is verified against vendored gpui at
  implementation time.
- **Resizable + collapse interaction:** collapse overrides the rendered body
  rather than fighting the resizable sizing model, avoiding state conflicts.
- **Strict lints on new gpui glue:** route errors to the banner/toast; no
  `unwrap`/`expect`; verify each new widget builder against vendored source.

## Open items for implementation

- Confirm the exact `ResizableState` construction and the `resizable_panel` body
  API against the vendored gpui-component source.
- Confirm the styled scrollbar wrapper (`overflow_y_scrollbar`) vs core
  `overflow_y_scroll` + `track_scroll`, and whether a `ScrollHandle` must be
  stored per scroll region.
- Confirm the channel type available in the gpui async context for the progress
  drain.
- Confirm `IconName` variant names (`Refresh`/`Redo`, `Delete`/`Trash`) against
  the vendored icon set.
