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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppConfig {
    /// Enables legacy file scanning.
    pub legacy_version_file: bool,
    /// Optional registry source metadata.
    pub registry: RegistryConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            legacy_version_file: true,
            registry: RegistryConfig::default(),
        }
    }
}

/// Registry configuration written to `config.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct RegistryConfig {
    /// Optional registry index path or URL.
    pub source: Option<String>,
}

/// Tool versions written in TOML configuration files.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ToolVersions {
    /// Tools pinned in the file.
    #[serde(default)]
    pub tools: BTreeMap<String, String>,
}
