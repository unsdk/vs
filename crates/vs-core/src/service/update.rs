//! Services for refreshing plugin metadata and sources.

use vs_plugin_api::PluginBackendKind;
use vs_registry::RegistryEntry;

use crate::plugin_source::is_remote_source;
use crate::registry_source::{
    fetch_plugin_manifest, fetch_plugin_manifest_from_url, fetch_url_text,
    is_remote_registry_source, parse_registry_entries, registry_index_url,
};
use crate::{App, CoreError};

impl App {
    /// Refreshes the searchable plugin index from the configured registry source.
    pub fn update_registry(&self) -> Result<usize, CoreError> {
        let config = self.app_config()?;
        let source = config.registry.address;
        if source.is_empty() {
            return Err(CoreError::Unsupported(String::from(
                "registry.address is not configured",
            )));
        }
        let registry_source = if is_remote_registry_source(&source) {
            registry_index_url(&source)
        } else {
            source
        };
        let mut entries = if is_remote_registry_source(&registry_source) {
            let content = fetch_url_text(&registry_source)?;
            parse_registry_entries(&content).map_err(|error| CoreError::RegistrySource {
                path: registry_source.clone().into(),
                message: error.to_string(),
            })?
        } else {
            let path = self.normalize_source_path(&registry_source);
            let content =
                std::fs::read_to_string(&path).map_err(|error| CoreError::RegistrySource {
                    path: path.clone(),
                    message: error.to_string(),
                })?;
            parse_registry_entries(&content).map_err(|error| CoreError::RegistrySource {
                path: path.clone(),
                message: error.to_string(),
            })?
        };

        if !is_remote_registry_source(&registry_source) {
            let path = self.normalize_source_path(&registry_source);
            let base_dir = path.parent().unwrap_or(&self.cwd);
            for entry in &mut entries {
                let source_path = std::path::PathBuf::from(&entry.source);
                if source_path.is_relative() {
                    entry.source = base_dir.join(source_path).display().to_string();
                }
            }
        }

        entries.retain(|entry| self.ensure_backend_supported(entry.backend).is_ok());
        self.registry.replace_available_plugins(&entries)?;
        Ok(entries.len())
    }

    /// Updates a locally added plugin from its manifest URL or registry metadata.
    pub fn update_plugin(&self, name: &str) -> Result<RegistryEntry, CoreError> {
        let entry = self
            .added_plugins()?
            .into_iter()
            .find(|entry| entry.matches(name))
            .ok_or_else(|| CoreError::UnknownPlugin(name.to_string()))?;
        let materialized = self.materialize_plugin_entry(&entry)?;
        let plugin = self.load_plugin(&materialized)?;
        let manifest = plugin.manifest().clone();

        let mut refreshed = RegistryEntry {
            name: entry.name.clone(),
            source: entry.source.clone(),
            backend: entry.backend,
            description: manifest.description.clone().or(entry.description.clone()),
            aliases: entry.aliases.clone(),
        };

        if let Some(source) =
            self.resolve_plugin_update_source(&entry, &manifest.name, &manifest)?
        {
            refreshed.source = source;
        }

        let refreshed = if is_remote_source(&refreshed.source) {
            self.materialize_plugin_entry_with_refresh(&refreshed, true)?
        } else {
            self.materialize_plugin_entry(&refreshed)?
        };
        self.registry.add_plugin(refreshed.clone())?;
        Ok(refreshed)
    }

    /// Updates every locally added plugin.
    pub fn update_all_plugins(&self) -> Result<Vec<RegistryEntry>, CoreError> {
        let entries = self.added_plugins()?;
        let mut updated = Vec::new();
        for entry in entries {
            updated.push(self.update_plugin(&entry.name)?);
        }
        Ok(updated)
    }

    fn resolve_plugin_update_source(
        &self,
        entry: &RegistryEntry,
        manifest_name: &str,
        manifest: &vs_plugin_api::PluginManifest,
    ) -> Result<Option<String>, CoreError> {
        if let Some(manifest_url) = manifest
            .manifest_url
            .as_deref()
            .or(manifest.update_url.as_deref())
        {
            let plugin_manifest = fetch_plugin_manifest_from_url(manifest_url)?;
            if !plugin_manifest.download_url.is_empty() {
                return Ok(Some(plugin_manifest.download_url));
            }
        }

        if entry.backend == PluginBackendKind::Lua {
            let config = self.app_config()?;
            if !config.registry.address.is_empty() {
                if let Ok(plugin_manifest) =
                    fetch_plugin_manifest(&config.registry.address, manifest_name)
                {
                    if !plugin_manifest.download_url.is_empty() {
                        return Ok(Some(plugin_manifest.download_url));
                    }
                }
            }
        }

        if is_remote_source(&entry.source) {
            return Ok(Some(entry.source.clone()));
        }

        Ok(None)
    }
}
