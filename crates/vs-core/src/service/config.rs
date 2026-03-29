use vs_config::{
    app_config_to_value, flatten_app_config, read_app_config, set_app_config_value,
    unset_app_config_value, write_app_config,
};

use crate::{App, CoreError};

impl App {
    /// Lists current application config values.
    pub fn list_config(&self) -> Result<Vec<(String, String)>, CoreError> {
        let config = self.app_config()?;
        Ok(flatten_app_config(&config))
    }

    /// Returns the whole config as a YAML-like value.
    pub fn config_value(&self) -> Result<serde_yaml::Value, CoreError> {
        let config = self.app_config()?;
        app_config_to_value(&config).map_err(Into::into)
    }

    /// Returns config entries for an exact key or section prefix.
    pub fn config_entries_for_key(&self, key: &str) -> Result<Vec<(String, String)>, CoreError> {
        let entries = self.list_config()?;
        if let Some(entry) = entries.iter().find(|(entry_key, _)| entry_key == key) {
            return Ok(vec![entry.clone()]);
        }

        let prefix = format!("{key}.");
        let matching = entries
            .into_iter()
            .filter(|(entry_key, _)| entry_key.starts_with(&prefix))
            .collect::<Vec<_>>();
        if matching.is_empty() {
            return Err(CoreError::Config(vs_config::ConfigError::UnknownKey(
                key.to_string(),
            )));
        }
        Ok(matching)
    }

    /// Sets a config key.
    pub fn set_config_value(&self, key: &str, value: &str) -> Result<(), CoreError> {
        let mut config = read_app_config(self.home())?;
        set_app_config_value(&mut config, key, value)?;
        write_app_config(self.home(), &config)?;
        Ok(())
    }

    /// Unsets a config key.
    pub fn unset_config_value(&self, key: &str) -> Result<(), CoreError> {
        let mut config = read_app_config(self.home())?;
        unset_app_config_value(&mut config, key)?;
        write_app_config(self.home(), &config)?;
        Ok(())
    }
}
