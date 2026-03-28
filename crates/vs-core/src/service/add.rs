use vs_plugin_api::PluginBackendKind;
use vs_registry::RegistryEntry;

use crate::plugin_source::is_remote_source;
use crate::registry_source::fetch_plugin_manifest;
use crate::{App, CoreError};

impl App {
    /// Adds a plugin to the local home.
    pub fn add_plugin(
        &self,
        name: &str,
        source: Option<String>,
        backend: Option<PluginBackendKind>,
    ) -> Result<RegistryEntry, CoreError> {
        let entry = if let Some(source) = source {
            let backend = backend.unwrap_or(self.default_backend()?);
            self.ensure_backend_supported(backend)?;
            RegistryEntry {
                name: name.to_string(),
                source: if is_remote_source(&source) {
                    source
                } else {
                    self.normalize_source_path(&source).display().to_string()
                },
                backend,
                description: None,
                aliases: Vec::new(),
            }
        } else {
            let mut entry = self.resolve_registry_entry(name)?;
            if entry.backend == PluginBackendKind::Lua {
                let config = self.app_config()?;
                if !config.registry.address.is_empty() {
                    if let Ok(manifest) =
                        fetch_plugin_manifest(&config.registry.address, &entry.name)
                    {
                        entry.source = manifest.download_url;
                        entry.description = manifest.description.or(entry.description);
                    }
                }
            }
            entry
        };
        let entry = self.materialize_plugin_entry(&entry)?;

        self.registry.add_plugin(entry.clone())?;
        Ok(entry)
    }
}
