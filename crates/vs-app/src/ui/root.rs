//! Root view: owns all UI state and the top-level window layout.

use gpui::prelude::*;
use gpui::{div, px, Context, Entity, IntoElement, ParentElement, SharedString, Styled, Window};
use gpui_component::input::InputState;
use gpui_component::notification::{Notification, NotificationType};
use gpui_component::WindowExt;

use vs_core::CoreError;

use crate::model::{ScopeChoice, ToolRow, VersionRow};
use crate::service::AppService;

/// Which tab the Add-tool panel is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AddTab {
    Registry,
    Source,
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
    /// Number of in-flight background operations; used to show a busy hint.
    pub(crate) busy: u32,
    pub(crate) show_add: bool,
    pub(crate) add_tab: AddTab,
    pub(crate) add_registry: Vec<String>,
    pub(crate) add_filter_input: Entity<InputState>,
    pub(crate) add_source_input: Entity<InputState>,
    pub(crate) add_alias_input: Entity<InputState>,
    pub(crate) add_backend: crate::model::BackendChoice,
}

impl RootView {
    pub fn new(service: AppService, window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let filter_input = cx.new(|cx| InputState::new(window, cx).placeholder("Filter tools…"));
        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search versions…"));
        let add_filter_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search registry…"));
        let add_source_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("git URL or local path"));
        let add_alias_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("alias (optional)"));
        let view = Self {
            service,
            tools: Vec::new(),
            selected: None,
            installed: Vec::new(),
            available: Vec::new(),
            scope: ScopeChoice::Project,
            filter_input,
            search_input,
            busy: 0,
            show_add: false,
            add_tab: AddTab::Registry,
            add_registry: Vec::new(),
            add_filter_input,
            add_source_input,
            add_alias_input,
            add_backend: crate::model::BackendChoice::Lua,
        };
        view.reload_tools(cx);
        view
    }

    /// Reload the sidebar tool list in the background.
    pub(crate) fn reload_tools(&self, cx: &mut Context<'_, Self>) {
        let service = self.service.clone();
        cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move { service.tool_rows() }).await;
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

    /// Stash an error to surface as a toast. (`report_error` needs a `Window`;
    /// when we only have `Context`, log to stderr as a fallback for v1.)
    pub(crate) fn deferred_error(&self, err: CoreError, _cx: &mut Context<'_, Self>) {
        eprintln!("vs-app: {err}");
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
                .background_spawn(async move { svc_available.search_available(&name_available, "") })
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

    /// Run a blocking `service` operation off the UI thread, toast the outcome,
    /// then refresh the detail + sidebar lists. `op` returns the success message.
    pub(crate) fn run_action<F>(&mut self, window: &mut Window, cx: &mut Context<'_, Self>, op: F)
    where
        F: FnOnce(AppService) -> Result<String, CoreError> + Send + 'static,
    {
        let service = self.service.clone();
        self.busy += 1;
        cx.spawn_in(window, async move |this, cx| {
            let result = cx.background_spawn(async move { op(service) }).await;
            this.update_in(cx, |this, window, cx| {
                this.busy = this.busy.saturating_sub(1);
                match result {
                    Ok(msg) => window.push_notification(
                        Notification::from((NotificationType::Success, SharedString::from(msg))),
                        cx,
                    ),
                    Err(err) => window.push_notification(
                        Notification::from((
                            NotificationType::Error,
                            SharedString::from(format!("{err}")),
                        )),
                        cx,
                    ),
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
}

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let body = if self.show_add {
            self.render_add_panel(window, cx).into_any_element()
        } else {
            div()
                .flex()
                .flex_1()
                .child(self.render_sidebar(window, cx))
                .child(self.render_detail(window, cx))
                .into_any_element()
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(self.render_title_bar(window, cx))
            .child(body)
    }
}

impl RootView {
    /// Title bar: app name, registry refresh, and the scope selector.
    fn render_title_bar(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<'_, Self>,
    ) -> impl IntoElement {
        div()
            .h(px(40.0))
            .px_3()
            .flex()
            .items_center()
            .justify_between()
            .child(div().child("vs"))
            .child(div().child(if self.busy > 0 {
                format!("scope: {} · working…", self.scope.label())
            } else {
                format!("scope: {}", self.scope.label())
            }))
    }
}
