//! Shared configuration data types used across the workspace.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Resolution scope for a tool version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Version pinned in a project file.
    Project,
    /// Version pinned for the active shell session.
    Session,
    /// Version pinned globally for the user.
    Global,
    /// Version provided by the operating system.
    System,
}

/// Source metadata for a resolved tool version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedToolVersion {
    /// Tool name.
    pub plugin: String,
    /// Resolved version string.
    pub version: String,
    /// Scope that produced the version.
    pub scope: Scope,
    /// Source file backing the version.
    pub source: PathBuf,
}

/// The active home directory plus migration candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeLayout {
    /// Active home used by `vs`.
    pub active_home: PathBuf,
    /// Legacy home locations that may be migrated.
    pub migration_candidates: Vec<PathBuf>,
}

/// Global configuration file for `vs`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct AppConfig {
    /// Proxy configuration.
    pub proxy: ProxyConfig,
    /// SDK storage configuration.
    pub storage: StorageConfig,
    /// Registry configuration.
    pub registry: RegistryConfig,
    /// Legacy version file settings.
    pub legacy_version_file: LegacyVersionFileConfig,
    /// Cache settings.
    pub cache: CacheConfig,
}

/// Proxy configuration written to `config.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ProxyConfig {
    /// Whether the proxy is enabled.
    pub enable: bool,
    /// Proxy URL.
    pub url: String,
}

/// Storage configuration written to `config.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct StorageConfig {
    /// Alternative SDK storage root.
    pub sdk_path: String,
}

/// Registry configuration written to `config.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct RegistryConfig {
    /// Registry base address or index URL.
    pub address: String,
}

/// Legacy version file configuration written to `config.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LegacyVersionFileConfig {
    /// Whether legacy file parsing is enabled.
    pub enable: bool,
    /// Legacy parsing strategy.
    pub strategy: String,
}

impl Default for LegacyVersionFileConfig {
    fn default() -> Self {
        Self {
            enable: true,
            strategy: String::from("specified"),
        }
    }
}

/// Cache configuration written to `config.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CacheConfig {
    /// Available hook cache duration string.
    pub available_hook_duration: String,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            available_hook_duration: String::from("12h"),
        }
    }
}

/// Tool versions written in TOML configuration files.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ToolVersions {
    /// Tools pinned in the file.
    #[serde(default)]
    pub tools: BTreeMap<String, String>,
}
