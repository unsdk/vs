use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use mlua::{Lua, LuaSerdeExt, Value};
use serde::Deserialize;
use vs_plugin_api::{
    AvailableVersion, EnvKey, InstallPlan, Plugin, PluginBackendKind, PluginError, PluginManifest,
};

#[derive(Debug, Deserialize)]
struct MetadataFile {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    legacy_filenames: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct InstallSource {
    source: String,
}

#[derive(Debug, Deserialize)]
struct EnvTemplate {
    key: String,
    value: String,
}

/// Lua-backed plugin loaded from a source directory.
#[derive(Debug)]
pub struct LuaPlugin {
    manifest: PluginManifest,
    versions: Vec<AvailableVersion>,
    install_sources: BTreeMap<String, InstallSource>,
    env_templates: Vec<EnvTemplate>,
    legacy_filenames: Vec<String>,
}

impl LuaPlugin {
    /// Loads a plugin from a source directory.
    pub fn load(source: &Path) -> Result<Self, PluginError> {
        let metadata = load_lua_file::<MetadataFile>(source, "metadata.lua")?;
        let versions = load_lua_file::<Vec<AvailableVersion>>(source, "hooks/available.lua")?;
        let install_sources =
            load_lua_file::<BTreeMap<String, InstallSource>>(source, "hooks/pre_install.lua")?;
        let env_templates = load_lua_file::<Vec<EnvTemplate>>(source, "hooks/env_keys.lua")?;

        Ok(Self {
            manifest: PluginManifest {
                name: metadata.name,
                backend: PluginBackendKind::Lua,
                source: source.to_path_buf(),
                description: metadata.description,
                aliases: metadata.aliases,
            },
            versions,
            install_sources,
            env_templates,
            legacy_filenames: metadata.legacy_filenames,
        })
    }
}

impl Plugin for LuaPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn available_versions(&self) -> Result<Vec<AvailableVersion>, PluginError> {
        Ok(self.versions.clone())
    }

    fn install_plan(&self, version: &str) -> Result<InstallPlan, PluginError> {
        let install_source =
            self.install_sources
                .get(version)
                .ok_or_else(|| PluginError::VersionNotFound {
                    plugin: self.manifest.name.clone(),
                    version: version.to_string(),
                })?;
        Ok(InstallPlan {
            plugin: self.manifest.name.clone(),
            version: version.to_string(),
            source_dir: self.manifest.source.join(&install_source.source),
            legacy_filenames: self.legacy_filenames.clone(),
        })
    }

    fn env_keys(&self, _version: &str, install_dir: &Path) -> Result<Vec<EnvKey>, PluginError> {
        Ok(self
            .env_templates
            .iter()
            .map(|template| EnvKey {
                key: template.key.clone(),
                value: template
                    .value
                    .replace("{install_dir}", &install_dir.display().to_string()),
            })
            .collect())
    }

    fn parse_legacy_file(
        &self,
        file_name: &str,
        content: &str,
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

fn load_lua_file<T>(root: &Path, relative_path: &str) -> Result<T, PluginError>
where
    T: for<'de> Deserialize<'de>,
{
    let path = root.join(relative_path);
    let content = fs::read_to_string(&path).map_err(|error| PluginError::InvalidSource {
        path: path.clone(),
        message: error.to_string(),
    })?;
    let lua = Lua::new();
    let value = lua
        .load(&content)
        .set_name(path.display().to_string())
        .eval::<Value>()
        .map_err(|error| PluginError::InvalidSource {
            path: path.clone(),
            message: error.to_string(),
        })?;
    lua.from_value(value)
        .map_err(|error| PluginError::InvalidSource {
            path,
            message: error.to_string(),
        })
}

/// Returns the path where a Lua plugin should store helper modules.
pub fn lua_library_dir(root: &Path) -> PathBuf {
    root.join("lib")
}
