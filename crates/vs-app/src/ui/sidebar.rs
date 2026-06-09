//! Sidebar render + interactions (filter, tool selection, Add button).

use gpui::prelude::*;
use gpui::{Context, IntoElement, ParentElement, Styled, Window, div, px};
use gpui_component::button::Button;
use gpui_component::input::Input;

use crate::model::filter_tool_rows;
use crate::ui::RootView;

impl RootView {
    pub(crate) fn render_sidebar(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl IntoElement {
        let query = self.filter_input.read(cx).value().to_string();
        let visible = filter_tool_rows(&self.tools, &query);
        let selected = self.selected.clone();

        div()
            .w(px(210.0))
            .flex()
            .flex_col()
            .gap_2()
            .p_2()
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(div().flex_1().child(Input::new(&self.filter_input)))
                    .child(
                        Button::new("add-tool")
                            .label("＋ Add")
                            .on_click(cx.listener(|this, _ev, window, cx| {
                                this.open_add_tool(window, cx);
                            })),
                    ),
            )
            .child(div().text_xs().child("TOOLS"))
            .children(visible.into_iter().enumerate().map(|(ix, tool)| {
                let name = tool.name.clone();
                let is_selected = selected.as_deref() == Some(name.as_str());
                let label = match &tool.current_version {
                    Some(v) => format!("{}  ·  {}", tool.name, v),
                    None => format!("{}  ·  —", tool.name),
                };
                div()
                    .id(("tool-row", ix))
                    .py_1()
                    .px_2()
                    .rounded_md()
                    .cursor_pointer()
                    .when(is_selected, |el| el.bg(gpui::rgba(0x6678_ff33)))
                    .child(label)
                    .on_click(cx.listener(move |this, _ev, _window, cx| {
                        this.select_tool(name.clone(), cx);
                    }))
            }))
    }
}
