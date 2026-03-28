use crate::registry_source::{
    fetch_url_text, is_remote_registry_source, parse_registry_entries, registry_index_url,
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
}
