use vs_plugin_api::PluginBackendKind;
use vs_registry::RegistryEntry;

use crate::plugin_source::is_remote_source;
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
            self.resolve_registry_entry(name)?
        };
        let entry = self.materialize_plugin_entry(&entry)?;

        self.registry.add_plugin(entry.clone())?;
        Ok(entry)
    }
}
