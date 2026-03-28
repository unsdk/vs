use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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
    /// Plugin runtime version.
    #[serde(default)]
    pub version: Option<String>,
    /// Plugin homepage or repository.
    #[serde(default)]
    pub homepage: Option<String>,
    /// Plugin update URL.
    #[serde(default)]
    pub update_url: Option<String>,
    /// Plugin manifest URL.
    #[serde(default)]
    pub manifest_url: Option<String>,
    /// Minimum runtime version required by the plugin.
    #[serde(default)]
    pub min_runtime_version: Option<String>,
    /// Additional plugin notes.
    #[serde(default)]
    pub notes: Vec<String>,
    /// Legacy file names handled by the plugin.
    #[serde(default)]
    pub legacy_filenames: Vec<String>,
}

/// A version published by a plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvailableVersion {
    /// Version string as understood by the plugin.
    pub version: String,
    /// Optional human-readable note.
    #[serde(default)]
    pub note: Option<String>,
    /// Additional packages associated with the version.
    #[serde(default)]
    pub additions: Vec<AvailableAddition>,
}

/// An additional package listed next to an available version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvailableAddition {
    /// Package name.
    pub name: String,
    /// Package version.
    pub version: String,
    /// Optional note.
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

/// A checksum used to verify a downloaded artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checksum {
    /// Checksum algorithm.
    pub algorithm: String,
    /// Checksum value.
    pub value: String,
}

/// An artifact source referenced by an install plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum InstallSource {
    /// A local directory that should be copied recursively.
    Directory { path: PathBuf },
    /// A local file that may be moved or extracted.
    File { path: PathBuf },
    /// A remote URL that should be downloaded.
    Url {
        url: String,
        #[serde(default)]
        headers: BTreeMap<String, String>,
    },
}

/// An artifact returned by a plugin install hook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallArtifact {
    /// Artifact name.
    pub name: String,
    /// Artifact version.
    pub version: String,
    /// Artifact source.
    pub source: InstallSource,
    /// Optional note.
    #[serde(default)]
    pub note: Option<String>,
    /// Optional checksum.
    #[serde(default)]
    pub checksum: Option<Checksum>,
}

/// Installation plan returned by a plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallPlan {
    /// Plugin name used for diagnostics.
    pub plugin: String,
    /// Version that will be installed.
    pub version: String,
    /// Primary runtime artifact.
    pub main: InstallArtifact,
    /// Additional artifacts.
    #[serde(default)]
    pub additions: Vec<InstallArtifact>,
    /// Legacy file names understood by the plugin.
    #[serde(default)]
    pub legacy_filenames: Vec<String>,
}

/// An installed artifact on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledArtifact {
    /// Artifact name.
    pub name: String,
    /// Artifact version.
    pub version: String,
    /// Installed path.
    pub path: PathBuf,
    /// Optional note.
    #[serde(default)]
    pub note: Option<String>,
}

/// Fully installed runtime layout for a plugin version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledRuntime {
    /// Plugin identifier.
    pub plugin: String,
    /// Installed version.
    pub version: String,
    /// Version root directory.
    pub root_dir: PathBuf,
    /// Main installed artifact.
    pub main: InstalledArtifact,
    /// Additional installed artifacts.
    #[serde(default)]
    pub additions: Vec<InstalledArtifact>,
}

impl InstalledRuntime {
    /// Returns the main runtime path.
    pub fn main_path(&self) -> &Path {
        &self.main.path
    }
}
