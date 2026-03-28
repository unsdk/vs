use crate::{App, CoreError};

impl App {
    /// Uninstalls a plugin version from the local cache.
    pub fn uninstall_plugin_version(
        &self,
        plugin_name: &str,
        version: &str,
    ) -> Result<bool, CoreError> {
        self.installer
            .uninstall(plugin_name, version)
            .map_err(Into::into)
    }
}
