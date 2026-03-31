//! Backend adapter for loading native WASI plugins.

use std::fs;
use std::path::Path;

use serde::Deserialize;
use vs_plugin_api::{
    AvailableVersion, EnvKey, InstallArtifact, InstallPlan, InstallSource, InstalledRuntime,
    Plugin, PluginBackendKind, PluginError, PluginManifest,
};

#[derive(Debug, Deserialize)]
struct DescriptorFile {
    plugin: PluginSection,
    #[serde(default)]
    versions: Vec<VersionSection>,
    #[serde(default)]
    env: Vec<EnvSection>,
}

#[derive(Debug, Deserialize)]
struct PluginSection {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    legacy_filenames: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct VersionSection {
    version: String,
    source: String,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EnvSection {
    key: String,
    value: String,
}

/// Manifest-backed runtime that models a native WASI plugin contract.
#[derive(Debug)]
pub struct WasiPlugin {
    manifest: PluginManifest,
    versions: Vec<VersionSection>,
    env: Vec<EnvSection>,
    legacy_filenames: Vec<String>,
}

impl WasiPlugin {
    /// Loads a native plugin descriptor from `component.toml`.
    pub fn load(source: &Path) -> Result<Self, PluginError> {
        let path = source.join("component.toml");
        let content = fs::read_to_string(&path).map_err(|error| PluginError::InvalidSource {
            path: path.clone(),
            message: error.to_string(),
        })?;
        let descriptor = toml::from_str::<DescriptorFile>(&content).map_err(|error| {
            PluginError::InvalidSource {
                path,
                message: error.to_string(),
            }
        })?;

        Ok(Self {
            manifest: PluginManifest {
                name: descriptor.plugin.name,
                backend: PluginBackendKind::Wasi,
                source: source.to_path_buf(),
                description: descriptor.plugin.description,
                aliases: descriptor.plugin.aliases,
                version: None,
                homepage: None,
                license: None,
                update_url: None,
                manifest_url: None,
                min_runtime_version: None,
                notes: Vec::new(),
                legacy_filenames: descriptor.plugin.legacy_filenames.clone(),
            },
            versions: descriptor.versions,
            env: descriptor.env,
            legacy_filenames: descriptor.plugin.legacy_filenames,
        })
    }
}

impl Plugin for WasiPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn available_versions(&self, _args: &[String]) -> Result<Vec<AvailableVersion>, PluginError> {
        Ok(self
            .versions
            .iter()
            .map(|version| AvailableVersion {
                version: version.version.clone(),
                note: version.note.clone(),
                additions: Vec::new(),
            })
            .collect())
    }

    fn install_plan(&self, version: &str) -> Result<InstallPlan, PluginError> {
        let version = self
            .versions
            .iter()
            .find(|candidate| candidate.version == version)
            .ok_or_else(|| PluginError::VersionNotFound {
                plugin: self.manifest.name.clone(),
                version: version.to_string(),
            })?;
        Ok(InstallPlan {
            plugin: self.manifest.name.clone(),
            version: version.version.clone(),
            main: InstallArtifact {
                name: self.manifest.name.clone(),
                version: version.version.clone(),
                source: InstallSource::Directory {
                    path: self.manifest.source.join(&version.source),
                },
                note: version.note.clone(),
                checksum: None,
            },
            additions: Vec::new(),
            legacy_filenames: self.legacy_filenames.clone(),
        })
    }

    fn env_keys(&self, runtime: &InstalledRuntime) -> Result<Vec<EnvKey>, PluginError> {
        Ok(self
            .env
            .iter()
            .map(|entry| EnvKey {
                key: entry.key.clone(),
                value: entry
                    .value
                    .replace("{install_dir}", &runtime.main.path.display().to_string()),
            })
            .collect())
    }

    fn parse_legacy_file(
        &self,
        file_name: &str,
        _file_path: &Path,
        content: &str,
        _installed_versions: &[String],
    ) -> Result<Option<String>, PluginError> {
        if self.legacy_filenames.iter().any(|name| name == file_name) {
            let trimmed = content.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                Ok(Some(trimmed.to_string()))
            }
        } else {
            Ok(None)
        }
    }
}

/// Loads native plugins backed by a typed descriptor.
#[derive(Debug, Default, Clone, Copy)]
pub struct WasiBackend;

impl WasiBackend {
    /// Loads a native plugin from disk.
    pub fn load(&self, source: &Path) -> Result<Box<dyn Plugin>, PluginError> {
        Ok(Box::new(WasiPlugin::load(source)?))
    }
}
