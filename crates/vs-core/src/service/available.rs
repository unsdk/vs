use vs_registry::RegistryEntry;

use crate::{App, CoreError};

impl App {
    /// Lists plugins in the available registry index.
    pub fn available_plugins(&self) -> Result<Vec<RegistryEntry>, CoreError> {
        self.ensure_registry_index_loaded()?;
        self.registry.available_plugins().map_err(Into::into)
    }

    /// Lists plugins added to the local home.
    pub fn added_plugins(&self) -> Result<Vec<RegistryEntry>, CoreError> {
        self.registry.added_plugins().map_err(Into::into)
    }
}
