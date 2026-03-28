use crate::{App, CoreError};

impl App {
    /// Removes a plugin from the local home.
    pub fn remove_plugin(&self, name: &str) -> Result<bool, CoreError> {
        self.registry.remove_plugin(name).map_err(Into::into)
    }
}
