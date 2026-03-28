use crate::{App, CoreError, InstalledVersion};

impl App {
    /// Installs the newest available version for a plugin.
    pub fn upgrade_plugin(&self, plugin_name: &str) -> Result<InstalledVersion, CoreError> {
        self.install_plugin_version(plugin_name, None)
    }
}
