//! Services for listing installed runtimes.

use std::fs;

use crate::{App, CoreError, InstalledVersion};

impl App {
    /// Lists installed tool versions in the local cache.
    pub fn list_installed_versions(&self) -> Result<Vec<InstalledVersion>, CoreError> {
        let cache_root = self.home().join("cache");
        if !cache_root.exists() {
            return Ok(Vec::new());
        }
        let mut installed = Vec::new();

        for plugin_entry in fs::read_dir(cache_root)? {
            let plugin_entry = plugin_entry?;
            if !plugin_entry.file_type()?.is_dir() {
                continue;
            }
            let plugin_name = match plugin_entry.file_name().into_string() {
                Ok(name) => name,
                Err(_) => continue,
            };
            let versions_root = plugin_entry.path().join("versions");
            if !versions_root.exists() {
                continue;
            }
            for version_entry in fs::read_dir(&versions_root)? {
                let version_entry = version_entry?;
                if !version_entry.file_type()?.is_dir() {
                    continue;
                }
                if let Ok(version) = version_entry.file_name().into_string() {
                    installed.push(InstalledVersion {
                        plugin: plugin_name.clone(),
                        version,
                        install_dir: version_entry.path(),
                    });
                }
            }
        }

        installed.sort_by(|left, right| {
            left.plugin
                .cmp(&right.plugin)
                .then(right.version.cmp(&left.version))
        });
        Ok(installed)
    }
}
