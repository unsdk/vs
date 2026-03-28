use std::path::PathBuf;

use vs_plugin_api::PluginBackendKind;
use vs_registry::RegistryEntry;

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
            RegistryEntry {
                name: name.to_string(),
                source: self.normalize_source_path(&source).display().to_string(),
                backend: backend.unwrap_or(PluginBackendKind::Lua),
                description: None,
                aliases: Vec::new(),
            }
        } else {
            let entry = self.resolve_registry_entry(name)?;
            RegistryEntry {
                source: PathBuf::from(&entry.source)
                    .canonicalize()
                    .unwrap_or_else(|_| self.normalize_source_path(&entry.source))
                    .display()
                    .to_string(),
                ..entry
            }
        };

        self.registry.add_plugin(entry.clone())?;
        Ok(entry)
    }
}
