use vs_plugin_api::AvailableVersion;

use crate::{App, CoreError};

impl App {
    /// Lists available SDK versions for a plugin.
    pub fn search_versions(
        &self,
        plugin_name: &str,
        args: &[String],
    ) -> Result<Vec<AvailableVersion>, CoreError> {
        let entry = self.resolve_registry_entry(plugin_name)?;
        let plugin = self.load_plugin(&entry)?;
        plugin.available_versions(args).map_err(Into::into)
    }
}
