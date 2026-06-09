//! Root view: owns all UI state and the top-level window layout.

use gpui::prelude::*;
use gpui::{div, px, Context, Entity, IntoElement, ParentElement, Styled, Window};
use gpui_component::input::{Input, InputState};

use vs_core::CoreError;

use crate::model::{ScopeChoice, ToolRow, VersionRow};
use crate::service::AppService;

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
}

impl RootView {
    pub fn new(service: AppService, window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let filter_input = cx.new(|cx| InputState::new(window, cx).placeholder("Filter tools…"));
        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search versions…"));
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
}

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(self.render_title_bar(window, cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .child(self.render_sidebar(window, cx))
                    .child(self.render_detail(window, cx)),
            )
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

    fn render_sidebar(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<'_, Self>,
    ) -> impl IntoElement {
        div()
            .w(px(200.0))
            .p_2()
            .flex()
            .flex_col()
            .child(Input::new(&self.filter_input))
            .child("TOOLS")
            .children(self.tools.iter().map(|t| {
                let label = match &t.current_version {
                    Some(v) => format!("{} ({})", t.name, v),
                    None => t.name.clone(),
                };
                div().py_1().child(label)
            }))
    }

    fn render_detail(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<'_, Self>,
    ) -> impl IntoElement {
        div()
            .flex_1()
            .p_2()
            .flex()
            .flex_col()
            .child(match &self.selected {
                Some(name) => format!("Selected: {name}"),
                None => "Select a tool".to_string(),
            })
            .child(Input::new(&self.search_input))
            .child(format!(
                "installed: {} · available: {}",
                self.installed.len(),
                self.available.len()
            ))
    }
}
