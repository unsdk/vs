//! Plugin registration services.

use vs_plugin_api::PluginBackendKind;
use vs_registry::RegistryEntry;

use crate::plugin_source::is_remote_source;
use crate::registry_source::fetch_plugin_manifest;
use crate::{App, CoreError};

impl App {
    /// Adds a plugin to the local home.
    pub fn add_plugin(
        &self,
        name: Option<&str>,
        source: Option<String>,
        backend: Option<PluginBackendKind>,
        alias: Option<&str>,
    ) -> Result<RegistryEntry, CoreError> {
        let entry = if let Some(source) = source {
            let backend = backend.unwrap_or(self.default_backend()?);
            self.ensure_backend_supported(backend)?;
            let requested_name = name.or(alias).unwrap_or("plugin");
            let seed_entry = RegistryEntry {
                name: requested_name.to_string(),
                source: if is_remote_source(&source) {
                    source
                } else {
                    self.normalize_source_path(&source).display().to_string()
                },
                backend,
                description: None,
                aliases: Vec::new(),
            };
            let materialized = self.materialize_plugin_entry(&seed_entry)?;
            let plugin = self.load_plugin(&materialized)?;
            let manifest = plugin.manifest().clone();
            let final_name = alias.or(name).unwrap_or(&manifest.name).to_string();
            let mut aliases = manifest.aliases;
            push_alias(&mut aliases, name);
            push_alias(&mut aliases, Some(&manifest.name));
            aliases.retain(|entry_alias| entry_alias != &final_name);
            RegistryEntry {
                name: final_name,
                source: materialized.source,
                backend,
                description: manifest.description,
                aliases,
            }
        } else {
            let name = name.ok_or_else(|| {
                CoreError::Unsupported(String::from(
                    "add requires a plugin name unless --source is provided",
                ))
            })?;
            let mut entry = self.resolve_registry_entry(name)?;
            if entry.backend == PluginBackendKind::Lua {
                let config = self.app_config()?;
                if !config.registry.address.is_empty() {
                    if let Ok(manifest) = fetch_plugin_manifest(
                        &config.registry.address,
                        &entry.name,
                        self.proxy_url(),
                    ) {
                        entry.source = manifest.download_url;
                        entry.description = manifest.description.or(entry.description);
                    }
                }
            }
            let materialized = self.materialize_plugin_entry(&entry)?;
            let plugin = self.load_plugin(&materialized)?;
            let manifest = plugin.manifest().clone();
            let final_name = alias.unwrap_or(name).to_string();
            let mut aliases = entry.aliases;
            push_alias(&mut aliases, Some(name));
            push_alias(&mut aliases, Some(&manifest.name));
            aliases.extend(manifest.aliases);
            aliases.sort();
            aliases.dedup();
            aliases.retain(|entry_alias| entry_alias != &final_name);
            RegistryEntry {
                name: final_name,
                source: materialized.source,
                backend: entry.backend,
                description: manifest.description.or(entry.description),
                aliases,
            }
        };

        self.registry.add_plugin(entry.clone())?;
        Ok(entry)
    }
}

fn push_alias(aliases: &mut Vec<String>, alias: Option<&str>) {
    if let Some(alias) = alias {
        let alias = alias.trim();
        if !alias.is_empty() {
            aliases.push(alias.to_string());
        }
    }
}
