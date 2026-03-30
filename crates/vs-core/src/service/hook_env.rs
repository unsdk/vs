//! Services for generating shell hook environment updates.

use vs_shell::ShellKind;

use crate::{App, CoreError};

impl App {
    /// Renders hidden shell hook output.
    pub fn hook_env(&self, shell: &str) -> Result<String, CoreError> {
        let shell = ShellKind::parse(shell)?;
        self.render_hook_env(shell)
    }
}
