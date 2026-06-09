//! Detail pane render + version actions for the selected tool.

use gpui::prelude::*;
use gpui::{div, Context, IntoElement, ParentElement, Styled, Window};
use gpui_component::button::Button;
use gpui_component::input::Input;
use gpui_component::Disableable;

use crate::model::{ScopeChoice, VersionRow};
use crate::ui::RootView;

impl RootView {
    pub(crate) fn render_detail(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl IntoElement {
        let Some(name) = self.selected.clone() else {
            return div()
                .flex_1()
                .p_4()
                .child("Select a tool on the left, or ＋ Add a new one.");
        };

        let installed = self.installed.clone();
        let available = self.available.clone();
        let scope = self.scope;

        let mut installed_rows = Vec::with_capacity(installed.len());
        for (ix, row) in installed.into_iter().enumerate() {
            installed_rows.push(self.render_installed_row(ix, &name, row, scope, cx));
        }
        let mut available_rows = Vec::with_capacity(available.len());
        for (ix, row) in available.into_iter().enumerate() {
            available_rows.push(self.render_available_row(ix, &name, row, cx));
        }

        div()
            .flex_1()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .child(div().text_lg().child(name.clone()))
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
            .children(available_rows)
    }

    fn render_installed_row(
        &self,
        ix: usize,
        name: &str,
        row: VersionRow,
        scope: ScopeChoice,
        cx: &mut Context<'_, Self>,
    ) -> impl IntoElement + use<> {
        let label = if row.current {
            format!("{}  ✓ current", row.version)
        } else {
            row.version.clone()
        };
        let use_name = name.to_string();
        let use_version = row.version.clone();
        let uninstall_name = name.to_string();
        let uninstall_version = row.version;
        div()
            .id(("installed", ix))
            .flex()
            .items_center()
            .justify_between()
            .py_1()
            .child(div().child(label))
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(Button::new(("use", ix)).label("Use").on_click(cx.listener(
                        move |this, _ev, window, cx| {
                            let (n, v) = (use_name.clone(), use_version.clone());
                            this.run_action(window, cx, move |svc| {
                                svc.use_version(&n, &v, scope)
                                    .map(|iv| format!("Now using {}@{}", iv.plugin, iv.version))
                            });
                        },
                    )))
                    .child(
                        Button::new(("uninstall", ix)).label("Uninstall").on_click(cx.listener(
                            move |this, _ev, window, cx| {
                                let (n, v) =
                                    (uninstall_name.clone(), uninstall_version.clone());
                                this.run_action(window, cx, move |svc| {
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
                        )),
                    ),
            )
    }

    fn render_available_row(
        &self,
        ix: usize,
        name: &str,
        row: VersionRow,
        cx: &mut Context<'_, Self>,
    ) -> impl IntoElement + use<> {
        let install_name = name.to_string();
        let already = row.installed;
        let display_version = row.version.clone();
        let install_version = row.version;
        div()
            .id(("available", ix))
            .flex()
            .items_center()
            .justify_between()
            .py_1()
            .child(div().child(display_version))
            .child(
                Button::new(("install", ix))
                    .label(if already { "Installed" } else { "Install" })
                    .disabled(already)
                    .on_click(cx.listener(move |this, _ev, window, cx| {
                        let (n, v) = (install_name.clone(), install_version.clone());
                        this.run_action(window, cx, move |svc| {
                            svc.install(&n, &v)
                                .map(|iv| format!("Installed {}@{}", iv.plugin, iv.version))
                        });
                    })),
            )
    }
}
