//! Desktop GUI for the `vs` runtime version manager, built on gpui + gpui-component.

mod model;
mod service;
mod ui;

use anyhow::Result;

/// Launch the GUI application. Wired up fully in later tasks.
pub fn run() -> Result<()> {
    // Replaced in Task 4 with the gpui Application bootstrap.
    Ok(())
}
