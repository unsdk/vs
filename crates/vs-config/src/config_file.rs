use std::fs;
use std::path::{Path, PathBuf};

use serde_yaml::Value;

use crate::ConfigError;
use crate::types::{AppConfig, ToolVersions};

/// Reads `config.yaml` from the active home.
pub fn read_app_config(home: &Path) -> Result<AppConfig, ConfigError> {
    let path = home.join("config.yaml");
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let content = fs::read_to_string(&path)?;
    serde_yaml::from_str(&content).map_err(|error| ConfigError::Yaml {
        path,
        message: error.to_string(),
    })
}

/// Writes `config.yaml` to the active home.
pub fn write_app_config(home: &Path, config: &AppConfig) -> Result<(), ConfigError> {
    fs::create_dir_all(home)?;
    let path = home.join("config.yaml");
    let rendered = serde_yaml::to_string(config).map_err(|error| ConfigError::Yaml {
        path: path.clone(),
        message: error.to_string(),
    })?;
    fs::write(path, rendered)?;
    Ok(())
}

/// Reads a TOML tool file.
pub fn read_tool_versions(path: &Path) -> Result<ToolVersions, ConfigError> {
    let content = fs::read_to_string(path)?;
    toml::from_str(&content).map_err(|error| ConfigError::Toml {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

/// Writes a TOML tool file.
pub fn write_tool_versions(path: &Path, tools: &ToolVersions) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let rendered = toml::to_string_pretty(tools).map_err(|error| ConfigError::Toml {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    fs::write(path, rendered)?;
    Ok(())
}

/// Lists user-visible configuration values as strings.
pub fn flatten_app_config(config: &AppConfig) -> Vec<(String, String)> {
    vec![
        (
            String::from("legacyVersionFile"),
            config.legacy_version_file.to_string(),
        ),
        (
            String::from("registry.source"),
            config
                .registry
                .source
                .clone()
                .unwrap_or_else(|| String::from("<unset>")),
        ),
    ]
}

/// Sets a top-level config value by key.
pub fn set_app_config_value(
    config: &mut AppConfig,
    key: &str,
    value: &str,
) -> Result<(), ConfigError> {
    match key {
        "legacyVersionFile" => {
            config.legacy_version_file =
                value
                    .parse::<bool>()
                    .map_err(|_| ConfigError::InvalidValue {
                        key: key.to_string(),
                        value: value.to_string(),
                    })?;
        }
        "registry.source" => {
            config.registry.source = Some(value.to_string());
        }
        _ => {
            return Err(ConfigError::UnknownKey(key.to_string()));
        }
    }
    Ok(())
}

/// Unsets a top-level config value by key.
pub fn unset_app_config_value(config: &mut AppConfig, key: &str) -> Result<(), ConfigError> {
    match key {
        "legacyVersionFile" => {
            config.legacy_version_file = AppConfig::default().legacy_version_file;
        }
        "registry.source" => {
            config.registry.source = None;
        }
        _ => {
            return Err(ConfigError::UnknownKey(key.to_string()));
        }
    }
    Ok(())
}

/// Converts an application config to a YAML value for debugging.
pub fn app_config_to_value(config: &AppConfig) -> Result<Value, ConfigError> {
    serde_yaml::to_value(config).map_err(|error| ConfigError::Yaml {
        path: PathBuf::from("config.yaml"),
        message: error.to_string(),
    })
}
