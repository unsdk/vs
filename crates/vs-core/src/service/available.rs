//! Services for listing available plugins from configured registries.

use vs_registry::RegistryEntry;

use crate::{App, CoreError};

impl App {
    /// Lists plugins in the available registry index.
    pub fn available_plugins(&self) -> Result<Vec<RegistryEntry>, CoreError> {
        self.refresh_registry_index_with_fallback()?;
        self.registry.available_plugins().map_err(Into::into)
    }

    /// Lists plugins added to the local home.
    pub fn added_plugins(&self) -> Result<Vec<RegistryEntry>, CoreError> {
        self.registry.added_plugins().map_err(Into::into)
    }
}
