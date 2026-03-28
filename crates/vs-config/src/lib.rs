//! Home, config, and version resolution for `vs`.

mod config_file;
mod home;
mod legacy;
mod project;
mod resolution;
mod types;

use std::path::PathBuf;

use thiserror::Error;

pub use config_file::{
    app_config_to_value, flatten_app_config, read_app_config, read_tool_versions,
    set_app_config_value, unset_app_config_value, write_app_config, write_tool_versions,
};
pub use home::{resolve_home, resolve_home_with};
pub use legacy::{
    find_legacy_file, parse_legacy_versions, read_legacy_versions, supported_legacy_files,
};
pub use project::{find_project_file, preferred_project_file};
pub use resolution::{global_tools_file, resolve_tool_version, session_tools_file};
pub use types::{
    AppConfig, CacheConfig, HomeLayout, LegacyVersionFileConfig, ProxyConfig, RegistryConfig,
    ResolvedToolVersion, Scope, StorageConfig, ToolVersions,
};

/// Errors returned by configuration services.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The system home directory is unavailable.
    #[error("failed to resolve the user home directory")]
    HomeDirectoryUnavailable,
    /// An I/O operation failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A TOML file could not be parsed.
    #[error("failed to parse TOML file at {path}: {message}")]
    Toml { path: PathBuf, message: String },
    /// A YAML file could not be parsed.
    #[error("failed to parse YAML file at {path}: {message}")]
    Yaml { path: PathBuf, message: String },
    /// A requested configuration key is not supported.
    #[error("unknown config key: {0}")]
    UnknownKey(String),
    /// A configuration value could not be parsed.
    #[error("invalid value {value} for key {key}")]
    InvalidValue { key: String, value: String },
    /// A legacy file name is unsupported.
    #[error("unsupported legacy file: {0}")]
    UnsupportedLegacyFile(PathBuf),
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn resolve_home_should_prefer_vs_home() -> Result<(), Box<dyn Error>> {
        let temp_dir = TempDir::new()?;
        let explicit_home = temp_dir.path().join("explicit");
        let layout = resolve_home_with(Some(explicit_home.clone()), temp_dir.path().join("user"))?;

        assert_eq!(layout.active_home, explicit_home);
        assert!(layout.migration_candidates.is_empty());
        Ok(())
    }

    #[test]
    fn resolve_tool_version_should_prefer_project_config_over_global() -> Result<(), Box<dyn Error>>
    {
        let temp_dir = TempDir::new()?;
        let home = temp_dir.path().join("home");
        fs::create_dir_all(home.join("global"))?;
        write_tool_versions(
            &global_tools_file(&home),
            &ToolVersions {
                tools: [("nodejs".to_string(), "18.19.0".to_string())]
                    .into_iter()
                    .collect(),
            },
        )?;

        let project = temp_dir.path().join("project");
        fs::create_dir_all(&project)?;
        write_tool_versions(
            &preferred_project_file(&project),
            &ToolVersions {
                tools: [("nodejs".to_string(), "20.11.1".to_string())]
                    .into_iter()
                    .collect(),
            },
        )?;

        let resolved = resolve_tool_version(&home, &project, None, "nodejs")?
            .ok_or_else(|| std::io::Error::other("missing resolved version"))?;

        assert_eq!(resolved.version, "20.11.1");
        assert_eq!(resolved.scope, Scope::Project);
        Ok(())
    }

    #[test]
    fn parse_legacy_versions_should_support_tool_versions() -> Result<(), Box<dyn Error>> {
        let versions =
            parse_legacy_versions(Some(".tool-versions"), "nodejs 20.11.1\njava 21-tem\n")
                .ok_or_else(|| std::io::Error::other("missing parsed legacy versions"))?;

        assert_eq!(versions.tools.get("nodejs"), Some(&String::from("20.11.1")));
        assert_eq!(versions.tools.get("java"), Some(&String::from("21-tem")));
        Ok(())
    }
}
