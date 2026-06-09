//! Detail pane render + version actions for the selected tool.

use gpui::prelude::*;
use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, Window, div};
use gpui_component::Disableable;
use gpui_component::Sizable;
use gpui_component::button::Button;
use gpui_component::button::ButtonVariants;
use gpui_component::input::Input;
use gpui_component::progress::Progress;
use gpui_component::scroll::ScrollableElement;
use gpui_component::spinner::Spinner;
use gpui_component::tag::Tag;
use gpui_component::{Icon, IconName, Size};

use crate::model::{ScopeChoice, VersionRow, progress_percent};
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
        let mut available_rows: Vec<AnyElement> = Vec::with_capacity(available.len());
        for (ix, row) in available.into_iter().enumerate() {
            available_rows.push(self.render_available_row(ix, &name, row, cx).into_any_element());
        }
        if available_rows.is_empty() {
            available_rows.push(
                div()
                    .py_1()
                    .text_xs()
                    .child("No versions yet — type a query and press Search.")
                    .into_any_element(),
            );
        }

        let update_loading = self.pending.contains(&format!("update:{name}"));
        let remove_loading = self.pending.contains(&format!("remove:{name}"));

        div()
            .flex_1()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .child({
                let update_name = name.clone();
                let remove_name = name.clone();
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_lg().child(name.clone()))
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .child(
                                Button::new("update-plugin")
                                    .label("Update plugin")
                                    .loading(update_loading)
                                    .disabled(update_loading)
                                    .on_click(cx.listener(move |this, _ev, window, cx| {
                                        let n = update_name.clone();
                                        let key = format!("update:{n}");
                                        this.run_action(key, window, cx, move |svc| {
                                            svc.update_plugin(&n).map(|()| format!("Updated {n}"))
                                        });
                                    })),
                            )
                            .child(
                                Button::new("remove-plugin")
                                    .label("Remove plugin")
                                    .danger()
                                    .icon(Icon::new(IconName::Delete))
                                    .loading(remove_loading)
                                    .disabled(remove_loading)
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
                            ),
                    )
            })
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
    }

    fn render_installed_row(
        &self,
        ix: usize,
        name: &str,
        row: VersionRow,
        scope: ScopeChoice,
        cx: &mut Context<'_, Self>,
    ) -> impl IntoElement + use<> {
        let version_text = row.version.clone();
        let is_current = row.current;
        let use_name = name.to_string();
        let use_version = row.version.clone();
        let uninstall_name = name.to_string();
        let uninstall_version = row.version;
        let use_key = format!("use:{name}:{use_version}");
        let uninstall_key = format!("uninstall:{name}:{uninstall_version}");
        let use_loading = self.pending.contains(&use_key);
        let uninstall_loading = self.pending.contains(&uninstall_key);
        div()
            .id(("installed", ix))
            .flex()
            .items_center()
            .justify_between()
            .py_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().child(version_text))
                    .when(is_current, |el| {
                        el.child(Tag::success().with_size(Size::Small).child("current"))
                    }),
            )
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(
                        Button::new(("use", ix))
                            .label("Use")
                            .loading(use_loading)
                            .disabled(use_loading)
                            .on_click(cx.listener(move |this, _ev, window, cx| {
                                let (n, v) = (use_name.clone(), use_version.clone());
                                let key = format!("use:{n}:{v}");
                                this.run_action(key, window, cx, move |svc| {
                                    svc.use_version(&n, &v, scope).map(|iv| {
                                        format!("Now using {}@{}", iv.plugin, iv.version)
                                    })
                                });
                            })),
                    )
                    .child(
                        Button::new(("uninstall", ix))
                            .label("Uninstall")
                            .danger()
                            .icon(Icon::new(IconName::Delete))
                            .loading(uninstall_loading)
                            .disabled(uninstall_loading)
                            .on_click(cx.listener(move |this, _ev, window, cx| {
                                let (n, v) =
                                    (uninstall_name.clone(), uninstall_version.clone());
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
                            })),
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
        let progress_key =
            format!("install:{name}:{version}", name = name, version = install_version);
        let install_loading = self.pending.contains(&progress_key);
        let row_progress = self
            .progress
            .as_ref()
            .filter(|p| p.key == progress_key)
            .map(|p| progress_percent(p.done, p.total));
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
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
                            .primary()
                            .loading(install_loading)
                            .disabled(already || install_loading)
                            .on_click(cx.listener(move |this, _ev, window, cx| {
                                this.run_install(
                                    install_name.clone(),
                                    install_version.clone(),
                                    window,
                                    cx,
                                );
                            })),
                    ),
            )
            .when_some(row_progress, |el, pct| match pct {
                Some(value) => el.child(div().w_full().child(Progress::new().value(value))),
                None => el.child(Spinner::new().with_size(Size::Small)),
            })
    }
}
