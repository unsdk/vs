//! Services for uninstalling previously materialized runtimes.

use std::fs;

use crate::{App, CoreError, UninstallResult, UseScope};

impl App {
    /// Uninstalls a plugin version from the local cache.
    ///
    /// When the uninstalled version is currently active and other versions
    /// remain, auto-switches to the first remaining version (global scope).
    /// When no versions remain, removes the entire plugin cache directory.
    pub fn uninstall_plugin_version(
        &self,
        plugin_name: &str,
        version: &str,
    ) -> Result<UninstallResult, CoreError> {
        let was_current = self
            .current_tool(plugin_name)?
            .map(|current| current.version == version)
            .unwrap_or(false);

        // Call PreUninstall hook when the plugin and receipt are available.
        if let Ok(Some(runtime)) = self.installer.read_receipt(plugin_name, version) {
            if let Ok(entry) = self.resolve_registry_entry(plugin_name) {
                if let Ok(plugin) = self.load_plugin(&entry) {
                    if let Err(error) = plugin.pre_uninstall(&runtime) {
                        eprintln!("PreUninstall hook failed for {plugin_name}@{version}: {error}");
                    }
                }
            }
        }

        let removed = self.installer.uninstall(plugin_name, version)?;
        if !removed {
            return Ok(UninstallResult {
                removed: false,
                auto_switched: None,
            });
        }

        let remaining = self.installer.installed_versions(plugin_name)?;

        if remaining.is_empty() {
            let plugin_cache = self.home().join("cache").join(plugin_name);
            if plugin_cache.exists() {
                let _ = fs::remove_dir_all(plugin_cache);
            }
            return Ok(UninstallResult {
                removed: true,
                auto_switched: None,
            });
        }

        if was_current {
            let first = &remaining[0];
            let _ = self.use_tool(plugin_name, first, UseScope::Global, false);
            return Ok(UninstallResult {
                removed: true,
                auto_switched: Some(first.clone()),
            });
        }

        Ok(UninstallResult {
            removed: true,
            auto_switched: None,
        })
    }
}
