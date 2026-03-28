use std::fs;
use std::path::PathBuf;

use vs_registry::RegistryEntry;

use crate::{App, CoreError};

impl App {
    /// Refreshes the searchable plugin index from the configured registry source.
    pub fn update_registry(&self) -> Result<usize, CoreError> {
        let config = self.app_config()?;
        let source = config.registry.source.ok_or_else(|| {
            CoreError::Unsupported(String::from("registry.source is not configured"))
        })?;
        let path = self.normalize_source_path(&source);
        let content = fs::read_to_string(&path).map_err(|error| CoreError::RegistrySource {
            path: path.clone(),
            message: error.to_string(),
        })?;
        let mut entries =
            serde_json::from_str::<Vec<RegistryEntry>>(&content).map_err(|error| {
                CoreError::RegistrySource {
                    path: path.clone(),
                    message: error.to_string(),
                }
            })?;

        let base_dir = path.parent().unwrap_or(&self.cwd);
        for entry in &mut entries {
            let source_path = PathBuf::from(&entry.source);
            if source_path.is_relative() {
                entry.source = base_dir.join(source_path).display().to_string();
            }
        }

        self.registry.replace_available_plugins(&entries)?;
        Ok(entries.len())
    }
}
