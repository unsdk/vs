//! Root view: owns all UI state and the top-level window layout.

use std::collections::HashSet;

use gpui::prelude::*;
use gpui::{Context, Entity, IntoElement, ParentElement, SharedString, Styled, Window, div, px};
use gpui_component::WindowExt;
use gpui_component::alert::Alert;
use gpui_component::input::InputState;
use gpui_component::notification::{Notification, NotificationType};
use gpui_component::resizable::{ResizableState, h_resizable, resizable_panel};

use vs_core::CoreError;

use crate::model::{ScopeChoice, ToolRow, VersionRow};
use crate::service::AppService;

/// Which tab the Add-tool panel is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AddTab {
    Registry,
    Source,
}

/// Live state for an in-flight install download.
#[derive(Clone, Debug)]
pub(crate) struct InstallProgress {
    pub(crate) key: String,
    pub(crate) done: u64,
    pub(crate) total: Option<u64>,
}

/// Single source of truth for the GUI.
pub struct RootView {
    pub(crate) service: AppService,
    pub(crate) tools: Vec<ToolRow>,
    pub(crate) selected: Option<String>,
    pub(crate) installed: Vec<VersionRow>,
    pub(crate) available: Vec<VersionRow>,
    pub(crate) scope: ScopeChoice,
    pub(crate) filter_input: Entity<InputState>,
    pub(crate) search_input: Entity<InputState>,
    /// Action keys for in-flight operations (e.g. "install:nodejs:21.6.0").
    pub(crate) pending: HashSet<String>,
    /// Last error message, shown as a dismissible inline banner. None = hidden.
    pub(crate) last_error: Option<String>,
    pub(crate) show_add: bool,
    pub(crate) add_tab: AddTab,
    pub(crate) add_registry: Vec<String>,
    pub(crate) add_filter_input: Entity<InputState>,
    pub(crate) add_source_input: Entity<InputState>,
    pub(crate) add_alias_input: Entity<InputState>,
    pub(crate) add_backend: crate::model::BackendChoice,
    /// Persisted drag state for the sidebar/detail resizable split.
    pub(crate) resizable_state: Entity<ResizableState>,
    /// Whether the sidebar is collapsed to a thin strip.
    pub(crate) sidebar_collapsed: bool,
    /// Live state for the currently in-flight install download, if any.
    pub(crate) progress: Option<InstallProgress>,
}

impl RootView {
    pub fn new(service: AppService, window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let filter_input = cx.new(|cx| InputState::new(window, cx).placeholder("Filter tools…"));
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search versions…"));
        let add_filter_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search registry…"));
        let add_source_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("git URL or local path"));
        let add_alias_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("alias (optional)"));
        let resizable_state = cx.new(|_| ResizableState::default());
        let view = Self {
            service,
            tools: Vec::new(),
            selected: None,
            installed: Vec::new(),
            available: Vec::new(),
            scope: ScopeChoice::Project,
            filter_input,
            search_input,
            pending: HashSet::new(),
            last_error: None,
            show_add: false,
            add_tab: AddTab::Registry,
            add_registry: Vec::new(),
            add_filter_input,
            add_source_input,
            add_alias_input,
            add_backend: crate::model::BackendChoice::Lua,
            resizable_state,
            sidebar_collapsed: false,
            progress: None,
        };
        view.reload_tools(cx);
        view
    }

    /// Reload the sidebar tool list in the background.
    pub(crate) fn reload_tools(&self, cx: &mut Context<'_, Self>) {
        let service = self.service.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { service.tool_rows() })
                .await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(rows) => this.tools = rows,
                    Err(err) => this.deferred_error(err, cx),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Record an error message: log it and surface it as an inline banner.
    pub(crate) fn set_error(&mut self, message: String) {
        eprintln!("vs-app: {message}");
        self.last_error = Some(message);
    }

    /// Surface a `CoreError` from a load path as an inline banner.
    pub(crate) fn deferred_error(&mut self, err: CoreError, _cx: &mut Context<'_, Self>) {
        self.set_error(format!("{err}"));
    }

    /// Select a tool and load its versions.
    pub(crate) fn select_tool(&mut self, name: String, cx: &mut Context<'_, Self>) {
        self.selected = Some(name);
        self.installed.clear();
        self.available.clear();
        self.reload_detail(cx);
    }

    /// Reload installed + available versions for the selected tool.
    pub(crate) fn reload_detail(&self, cx: &mut Context<'_, Self>) {
        let Some(name) = self.selected.clone() else {
            return;
        };
        let svc_installed = self.service.clone();
        let svc_available = self.service.clone();
        let name_installed = name.clone();
        let name_available = name;
        cx.spawn(async move |this, cx| {
            let installed = cx
                .background_spawn(async move { svc_installed.installed_rows(&name_installed) })
                .await;
            let available = cx
                .background_spawn(
                    async move { svc_available.search_available(&name_available, "") },
                )
                .await;
            this.update(cx, |this, cx| {
                match installed {
                    Ok(rows) => this.installed = rows,
                    Err(err) => this.deferred_error(err, cx),
                }
                match available {
                    Ok(rows) => this.available = rows,
                    Err(err) => this.deferred_error(err, cx),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Open the Add-tool panel and load the registry plugin list.
    pub(crate) fn open_add_tool(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        self.show_add = true;
        self.add_tab = AddTab::Registry;
        self.load_registry(cx);
        cx.notify();
    }

    /// Load installable plugin names from the registry in the background.
    pub(crate) fn load_registry(&self, cx: &mut Context<'_, Self>) {
        let service = self.service.clone();
        cx.spawn(async move |this, cx| {
            let names = cx
                .background_spawn(async move { service.registry_plugin_names() })
                .await;
            this.update(cx, |this, cx| {
                match names {
                    Ok(names) => this.add_registry = names,
                    Err(err) => this.deferred_error(err, cx),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

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

    /// Re-run the Available search using the current search-box text.
    pub(crate) fn run_search(&self, cx: &mut Context<'_, Self>) {
        let Some(name) = self.selected.clone() else {
            return;
        };
        let query = self.search_input.read(cx).value().to_string();
        let service = self.service.clone();
        cx.spawn(async move |this, cx| {
            let rows = cx
                .background_spawn(async move { service.search_available(&name, &query) })
                .await;
            this.update(cx, |this, cx| {
                match rows {
                    Ok(rows) => this.available = rows,
                    Err(err) => this.deferred_error(err, cx),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Cycle Project → Global → Session → Project.
    pub(crate) fn next_scope(&mut self, cx: &mut Context<'_, Self>) {
        self.scope = match self.scope {
            ScopeChoice::Project => ScopeChoice::Global,
            ScopeChoice::Global => ScopeChoice::Session,
            ScopeChoice::Session => ScopeChoice::Project,
        };
        cx.notify();
    }
}

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
            .when(self.show_add, |el| {
                el.child(self.render_add_modal(window, cx))
            })
    }
}

impl RootView {
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

    /// Title bar: app name, registry refresh, and the scope selector.
    fn render_title_bar(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl IntoElement {
        use gpui_component::button::Button;
        let scope_label = format!("scope: {}", self.scope.label());
        div()
            .h(px(40.0))
            .px_3()
            .flex()
            .items_center()
            .justify_between()
            .child(div().child("vs"))
            .child(
                div()
                    .flex()
                    .gap_1()
                    .items_center()
                    .when(!self.pending.is_empty(), |el| {
                        el.child(div().child("working…"))
                    })
                    .child(
                        Button::new("refresh-registry")
                            .label("Refresh registry")
                            .on_click(cx.listener(|this, _ev, window, cx| {
                                this.run_action("refresh".to_string(), window, cx, |svc| {
                                    svc.refresh_registry()
                                        .map(|n| format!("Registry refreshed: {n} plugins"))
                                });
                            })),
                    )
                    .child(
                        Button::new("scope")
                            .label(scope_label)
                            .on_click(cx.listener(|this, _ev, _window, cx| this.next_scope(cx))),
                    ),
            )
    }
}
