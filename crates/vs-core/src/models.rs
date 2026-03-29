use std::path::PathBuf;

use vs_config::Scope;
use vs_plugin_api::{AvailableVersion, PluginManifest};
use vs_registry::RegistryEntry;

/// Scope to target when writing tool versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseScope {
    /// Persist the version globally.
    Global,
    /// Persist the version in the current project.
    Project,
    /// Persist the version for the active shell session.
    Session,
}

impl UseScope {
    /// Returns the stable lowercase scope label used by CLI output and plugin hooks.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Project => "project",
            Self::Session => "session",
        }
    }
}

/// A currently resolved tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentTool {
    /// Tool identifier.
    pub plugin: String,
    /// Resolved version.
    pub version: String,
    /// Resolution scope.
    pub scope: Scope,
    /// Backing source file.
    pub source: PathBuf,
}

/// A discovered installed version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledVersion {
    /// Tool identifier.
    pub plugin: String,
    /// Installed version.
    pub version: String,
    /// Installation path.
    pub install_dir: PathBuf,
}

/// Summary for `vs info`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginInfo {
    /// Registry entry used to resolve the plugin.
    pub entry: RegistryEntry,
    /// Static plugin manifest.
    pub manifest: PluginManifest,
    /// Available versions exposed by the backend.
    pub available_versions: Vec<AvailableVersion>,
    /// Installed versions already present in the local cache.
    pub installed_versions: Vec<String>,
}

/// Migration result for `vs migrate`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrateSummary {
    /// Source home that was migrated.
    pub source_home: PathBuf,
    /// Number of filesystem roots copied.
    pub copied_roots: usize,
}

/// Summary for a self-upgrade operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfUpgradeSummary {
    /// Currently running version.
    pub current_version: String,
    /// Latest available version discovered from the release feed.
    pub latest_version: String,
    /// Whether the binary was replaced.
    pub updated: bool,
}
