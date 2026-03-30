//! Shell activation service entry points.

use vs_shell::{ShellKind, render_activation};

use crate::{App, CoreError};

impl App {
    /// Renders an activation script for the selected shell.
    pub fn activate(&self, shell: &str) -> Result<String, CoreError> {
        let shell = ShellKind::parse(shell)?;
        Ok(render_activation(shell))
    }
}
