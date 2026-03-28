use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Supported plugin backend implementations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PluginBackendKind {
    /// Lua-compatible plugin backend.
    Lua,
    /// Native plugin backend modeled after a WASI component contract.
    Wasi,
}

/// Registry metadata for a plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Stable plugin identifier.
    pub name: String,
    /// Runtime backend used to execute hooks.
    pub backend: PluginBackendKind,
    /// Local source directory of the plugin.
    pub source: PathBuf,
    /// Optional human readable description.
    #[serde(default)]
    pub description: Option<String>,
    /// Alternative names accepted by the CLI.
    #[serde(default)]
    pub aliases: Vec<String>,
}

/// A version published by a plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvailableVersion {
    /// Version string as understood by the plugin.
    pub version: String,
    /// Optional human-readable note.
    #[serde(default)]
    pub note: Option<String>,
}

/// An environment key emitted by a plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvKey {
    /// Environment variable name.
    pub key: String,
    /// Environment variable value.
    pub value: String,
}

/// Filesystem location that should be installed for a version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallPlan {
    /// Plugin name used for diagnostics.
    pub plugin: String,
    /// Version that will be installed.
    pub version: String,
    /// Source directory containing the unpacked runtime.
    pub source_dir: PathBuf,
    /// Legacy file names understood by the plugin.
    #[serde(default)]
    pub legacy_filenames: Vec<String>,
}
