//! Services for querying plugin-provided version listings.

use vs_plugin_api::AvailableVersion;

use crate::{App, CoreError};

impl App {
    /// Lists available SDK versions for a plugin.
    pub fn search_versions(
        &self,
        plugin_name: &str,
        args: &[String],
    ) -> Result<Vec<AvailableVersion>, CoreError> {
        self.cached_available_versions(plugin_name, args)
    }
}
