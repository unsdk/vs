use vs_registry::RegistryEntry;

use crate::{App, CoreError};

impl App {
    /// Lists plugins in the available registry index.
    pub fn available_plugins(&self) -> Result<Vec<RegistryEntry>, CoreError> {
        self.registry.available_plugins().map_err(Into::into)
    }
}
