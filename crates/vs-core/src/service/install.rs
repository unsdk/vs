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
        let available_versions = plugin.available_versions(&[])?;
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
        let runtime = self.installer.install(&plan)?;
        plugin.post_install(&runtime)?;
        Ok(InstalledVersion {
            plugin: plugin_name.to_string(),
            version: runtime.version,
            install_dir: runtime.root_dir,
        })
    }

    /// Returns the version requested by the project config for a plugin, when present.
    pub fn project_tool_version(&self, plugin_name: &str) -> Result<Option<String>, CoreError> {
        Ok(vs_config::find_project_file(&self.cwd)
            .map(|path| vs_config::read_tool_versions(&path))
            .transpose()?
            .and_then(|tools| tools.tools.get(plugin_name).cloned()))
    }

    /// Lists configured tools that should be installed for the current context.
    pub fn configured_tools_for_install(&self) -> Result<Vec<(String, String)>, CoreError> {
        Ok(self
            .collect_current_tools()?
            .into_iter()
            .map(|tool| (tool.plugin, tool.version))
            .collect())
    }
}
