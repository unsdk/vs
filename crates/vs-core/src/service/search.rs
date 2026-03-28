use vs_registry::RegistryEntry;

use crate::{App, CoreError};

impl App {
    /// Searches the available registry index.
    pub fn search_plugins(&self, query: &str) -> Result<Vec<RegistryEntry>, CoreError> {
        self.registry.search(query).map_err(Into::into)
    }
}
