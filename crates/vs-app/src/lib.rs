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

/// Launch the GUI application. Wired up fully in later tasks.
pub fn run() -> Result<()> {
    // Replaced in Task 4 with the gpui Application bootstrap.
    Ok(())
}
