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
                .then(compare_versions_desc(&left.version, &right.version))
        });
        Ok(installed)
    }
}

fn compare_versions_desc(left: &str, right: &str) -> std::cmp::Ordering {
    compare_version_components(left, right).reverse()
}

fn compare_version_components(left: &str, right: &str) -> std::cmp::Ordering {
    let left_parts = version_components(left);
    let right_parts = version_components(right);
    for (left_part, right_part) in left_parts.iter().zip(right_parts.iter()) {
        let ordering = match (left_part.parse::<u64>(), right_part.parse::<u64>()) {
            (Ok(left_num), Ok(right_num)) => left_num.cmp(&right_num),
            _ => left_part.cmp(right_part),
        };
        if ordering != std::cmp::Ordering::Equal {
            return ordering;
        }
    }
    left_parts.len().cmp(&right_parts.len())
}

fn version_components(version: &str) -> Vec<&str> {
    version
        .trim_start_matches('v')
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect()
}
