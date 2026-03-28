use crate::{App, CoreError, PluginInfo};

impl App {
    /// Loads registry and backend metadata for a plugin.
    pub fn plugin_info(&self, name: &str) -> Result<PluginInfo, CoreError> {
        let entry = self.resolve_registry_entry(name)?;
        let plugin = self.load_plugin(&entry)?;
        let manifest = plugin.manifest().clone();
        let available_versions = plugin.available_versions(&[])?;
        let installed_versions = self.installer.installed_versions(&manifest.name)?;

        Ok(PluginInfo {
            entry,
            manifest,
            available_versions,
            installed_versions,
        })
    }
}
