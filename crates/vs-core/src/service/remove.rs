use std::fs;

use crate::{App, CoreError, UseScope};

impl App {
    /// Removes a plugin, its source directory, and all installed SDK versions.
    pub fn remove_plugin(&self, name: &str) -> Result<bool, CoreError> {
        let removed = self.registry.remove_plugin(name)?;
        if !removed {
            return Ok(false);
        }

        // Unuse from global scope (ignore errors if not set).
        let _ = self.unuse_tool(name, UseScope::Global);

        // Remove plugin source directory.
        let source_dir = self.home().join("plugins").join("sources").join(name);
        if source_dir.exists() {
            fs::remove_dir_all(&source_dir)?;
        }

        // Remove all installed SDK versions.
        let cache_dir = self.home().join("cache").join(name);
        if cache_dir.exists() {
            fs::remove_dir_all(&cache_dir)?;
        }

        Ok(true)
    }
}
