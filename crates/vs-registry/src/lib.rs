//! Plugin registry storage and lookup.

mod entry;
mod service;
mod store;

use std::path::PathBuf;

use thiserror::Error;

pub use entry::RegistryEntry;
pub use service::RegistryService;
pub use store::RegistryStore;

/// Errors returned by registry services.
#[derive(Debug, Error)]
pub enum RegistryError {
    /// An I/O operation failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A JSON file could not be parsed.
    #[error("failed to parse JSON file at {path}: {message}")]
    Json { path: PathBuf, message: String },
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use tempfile::TempDir;
    use vs_plugin_api::PluginBackendKind;

    use super::{RegistryEntry, RegistryService};

    #[test]
    fn resolve_should_prefer_added_plugins() -> Result<(), Box<dyn Error>> {
        let temp_dir = TempDir::new()?;
        let service = RegistryService::new(temp_dir.path());

        service.replace_available_plugins(&[RegistryEntry {
            name: String::from("nodejs"),
            source: String::from("/index"),
            backend: PluginBackendKind::Lua,
            description: None,
            aliases: Vec::new(),
        }])?;

        service.add_plugin(RegistryEntry {
            name: String::from("nodejs"),
            source: String::from("/added"),
            backend: PluginBackendKind::Lua,
            description: None,
            aliases: Vec::new(),
        })?;

        let resolved = service
            .resolve("nodejs")?
            .ok_or_else(|| std::io::Error::other("missing registry entry"))?;
        assert_eq!(resolved.source, "/added");
        Ok(())
    }
}
