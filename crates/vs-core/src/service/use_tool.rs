use vs_shell::{global_current_dir, link_directory, project_sdk_dir};

use crate::{App, CoreError, InstalledVersion, UseScope};

impl App {
    /// Installs and activates a tool version for a given scope.
    pub fn use_tool(
        &self,
        plugin_name: &str,
        version: &str,
        scope: UseScope,
        unlink: bool,
    ) -> Result<InstalledVersion, CoreError> {
        let installed = self.install_plugin_version(plugin_name, Some(version))?;
        match scope {
            UseScope::Global => {
                self.write_tool_assignment(
                    &vs_config::global_tools_file(self.home()),
                    plugin_name,
                    Some(version),
                )?;
                link_directory(
                    &installed.install_dir,
                    &global_current_dir(self.home(), plugin_name),
                )?;
            }
            UseScope::Project => {
                self.write_tool_assignment(
                    &self.preferred_project_file(),
                    plugin_name,
                    Some(version),
                )?;
                if !unlink {
                    link_directory(
                        &installed.install_dir,
                        &project_sdk_dir(&self.cwd, plugin_name),
                    )?;
                }
            }
            UseScope::Session => {
                let session_file = self.session_file()?;
                self.write_tool_assignment(&session_file, plugin_name, Some(version))?;
            }
        }
        Ok(installed)
    }
}
