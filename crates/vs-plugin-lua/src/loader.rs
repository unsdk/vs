//! Lua plugin loading and hook invocation logic.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use mlua::{Function, Lua, LuaSerdeExt, MultiValue, Table, Value};
use vs_plugin_api::{
    Checksum, EnvKey, InstallArtifact, InstallPlan, InstallSource, InstalledRuntime,
    IntoPluginResult, Plugin, PluginError, PluginManifest,
};

use crate::model::{
    AvailableHookCtx, AvailableHookResultItem, EnvKeysHookCtx, EnvKeysHookResultItem, MetadataFile,
    ParseLegacyFileHookResult, PostInstallHookCtx, PreInstallAdditionItem, PreInstallHookCtx,
    PreInstallHookResult, PreUninstallHookCtx, PreUseHookCtx, PreUseHookResult,
    build_installed_package_map, build_manifest,
};
use crate::module::{register_builtin_modules, set_package_paths};

const VFOX_COMPAT_RUNTIME_VERSION: &str = "1.0.6";

/// Lua-backed plugin loaded from a source directory.
pub struct LuaPlugin {
    lua: Lua,
    plugin_table: Table,
    manifest: PluginManifest,
}

impl std::fmt::Debug for LuaPlugin {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LuaPlugin")
            .field("manifest", &self.manifest)
            .finish()
    }
}

impl LuaPlugin {
    /// Loads a plugin from a source directory.
    pub fn load(source: &Path) -> Result<Self, PluginError> {
        let lua = Lua::new();
        set_package_paths(&lua, source)?;

        let plugin_name = source
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("plugin");
        register_builtin_modules(&lua, &compute_user_agent(plugin_name, None))?;
        set_runtime_globals(&lua, source)?;
        load_plugin_scripts(&lua, source)?;

        let plugin_table: Table =
            lua.globals()
                .get("PLUGIN")
                .map_err(|error| PluginError::InvalidSource {
                    path: source.to_path_buf(),
                    message: error.to_string(),
                })?;
        let metadata = metadata_from_table(&plugin_table, source)?;
        let manifest = build_manifest(metadata, source);
        register_builtin_modules(
            &lua,
            &compute_user_agent(&manifest.name, manifest.version.as_deref()),
        )?;

        Ok(Self {
            lua,
            plugin_table,
            manifest,
        })
    }

    fn has_function(&self, name: &str) -> Result<bool, PluginError> {
        Ok(!matches!(
            self.plugin_table
                .get::<Value>(name)
                .into_plugin_result()?,
            Value::Nil
        ))
    }

    fn call_hook<T, R>(&self, hook_name: &str, ctx: &T) -> Result<R, PluginError>
    where
        T: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let result = self.call_hook_raw(hook_name, ctx)?;
        decode_hook_result(&self.lua, result)
    }

    fn call_hook_raw<T>(&self, hook_name: &str, ctx: &T) -> Result<MultiValue, PluginError>
    where
        T: serde::Serialize,
    {
        let function: Function = self
            .plugin_table
            .get(hook_name)
            .into_plugin_result()?;
        let ctx_value = self
            .lua
            .to_value(ctx)
            .into_plugin_result()?;
        function
            .call::<MultiValue>((self.plugin_table.clone(), ctx_value))
            .into_plugin_result()
    }
}

impl Plugin for LuaPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn available_versions(
        &self,
        args: &[String],
    ) -> Result<Vec<vs_plugin_api::AvailableVersion>, PluginError> {
        let results: Vec<AvailableHookResultItem> = self.call_hook(
            "Available",
            &AvailableHookCtx {
                args: args.to_vec(),
            },
        )?;
        Ok(results.into_iter().map(Into::into).collect())
    }

    fn install_plan(&self, version: &str) -> Result<InstallPlan, PluginError> {
        let result: PreInstallHookResult = self.call_hook(
            "PreInstall",
            &PreInstallHookCtx { version },
        )?;
        let resolved_version = result.version.unwrap_or_else(|| version.to_string());
        let main_name = result
            .name
            .clone()
            .unwrap_or_else(|| self.manifest.name.clone());
        let main_source = result
            .url
            .as_deref()
            .map(|source| parse_install_source(source, &result.headers, &self.manifest.source))
            .transpose()?
            .ok_or_else(|| PluginError::VersionNotFound {
                plugin: self.manifest.name.clone(),
                version: version.to_string(),
            })?;

        Ok(InstallPlan {
            plugin: self.manifest.name.clone(),
            version: resolved_version.clone(),
            main: InstallArtifact {
                name: main_name,
                version: resolved_version.clone(),
                source: main_source,
                note: result.note.clone(),
                checksum: build_checksum(
                    result.sha256.as_deref(),
                    result.sha512.as_deref(),
                    result.sha1.as_deref(),
                    result.md5.as_deref(),
                ),
            },
            additions: result
                .addition
                .into_iter()
                .map(|addition| {
                    build_addition_artifact(&resolved_version, addition, &self.manifest.source)
                })
                .collect::<Result<Vec<_>, _>>()?,
            legacy_filenames: self.manifest.legacy_filenames.clone(),
        })
    }

    fn post_install(&self, runtime: &InstalledRuntime) -> Result<(), PluginError> {
        if !self.has_function("PostInstall")? {
            return Ok(());
        }
        let sdk_info = build_installed_package_map(runtime);
        let _ = self.call_hook_raw(
            "PostInstall",
            &PostInstallHookCtx {
                root_path: runtime.root_dir.display().to_string(),
                sdk_info,
            },
        )?;
        Ok(())
    }

    fn env_keys(&self, runtime: &InstalledRuntime) -> Result<Vec<EnvKey>, PluginError> {
        let sdk_info = build_installed_package_map(runtime);
        let results: Vec<EnvKeysHookResultItem> = self.call_hook(
            "EnvKeys",
            &EnvKeysHookCtx {
                main: crate::model::InstalledPackageItem {
                    path: runtime.main.path.display().to_string(),
                    version: runtime.main.version.clone(),
                    name: runtime.main.name.clone(),
                    note: runtime.main.note.clone(),
                },
                path: runtime.main.path.display().to_string(),
                sdk_info,
            },
        )?;
        Ok(results
            .into_iter()
            .map(|item| EnvKey {
                key: item.key,
                value: item.value,
            })
            .collect())
    }

    fn pre_use(
        &self,
        requested_version: &str,
        scope: &str,
        cwd: &Path,
        previous_version: Option<&str>,
        installed: &[InstalledRuntime],
    ) -> Result<Option<String>, PluginError> {
        if !self.has_function("PreUse")? {
            return Ok(None);
        }
        let installed_sdks = installed
            .iter()
            .map(|runtime| {
                (
                    runtime.version.clone(),
                    crate::model::InstalledPackageItem {
                        path: runtime.main.path.display().to_string(),
                        version: runtime.main.version.clone(),
                        name: runtime.main.name.clone(),
                        note: runtime.main.note.clone(),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        let result: PreUseHookResult = self.call_hook(
            "PreUse",
            &PreUseHookCtx {
                cwd: cwd.display().to_string(),
                scope: scope.to_string(),
                version: requested_version.to_string(),
                previous_version: previous_version.map(ToString::to_string),
                installed_sdks,
            },
        )?;
        Ok(Some(result.version))
    }

    fn parse_legacy_file(
        &self,
        file_name: &str,
        file_path: &Path,
        content: &str,
        installed_versions: &[String],
    ) -> Result<Option<String>, PluginError> {
        if !self
            .manifest
            .legacy_filenames
            .iter()
            .any(|name| name == file_name)
        {
            return Ok(None);
        }

        if self.has_function("ParseLegacyFile")? {
            let function: Function = self
                .plugin_table
                .get("ParseLegacyFile")
                .into_plugin_result()?;

            let ctx = self
                .lua
                .create_table()
                .into_plugin_result()?;
            ctx.set("filepath", file_path.display().to_string())
                .into_plugin_result()?;
            ctx.set("filename", file_name.to_string())
                .into_plugin_result()?;

            let versions: Vec<String> = installed_versions.to_vec();
            let get_versions = self
                .lua
                .create_function(move |lua, ()| {
                    let table = lua.create_table()?;
                    for (i, v) in versions.iter().enumerate() {
                        table.set(i + 1, v.as_str())?;
                    }
                    Ok(table)
                })
                .into_plugin_result()?;
            ctx.set("getInstalledVersions", get_versions)
                .into_plugin_result()?;

            let result = function
                .call::<MultiValue>((self.plugin_table.clone(), ctx))
                .into_plugin_result()?;
            let result: ParseLegacyFileHookResult = decode_hook_result(&self.lua, result)?;
            let version = result.version.trim().to_string();
            return if version.is_empty() {
                Ok(None)
            } else {
                Ok(Some(version))
            };
        }

        let trimmed = content.trim();
        if trimmed.is_empty() {
            Ok(None)
        } else {
            Ok(Some(trimmed.to_string()))
        }
    }

    fn pre_uninstall(&self, runtime: &InstalledRuntime) -> Result<(), PluginError> {
        if !self.has_function("PreUninstall")? {
            return Ok(());
        }
        let sdk_info = build_installed_package_map(runtime);
        let main = crate::model::InstalledPackageItem {
            path: runtime.main.path.display().to_string(),
            version: runtime.main.version.clone(),
            name: runtime.main.name.clone(),
            note: runtime.main.note.clone(),
        };
        let _ = self.call_hook_raw(
            "PreUninstall",
            &PreUninstallHookCtx { main, sdk_info },
        )?;
        Ok(())
    }
}

fn decode_hook_result<T>(lua: &Lua, values: MultiValue) -> Result<T, PluginError>
where
    T: serde::de::DeserializeOwned,
{
    let mut iter = values.into_iter();
    let first = iter.next().unwrap_or(Value::Nil);
    let second = iter.next().unwrap_or(Value::Nil);
    if !matches!(second, Value::Nil) {
        let message = match second {
            Value::String(text) => text.to_string_lossy(),
            value => value.type_name().to_string(),
        };
        return Err(PluginError::Backend(message));
    }
    if matches!(first, Value::Nil) {
        return Err(PluginError::NoResultProvided);
    }
    lua.from_value(first)
        .into_plugin_result()
}

fn parse_install_source(
    source: &str,
    headers: &BTreeMap<String, String>,
    plugin_root: &Path,
) -> Result<InstallSource, PluginError> {
    if source.starts_with("http://") || source.starts_with("https://") {
        return Ok(InstallSource::Url {
            url: source.to_string(),
            headers: headers.clone(),
        });
    }
    let path = {
        let candidate = PathBuf::from(source);
        if candidate.is_absolute() {
            candidate
        } else {
            plugin_root.join(candidate)
        }
    };
    if path.is_dir() {
        Ok(InstallSource::Directory { path })
    } else {
        Ok(InstallSource::File { path })
    }
}

fn build_addition_artifact(
    resolved_version: &str,
    addition: PreInstallAdditionItem,
    plugin_root: &Path,
) -> Result<InstallArtifact, PluginError> {
    let source = addition
        .url
        .as_deref()
        .map(|source| parse_install_source(source, &addition.headers, plugin_root))
        .transpose()?
        .ok_or_else(|| {
            PluginError::Backend(format!(
                "additional artifact {} does not provide a source",
                addition.name
            ))
        })?;
    Ok(InstallArtifact {
        name: addition.name,
        version: addition
            .version
            .unwrap_or_else(|| resolved_version.to_string()),
        source,
        note: addition.note,
        checksum: build_checksum(
            addition.sha256.as_deref(),
            addition.sha512.as_deref(),
            addition.sha1.as_deref(),
            addition.md5.as_deref(),
        ),
    })
}

fn build_checksum(
    sha256: Option<&str>,
    sha512: Option<&str>,
    sha1: Option<&str>,
    md5: Option<&str>,
) -> Option<Checksum> {
    sha256
        .map(|value| Checksum {
            algorithm: String::from("sha256"),
            value: value.to_string(),
        })
        .or_else(|| {
            md5.map(|value| Checksum {
                algorithm: String::from("md5"),
                value: value.to_string(),
            })
        })
        .or_else(|| {
            sha1.map(|value| Checksum {
                algorithm: String::from("sha1"),
                value: value.to_string(),
            })
        })
        .or_else(|| {
            sha512.map(|value| Checksum {
                algorithm: String::from("sha512"),
                value: value.to_string(),
            })
        })
}

fn set_runtime_globals(lua: &Lua, source: &Path) -> Result<(), PluginError> {
    let globals = lua.globals();
    globals
        .set("OS_TYPE", runtime_os_type())
        .into_plugin_result()?;
    globals
        .set("ARCH_TYPE", runtime_arch_type())
        .into_plugin_result()?;

    let runtime = lua
        .create_table()
        .into_plugin_result()?;
    runtime
        .set("osType", runtime_os_type())
        .into_plugin_result()?;
    runtime
        .set("archType", runtime_arch_type())
        .into_plugin_result()?;
    runtime
        .set("version", VFOX_COMPAT_RUNTIME_VERSION)
        .into_plugin_result()?;
    runtime
        .set("pluginDirPath", source.display().to_string())
        .into_plugin_result()?;
    globals
        .set("RUNTIME", runtime)
        .into_plugin_result()
}

fn load_plugin_scripts(lua: &Lua, source: &Path) -> Result<(), PluginError> {
    let main_path = source.join("main.lua");
    if main_path.exists() {
        return exec_lua_file(lua, &main_path);
    }

    let metadata_path = source.join("metadata.lua");
    if !metadata_path.exists() {
        return Err(PluginError::InvalidSource {
            path: metadata_path,
            message: String::from("metadata.lua not found"),
        });
    }
    exec_lua_file(lua, &metadata_path)?;

    for hook in [
        "available",
        "pre_install",
        "env_keys",
        "post_install",
        "pre_use",
        "parse_legacy_file",
        "pre_uninstall",
    ] {
        let path = source.join("hooks").join(format!("{hook}.lua"));
        if path.exists() {
            exec_lua_file(lua, &path)?;
        }
    }
    Ok(())
}

fn exec_lua_file(lua: &Lua, path: &Path) -> Result<(), PluginError> {
    let content = fs::read_to_string(path).map_err(|error| PluginError::InvalidSource {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    lua.load(&content)
        .set_name(path.display().to_string())
        .exec()
        .map_err(|error| PluginError::InvalidSource {
            path: path.to_path_buf(),
            message: error.to_string(),
        })
}

fn runtime_os_type() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        other => other,
    }
}

fn runtime_arch_type() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => other,
    }
}

fn compute_user_agent(plugin_name: &str, plugin_version: Option<&str>) -> String {
    let mut components = vec![format!("vfox/{VFOX_COMPAT_RUNTIME_VERSION}")];
    if let Some(version) = plugin_version {
        components.push(format!("vfox-{plugin_name}/{version}"));
    } else {
        components.push(format!("vfox-{plugin_name}"));
    }
    components.join(" ")
}

fn metadata_from_table(plugin_table: &Table, source: &Path) -> Result<MetadataFile, PluginError> {
    Ok(MetadataFile {
        name: plugin_table
            .get("name")
            .map_err(|error| PluginError::InvalidSource {
                path: source.to_path_buf(),
                message: error.to_string(),
            })?,
        version: plugin_table.get("version").ok(),
        description: plugin_table.get("description").ok(),
        aliases: plugin_table.get("aliases").unwrap_or_default(),
        homepage: plugin_table.get("homepage").ok(),
        license: plugin_table.get("license").ok(),
        update_url: plugin_table
            .get("updateUrl")
            .ok()
            .or_else(|| plugin_table.get("update_url").ok()),
        manifest_url: plugin_table
            .get("manifestUrl")
            .ok()
            .or_else(|| plugin_table.get("manifest_url").ok()),
        min_runtime_version: plugin_table
            .get("minRuntimeVersion")
            .ok()
            .or_else(|| plugin_table.get("min_runtime_version").ok()),
        notes: plugin_table.get("notes").unwrap_or_default(),
        legacy_filenames: plugin_table
            .get("legacyFilenames")
            .ok()
            .or_else(|| plugin_table.get("legacy_filenames").ok())
            .unwrap_or_default(),
    })
}

/// Returns the path where a Lua plugin should store helper modules.
pub fn lua_library_dir(root: &Path) -> PathBuf {
    root.join("lib")
}
