//! Services for removing active version selections from a scope.

use vs_shell::{global_current_dir, project_sdk_dir, remove_existing};

use crate::{App, CoreError, UseScope};

impl App {
    /// Removes an active tool version from the selected scope.
    pub fn unuse_tool(&self, plugin_name: &str, scope: UseScope) -> Result<(), CoreError> {
        match scope {
            UseScope::Global => {
                self.write_tool_assignment(
                    &vs_config::global_tools_file(self.home()),
                    plugin_name,
                    None,
                )?;
                remove_existing(&global_current_dir(self.home(), plugin_name))?;
            }
            UseScope::Project => {
                self.write_tool_assignment(&self.preferred_project_file(), plugin_name, None)?;
                remove_existing(&project_sdk_dir(&self.cwd, plugin_name))?;
            }
            UseScope::Session => {
                let session_file = self.session_file()?;
                self.write_tool_assignment(&session_file, plugin_name, None)?;
            }
        }
        Ok(())
    }
}
