//! Add-tool panel: browse the registry or add a plugin from a source.

use gpui::prelude::*;
use gpui::{Context, IntoElement, ParentElement, Styled, Window, div};
use gpui_component::button::Button;
use gpui_component::input::Input;

use crate::model::{AddSource, BackendChoice};
use crate::ui::RootView;
use crate::ui::root::AddTab;

impl RootView {
    pub(crate) fn render_add_panel(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl IntoElement {
        let tab = self.add_tab;
        div()
            .flex_1()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_lg().child("Add a tool"))
                    .child(
                        Button::new("add-close")
                            .label("Close")
                            .on_click(cx.listener(|this, _ev, _window, cx| {
                                this.show_add = false;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(Button::new("tab-registry").label("From registry").on_click(
                        cx.listener(|this, _ev, _window, cx| {
                            this.add_tab = AddTab::Registry;
                            cx.notify();
                        }),
                    ))
                    .child(
                        Button::new("tab-source")
                            .label("From source")
                            .on_click(cx.listener(|this, _ev, _window, cx| {
                                this.add_tab = AddTab::Source;
                                cx.notify();
                            })),
                    ),
            )
            .child(match tab {
                AddTab::Registry => self.render_add_registry(cx).into_any_element(),
                AddTab::Source => self.render_add_source(cx).into_any_element(),
            })
    }

    fn render_add_registry(&mut self, cx: &mut Context<'_, Self>) -> impl IntoElement + use<> {
        let query = self.add_filter_input.read(cx).value().to_lowercase();
        let names: Vec<String> = self
            .add_registry
            .iter()
            .filter(|n| query.is_empty() || n.to_lowercase().contains(&query))
            .cloned()
            .collect();
        // Build rows with a for-loop into a Vec (NOT `.children(map(...))`) so the
        // `cx.listener` calls don't double-borrow `cx` under Rust 2024 — see detail.rs.
        let mut rows = Vec::with_capacity(names.len());
        for (ix, name) in names.into_iter().enumerate() {
            let add_name = name.clone();
            rows.push(
                div()
                    .id(("registry", ix))
                    .flex()
                    .items_center()
                    .justify_between()
                    .py_1()
                    .child(div().child(name))
                    .child(
                        Button::new(("add-reg", ix))
                            .label("Add")
                            .on_click(cx.listener(move |this, _ev, window, cx| {
                                let n = add_name.clone();
                                this.show_add = false;
                                this.run_action(window, cx, move |svc| {
                                    svc.add(AddSource::Registry { name: n.clone() })
                                        .map(|()| format!("Added {n}"))
                                });
                            })),
                    ),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(Input::new(&self.add_filter_input))
            .children(rows)
    }

    fn render_add_source(&mut self, cx: &mut Context<'_, Self>) -> impl IntoElement + use<> {
        let backend = self.add_backend;
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(Input::new(&self.add_source_input))
            .child(Input::new(&self.add_alias_input))
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(div().child(format!("backend: {}", backend_label(backend))))
                    .child(
                        Button::new("backend-lua")
                            .label("Lua")
                            .on_click(cx.listener(|this, _ev, _window, cx| {
                                this.add_backend = BackendChoice::Lua;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("backend-wasi")
                            .label("Wasi")
                            .on_click(cx.listener(|this, _ev, _window, cx| {
                                this.add_backend = BackendChoice::Wasi;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                Button::new("add-source-submit")
                    .label("Add")
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
                        this.run_action(window, cx, move |svc| {
                            svc.add(AddSource::Source {
                                source: source.clone(),
                                alias,
                                backend,
                            })
                            .map(|()| format!("Added from {source}"))
                        });
                    })),
            )
    }
}

fn backend_label(backend: BackendChoice) -> &'static str {
    match backend {
        BackendChoice::Lua => "Lua",
        BackendChoice::Wasi => "Wasi",
    }
}
