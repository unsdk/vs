//! Types describing plugin registry entries.

use serde::{Deserialize, Serialize};
use vs_plugin_api::{PluginBackendKind, PluginManifest};

/// A plugin entry stored by the registry service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// Stable plugin identifier.
    pub name: String,
    /// Local source directory or registry target.
    pub source: String,
    /// Backend used to execute the plugin.
    pub backend: PluginBackendKind,
    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,
    /// Alternative names accepted by the CLI.
    #[serde(default)]
    pub aliases: Vec<String>,
}

impl RegistryEntry {
    /// Converts the entry into a plugin manifest.
    pub fn to_manifest(&self) -> PluginManifest {
        PluginManifest {
            name: self.name.clone(),
            backend: self.backend,
            source: self.source.clone().into(),
            description: self.description.clone(),
            aliases: self.aliases.clone(),
            version: None,
            homepage: None,
            license: None,
            update_url: None,
            manifest_url: None,
            min_runtime_version: None,
            notes: Vec::new(),
            legacy_filenames: Vec::new(),
        }
    }

    /// Matches the entry against a name or alias.
    pub fn matches(&self, query: &str) -> bool {
        self.name == query || self.aliases.iter().any(|alias| alias == query)
    }
}
