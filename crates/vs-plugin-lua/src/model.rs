//! Types exchanged between Lua hooks and the Rust host runtime.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use vs_plugin_api::{AvailableAddition, AvailableVersion, InstalledRuntime, PluginManifest};

/// Metadata extracted from a Lua plugin's top-level table.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataFile {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub update_url: Option<String>,
    #[serde(default)]
    pub manifest_url: Option<String>,
    #[serde(default)]
    pub min_runtime_version: Option<String>,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub legacy_filenames: Vec<String>,
}

/// Context passed to the `Available` hook.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableHookCtx {
    pub args: Vec<String>,
    pub runtime_version: &'static str,
}

/// One version candidate returned by the `Available` hook.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableHookResultItem {
    pub version: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub addition: Vec<AvailableAdditionItem>,
}

/// Additional artifact metadata nested under an available version.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableAdditionItem {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

impl From<AvailableHookResultItem> for AvailableVersion {
    fn from(value: AvailableHookResultItem) -> Self {
        Self {
            version: value.version,
            note: value.note,
            additions: value
                .addition
                .into_iter()
                .map(|addition| AvailableAddition {
                    name: addition.name,
                    version: addition.version.unwrap_or_default(),
                    note: addition.note,
                })
                .collect(),
        }
    }
}

/// Context passed to the `PreInstall` hook for a requested version.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreInstallHookCtx<'a> {
    pub version: &'a str,
    pub runtime_version: &'static str,
}

/// Install metadata returned by the `PreInstall` hook.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreInstallHookResult {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub sha512: Option<String>,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub md5: Option<String>,
    #[serde(default)]
    pub addition: Vec<PreInstallAdditionItem>,
}

/// Additional install artifact returned by the `PreInstall` hook.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreInstallAdditionItem {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub sha512: Option<String>,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub md5: Option<String>,
}

/// Serializable view of an installed package exposed back to Lua hooks.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPackageItem {
    pub path: String,
    pub version: String,
    pub name: String,
    #[serde(default)]
    pub note: Option<String>,
}

/// Context passed to the `EnvKeys` hook.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvKeysHookCtx {
    pub main: InstalledPackageItem,
    pub path: String,
    pub sdk_info: BTreeMap<String, InstalledPackageItem>,
    pub runtime_version: &'static str,
}

/// A single environment variable emitted by the `EnvKeys` hook.
#[derive(Debug, Deserialize)]
pub struct EnvKeysHookResultItem {
    pub key: String,
    pub value: String,
}

/// Context passed to the optional `PostInstall` hook.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostInstallHookCtx {
    pub root_path: String,
    pub sdk_info: BTreeMap<String, InstalledPackageItem>,
    pub runtime_version: &'static str,
}

/// Context passed to the optional `PreUse` hook.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreUseHookCtx {
    pub cwd: String,
    pub scope: String,
    pub version: String,
    #[serde(default)]
    pub previous_version: Option<String>,
    pub installed_sdks: BTreeMap<String, InstalledPackageItem>,
    pub runtime_version: &'static str,
}

/// Result returned by the optional `PreUse` hook.
#[derive(Debug, Deserialize)]
pub struct PreUseHookResult {
    pub version: String,
}

/// Builds the name-keyed package map exposed to Lua hooks.
pub fn build_installed_package_map(
    runtime: &InstalledRuntime,
) -> BTreeMap<String, InstalledPackageItem> {
    let mut packages = BTreeMap::new();
    packages.insert(
        runtime.main.name.clone(),
        InstalledPackageItem {
            path: runtime.main.path.display().to_string(),
            version: runtime.main.version.clone(),
            name: runtime.main.name.clone(),
            note: runtime.main.note.clone(),
        },
    );
    for addition in &runtime.additions {
        packages.insert(
            addition.name.clone(),
            InstalledPackageItem {
                path: addition.path.display().to_string(),
                version: addition.version.clone(),
                name: addition.name.clone(),
                note: addition.note.clone(),
            },
        );
    }
    packages
}

/// Converts deserialized Lua metadata into a [`PluginManifest`].
pub fn build_manifest(metadata: MetadataFile, source: &std::path::Path) -> PluginManifest {
    PluginManifest {
        name: metadata.name,
        backend: vs_plugin_api::PluginBackendKind::Lua,
        source: source.to_path_buf(),
        description: metadata.description,
        aliases: metadata.aliases,
        version: metadata.version,
        homepage: metadata.homepage,
        update_url: metadata.update_url,
        manifest_url: metadata.manifest_url,
        min_runtime_version: metadata.min_runtime_version,
        notes: metadata.notes,
        legacy_filenames: metadata.legacy_filenames,
    }
}
