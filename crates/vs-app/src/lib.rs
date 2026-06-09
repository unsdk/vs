//! Desktop GUI for the `vs` runtime version manager, built on gpui + gpui-component.

mod model;
mod service;
mod ui;

pub use model::{
    AddSource, BackendChoice, ScopeChoice, ToolRow, VersionRow,
    available_rows, filter_tool_rows, installed_rows, merge_tool_rows,
};
pub use service::AppService;

use anyhow::Result;
use gpui::{AppContext as _, Application, WindowOptions};
use gpui_component::Root;

use crate::ui::RootView;

/// Launch the GUI application.
pub fn run() -> Result<()> {
    let core = vs_core::App::from_env()?;
    let service = AppService::new(core);

    Application::new().run(move |cx| {
        gpui_component::init(cx);
        let opened = cx.open_window(WindowOptions::default(), |window, cx| {
            let view = cx.new(|cx| RootView::new(service.clone(), window, cx));
            cx.new(|cx| Root::new(view, window, cx))
        });
        if let Err(err) = opened {
            eprintln!("vs-app: failed to open window: {err}");
            cx.quit();
        }
    });

    Ok(())
}
