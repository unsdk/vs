//! Services for resolving directories used by `vs cd`.

use crate::{App, CoreError};

impl App {
    /// Returns the active home directory path.
    pub fn home_dir(&self) -> String {
        self.home().display().to_string()
    }

    /// Prints the active runtime directory for a tool.
    pub fn cd_path(&self, plugin_name: &str) -> Result<String, CoreError> {
        let current = self
            .current_tool(plugin_name)?
            .ok_or_else(|| CoreError::InactiveTool(plugin_name.to_string()))?;
        Ok(self.effective_runtime_dir(&current).display().to_string())
    }

    /// Returns the local plugin source directory.
    pub fn plugin_dir(&self, plugin_name: &str) -> Result<String, CoreError> {
        let entry = self.resolve_registry_entry(plugin_name)?;
        let entry = self.materialize_plugin_entry(&entry)?;
        Ok(self
            .normalize_source_path(&entry.source)
            .display()
            .to_string())
    }
}
