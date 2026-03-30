use std::collections::BTreeSet;
use std::env::{join_paths, split_paths};
use std::fs;
use std::path::{Path, PathBuf};

use vs_config::{
    AppConfig, HomeLayout, Scope, ToolVersions, find_legacy_file, find_project_file,
    global_tools_file, preferred_project_file, read_app_config, read_legacy_versions,
    read_tool_versions, resolve_home, resolve_tool_version, session_tools_file,
    write_tool_versions,
};
use vs_installer::Installer;
use vs_plugin_api::{EnvKey, Plugin, PluginBackendKind};
#[cfg(feature = "lua")]
use vs_plugin_lua::LuaBackend;
#[cfg(feature = "wasi")]
use vs_plugin_wasi::WasiBackend;
use vs_registry::{RegistryEntry, RegistryService};
use vs_shell::{
    EnvDelta, HomePaths, ShellKind, bin_dir, global_current_dir, home_paths, install_dir,
    project_sdk_dir,
};

use crate::error::CoreError;
use crate::models::CurrentTool;
#[cfg(feature = "lua")]
use crate::registry_source::DEFAULT_VFOX_REGISTRY_SOURCE;

/// Top-level application orchestrator.
#[derive(Debug, Clone)]
pub struct App {
    pub(crate) home_layout: HomeLayout,
    pub(crate) cwd: PathBuf,
    pub(crate) session_id: Option<String>,
    pub(crate) registry: RegistryService,
    pub(crate) installer: Installer,
    #[cfg(feature = "lua")]
    pub(crate) lua_backend: LuaBackend,
    #[cfg(feature = "wasi")]
    pub(crate) wasi_backend: WasiBackend,
}

impl App {
    /// Creates an application from the process environment.
    pub fn from_env() -> Result<Self, CoreError> {
        let home_layout = resolve_home()?;
        let cwd = std::env::current_dir().map_err(vs_config::ConfigError::from)?;
        let session_id = std::env::var("VS_SESSION_ID").ok();
        Self::new(home_layout, cwd, session_id)
    }

    /// Creates an application with explicit paths.
    pub fn new(
        home_layout: HomeLayout,
        cwd: PathBuf,
        session_id: Option<String>,
    ) -> Result<Self, CoreError> {
        let registry = RegistryService::new(home_layout.active_home.clone());
        let installer = Installer::new(home_layout.active_home.clone());
        let app = Self {
            home_layout,
            cwd,
            session_id,
            registry,
            installer,
            #[cfg(feature = "lua")]
            lua_backend: LuaBackend,
            #[cfg(feature = "wasi")]
            wasi_backend: WasiBackend,
        };
        app.ensure_home_layout()?;
        Ok(app)
    }

    pub(crate) fn home(&self) -> &Path {
        &self.home_layout.active_home
    }

    pub(crate) fn home_paths(&self) -> HomePaths {
        home_paths(self.home())
    }

    pub(crate) fn app_config(&self) -> Result<AppConfig, CoreError> {
        let mut config = read_app_config(self.home())?;
        if config.registry.address.is_empty() {
            if let Some(default_source) = self.default_registry_source() {
                config.registry.address = default_source.to_string();
            }
        }
        Ok(config)
    }

    pub(crate) fn ensure_home_layout(&self) -> Result<(), CoreError> {
        let layout = self.home_paths();
        fs::create_dir_all(&layout.home)?;
        fs::create_dir_all(&layout.registry_dir)?;
        fs::create_dir_all(&layout.plugins_dir)?;
        fs::create_dir_all(layout.plugins_dir.join("sources"))?;
        fs::create_dir_all(&layout.cache_dir)?;
        fs::create_dir_all(layout.home.join("downloads"))?;
        fs::create_dir_all(&layout.shims_dir)?;
        fs::create_dir_all(&layout.sessions_dir)?;
        fs::create_dir_all(&layout.global_dir)?;
        Ok(())
    }

    pub(crate) fn normalize_source_path(&self, source: &str) -> PathBuf {
        let path = PathBuf::from(source);
        if path.is_absolute() {
            path
        } else {
            self.cwd.join(path)
        }
    }

    pub(crate) fn resolve_registry_entry(&self, name: &str) -> Result<RegistryEntry, CoreError> {
        if let Some(entry) = self
            .registry
            .added_plugins()?
            .into_iter()
            .find(|entry| entry.matches(name))
        {
            return Ok(entry);
        }

        self.refresh_registry_index_with_fallback()?;
        self.registry
            .available_plugins()?
            .into_iter()
            .find(|entry| entry.matches(name))
            .ok_or_else(|| CoreError::UnknownPlugin(name.to_string()))
    }

    pub(crate) fn refresh_registry_index_with_fallback(&self) -> Result<(), CoreError> {
        let config = self.app_config()?;
        if config.registry.address.is_empty() {
            return Ok(());
        }

        match self.update_registry() {
            Ok(_) => Ok(()),
            Err(error) => {
                if self.registry.available_plugins()?.is_empty() {
                    Err(error)
                } else {
                    Ok(())
                }
            }
        }
    }

    pub(crate) fn load_plugin(&self, entry: &RegistryEntry) -> Result<Box<dyn Plugin>, CoreError> {
        let entry = self.materialize_plugin_entry(entry)?;
        #[cfg(any(feature = "lua", feature = "wasi"))]
        let source = self.normalize_source_path(&entry.source);
        match entry.backend {
            PluginBackendKind::Lua => {
                #[cfg(feature = "lua")]
                {
                    self.lua_backend.load(&source).map_err(Into::into)
                }
                #[cfg(not(feature = "lua"))]
                {
                    Err(CoreError::UnsupportedBackend {
                        backend: "lua",
                        feature: "lua",
                    })
                }
            }
            PluginBackendKind::Wasi => {
                #[cfg(feature = "wasi")]
                {
                    self.wasi_backend.load(&source).map_err(Into::into)
                }
                #[cfg(not(feature = "wasi"))]
                {
                    Err(CoreError::UnsupportedBackend {
                        backend: "wasi",
                        feature: "wasi",
                    })
                }
            }
        }
    }

    pub(crate) fn ensure_backend_supported(
        &self,
        backend: PluginBackendKind,
    ) -> Result<(), CoreError> {
        match backend {
            PluginBackendKind::Lua => {
                #[cfg(feature = "lua")]
                {
                    Ok(())
                }
                #[cfg(not(feature = "lua"))]
                {
                    Err(CoreError::UnsupportedBackend {
                        backend: "lua",
                        feature: "lua",
                    })
                }
            }
            PluginBackendKind::Wasi => {
                #[cfg(feature = "wasi")]
                {
                    Ok(())
                }
                #[cfg(not(feature = "wasi"))]
                {
                    Err(CoreError::UnsupportedBackend {
                        backend: "wasi",
                        feature: "wasi",
                    })
                }
            }
        }
    }

    pub(crate) fn default_backend(&self) -> Result<PluginBackendKind, CoreError> {
        #[cfg(all(feature = "lua", feature = "wasi"))]
        {
            Ok(PluginBackendKind::Lua)
        }
        #[cfg(all(feature = "lua", not(feature = "wasi")))]
        {
            Ok(PluginBackendKind::Lua)
        }
        #[cfg(all(feature = "wasi", not(feature = "lua")))]
        {
            Ok(PluginBackendKind::Wasi)
        }
        #[cfg(not(any(feature = "lua", feature = "wasi")))]
        {
            Err(CoreError::Unsupported(String::from(
                "no plugin backend is enabled in this build",
            )))
        }
    }

    pub(crate) fn default_registry_source(&self) -> Option<&'static str> {
        #[cfg(feature = "lua")]
        {
            Some(DEFAULT_VFOX_REGISTRY_SOURCE)
        }
        #[cfg(not(feature = "lua"))]
        {
            None
        }
    }

    pub(crate) fn write_tool_assignment(
        &self,
        path: &Path,
        plugin: &str,
        version: Option<&str>,
    ) -> Result<(), CoreError> {
        let mut tools = if path.exists() {
            read_tool_versions(path)?
        } else {
            ToolVersions::default()
        };
        match version {
            Some(version) => {
                tools.tools.insert(plugin.to_string(), version.to_string());
            }
            None => {
                tools.tools.remove(plugin);
            }
        }
        write_tool_versions(path, &tools)?;
        Ok(())
    }

    pub(crate) fn collect_current_tools(&self) -> Result<Vec<CurrentTool>, CoreError> {
        let mut names = BTreeSet::new();
        if let Some(path) = find_project_file(&self.cwd) {
            names.extend(read_tool_versions(&path)?.tools.into_keys());
        }
        if let Some(path) = find_legacy_file(&self.cwd) {
            names.extend(read_legacy_versions(&path)?.tools.into_keys());
        }
        let session_path = self
            .session_id
            .as_deref()
            .map(|session_id| session_tools_file(self.home(), session_id));
        if let Some(path) = session_path.as_deref() {
            if path.exists() {
                names.extend(read_tool_versions(path)?.tools.into_keys());
            }
        }
        let global_path = global_tools_file(self.home());
        if global_path.exists() {
            names.extend(read_tool_versions(&global_path)?.tools.into_keys());
        }

        let mut tools = names
            .into_iter()
            .filter_map(|plugin| {
                resolve_tool_version(self.home(), &self.cwd, self.session_id.as_deref(), &plugin)
                    .transpose()
                    .map(|resolved| {
                        resolved.map(|resolved| CurrentTool {
                            plugin: resolved.plugin,
                            version: resolved.version,
                            scope: resolved.scope,
                            source: resolved.source,
                        })
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        tools.sort_by(|left, right| left.plugin.cmp(&right.plugin));
        Ok(tools)
    }

    pub(crate) fn effective_runtime_dir(&self, current: &CurrentTool) -> PathBuf {
        match current.scope {
            Scope::Project => {
                let linked = project_sdk_dir(&self.cwd, &current.plugin);
                if linked.exists() {
                    linked
                } else {
                    install_dir(self.home(), &current.plugin, &current.version)
                }
            }
            Scope::Global => {
                let linked = global_current_dir(self.home(), &current.plugin);
                if linked.exists() {
                    linked
                } else {
                    install_dir(self.home(), &current.plugin, &current.version)
                }
            }
            Scope::Session | Scope::System => {
                install_dir(self.home(), &current.plugin, &current.version)
            }
        }
    }

    pub(crate) fn load_installed_runtime(
        &self,
        plugin: &str,
        version: &str,
    ) -> Result<Option<vs_plugin_api::InstalledRuntime>, CoreError> {
        self.installer
            .read_receipt(plugin, version)
            .map_err(Into::into)
    }

    pub(crate) fn build_env(&self) -> Result<EnvDelta, CoreError> {
        let current_tools = self.collect_current_tools()?;
        let mut delta = EnvDelta::default();

        for tool in &current_tools {
            let runtime_dir = self.effective_runtime_dir(tool);
            if let Some(runtime) = self.load_installed_runtime(&tool.plugin, &tool.version)? {
                // Relocate the runtime so that env-keys point through the
                // scope-specific symlink (e.g. .vs/sdks/nodejs) instead of the
                // raw cache directory.
                let runtime = runtime.relocate(&runtime_dir);
                if let Ok(entry) = self.resolve_registry_entry(&tool.plugin) {
                    let plugin = self.load_plugin(&entry)?;
                    let env_keys = plugin.env_keys(&runtime)?;
                    apply_env_keys(&mut delta, env_keys);
                } else {
                    delta.path_entries.push(bin_dir(runtime.main_path()));
                }
            } else {
                delta.path_entries.push(bin_dir(&runtime_dir));
            }
        }

        Ok(delta)
    }

    pub(crate) fn path_with_delta(&self, delta: &EnvDelta) -> Result<String, CoreError> {
        let mut entries = delta.path_entries.clone();
        // Use the original, clean PATH saved by the activation script so that
        // previously-injected vs entries are not duplicated on each hook call.
        let base_path = std::env::var_os("__VS_ORIG_PATH")
            .or_else(|| std::env::var_os("PATH"));
        let existing_entries = base_path
            .map(|paths| split_paths(&paths).collect::<Vec<_>>())
            .unwrap_or_default();
        entries.extend(existing_entries);
        let joined = join_paths(entries).map_err(|error| {
            CoreError::Unsupported(format!("failed to join PATH entries: {error}"))
        })?;
        Ok(joined.to_string_lossy().into_owned())
    }

    pub(crate) fn render_hook_env(&self, shell: ShellKind) -> Result<String, CoreError> {
        // Fast path: if the state hash has not changed since the last call,
        // all environment variables are already correct in the shell.
        let state_hash = self.compute_state_hash();
        let prev_hash = std::env::var("__VS_STATE_HASH").unwrap_or_default();
        if !prev_hash.is_empty() && state_hash == prev_hash {
            return Ok(String::new());
        }

        let delta = self.build_env()?;
        let path_value = self.path_with_delta(&delta)?;

        // Determine which env-var keys the previous hook-env call exported so
        // that we can unset any that are no longer relevant (e.g. after leaving
        // a project directory).
        let prev_keys: Vec<String> = std::env::var("__VS_VARS")
            .unwrap_or_default()
            .split(':')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        let new_keys: Vec<&str> = delta.vars.iter().map(|(k, _)| k.as_str()).collect();
        let stale_keys: Vec<&String> = prev_keys
            .iter()
            .filter(|k| !new_keys.contains(&k.as_str()))
            .collect();
        let new_keys_joined = new_keys.join(":");

        let mut lines = Vec::new();

        match shell {
            ShellKind::Bash | ShellKind::Zsh => {
                for key in &stale_keys {
                    lines.push(format!("unset {key}"));
                }
                for (key, value) in &delta.vars {
                    lines.push(format!("export {key}='{}'", value.replace('\'', "'\"'\"'")));
                }
                lines.push(format!(
                    "export PATH='{}'",
                    path_value.replace('\'', "'\"'\"'")
                ));
                lines.push(format!("export __VS_VARS='{new_keys_joined}'"));
                lines.push(format!("export __VS_STATE_HASH='{state_hash}'"));
            }
            ShellKind::Fish => {
                for key in &stale_keys {
                    lines.push(format!("set -e {key}"));
                }
                for (key, value) in &delta.vars {
                    lines.push(format!("set -gx {key} '{}'", value.replace('\'', "\\'")));
                }
                lines.push(format!(
                    "set -gx PATH '{}'",
                    path_value.replace('\'', "\\'")
                ));
                lines.push(format!("set -gx __VS_VARS '{new_keys_joined}'"));
                lines.push(format!("set -gx __VS_STATE_HASH '{state_hash}'"));
            }
            ShellKind::Nushell => {
                for key in &stale_keys {
                    lines.push(serde_json::json!({ key.as_str(): "" }).to_string());
                }
                for (key, value) in &delta.vars {
                    let payload = serde_json::json!({ key: value });
                    lines.push(payload.to_string());
                }
                lines.push(serde_json::json!({ "PATH": path_value }).to_string());
                lines.push(serde_json::json!({ "__VS_VARS": new_keys_joined }).to_string());
                lines.push(serde_json::json!({ "__VS_STATE_HASH": state_hash }).to_string());
            }
            ShellKind::Pwsh => {
                for key in &stale_keys {
                    lines.push(format!("Remove-Item Env:\\{key} -ErrorAction SilentlyContinue"));
                }
                for (key, value) in &delta.vars {
                    lines.push(format!("$env:{key} = '{}'", value.replace('\'', "''")));
                }
                lines.push(format!("$env:PATH = '{}'", path_value.replace('\'', "''")));
                lines.push(format!("$env:__VS_VARS = '{new_keys_joined}'"));
                lines.push(format!("$env:__VS_STATE_HASH = '{state_hash}'"));
            }
            ShellKind::Clink => {
                for key in &stale_keys {
                    lines.push(format!("set {key}="));
                }
                for (key, value) in &delta.vars {
                    lines.push(format!("set {key}={value}"));
                }
                lines.push(format!("set PATH={path_value}"));
                lines.push(format!("set __VS_VARS={new_keys_joined}"));
                lines.push(format!("set __VS_STATE_HASH={state_hash}"));
            }
        }

        Ok(lines.join("\n"))
    }

    /// Computes a hash over the inputs that affect hook-env output.
    ///
    /// When this hash matches `__VS_STATE_HASH` from the shell environment,
    /// the hook can return immediately without recalculating.
    fn compute_state_hash(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // CWD path.
        self.cwd.hash(&mut hasher);

        // CWD directory mtime — changes when files are created/deleted in it,
        // catching new project config or legacy version files.
        hash_file_mtime(&mut hasher, &self.cwd);

        // Config file mtimes.
        hash_file_mtime(&mut hasher, &global_tools_file(self.home()));

        if let Some(path) = find_project_file(&self.cwd) {
            hash_file_mtime(&mut hasher, &path);
        }
        if let Some(path) = find_legacy_file(&self.cwd) {
            hash_file_mtime(&mut hasher, &path);
        }
        if let Some(session_id) = &self.session_id {
            let session_path = session_tools_file(self.home(), session_id);
            hash_file_mtime(&mut hasher, &session_path);
        }

        format!("{:x}", hasher.finish())
    }

    pub(crate) fn preferred_project_file(&self) -> PathBuf {
        preferred_project_file(&self.cwd)
    }

    pub(crate) fn session_file(&self) -> Result<PathBuf, CoreError> {
        let session_id = self
            .session_id
            .as_deref()
            .ok_or(CoreError::MissingSessionId)?;
        Ok(session_tools_file(self.home(), session_id))
    }

    pub(crate) fn copy_tree(&self, source: &Path, destination: &Path) -> Result<(), CoreError> {
        if !source.exists() {
            return Ok(());
        }
        for entry in walkdir::WalkDir::new(source) {
            let entry = entry.map_err(|error| CoreError::Unsupported(error.to_string()))?;
            let relative = entry
                .path()
                .strip_prefix(source)
                .map_err(|error| CoreError::Unsupported(error.to_string()))?;
            let target = destination.join(relative);
            if entry.file_type().is_dir() {
                fs::create_dir_all(&target)?;
            } else {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(entry.path(), &target)?;
            }
        }
        Ok(())
    }
}

fn apply_env_keys(delta: &mut EnvDelta, env_keys: Vec<EnvKey>) {
    for env_key in env_keys {
        if env_key.key == "PATH" {
            delta.path_entries.push(PathBuf::from(env_key.value));
        } else {
            delta.vars.push((env_key.key, env_key.value));
        }
    }
}

fn hash_file_mtime(hasher: &mut impl std::hash::Hasher, path: &Path) {
    use std::hash::Hash;
    if let Ok(meta) = fs::metadata(path) {
        if let Ok(mtime) = meta.modified() {
            if let Ok(duration) = mtime.duration_since(std::time::UNIX_EPOCH) {
                duration.as_nanos().hash(hasher);
            }
        }
    }
}
