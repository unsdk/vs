use crate::{App, CoreError, InstalledVersion};

impl App {
    /// Installs a plugin version, choosing the first available version when omitted.
    pub fn install_plugin_version(
        &self,
        plugin_name: &str,
        version: Option<&str>,
    ) -> Result<InstalledVersion, CoreError> {
        let entry = self.resolve_registry_entry(plugin_name)?;
        let plugin = self.load_plugin(&entry)?;
        let available_versions = plugin.available_versions()?;
        let selected_version = version
            .map(str::to_string)
            .or_else(|| {
                available_versions
                    .first()
                    .map(|candidate| candidate.version.clone())
            })
            .ok_or_else(|| {
                CoreError::Unsupported(format!(
                    "plugin {plugin_name} does not expose installable versions"
                ))
            })?;

        let plan = plugin.install_plan(&selected_version)?;
        let install_dir = self.installer.install(&plan)?;
        Ok(InstalledVersion {
            plugin: plugin_name.to_string(),
            version: selected_version,
            install_dir,
        })
    }
}
