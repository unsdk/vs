use crate::{App, CoreError};

impl App {
    /// Prints the active runtime directory for a tool.
    pub fn cd_path(&self, plugin_name: &str) -> Result<String, CoreError> {
        let current = self
            .current_tool(plugin_name)?
            .ok_or_else(|| CoreError::InactiveTool(plugin_name.to_string()))?;
        Ok(self.effective_runtime_dir(&current).display().to_string())
    }
}
