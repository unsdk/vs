//! The high-level application façade for coordinating core services.

use std::collections::BTreeSet;
use std::env::{join_paths, split_paths};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use vs_config::{
    AppConfig, HomeLayout, ResolvedToolVersion, Scope, ToolVersions, find_project_file,
    global_tools_file, preferred_project_file, read_app_config, read_tool_versions, resolve_home,
    session_tools_file, supported_legacy_files, write_tool_versions,
};
use vs_installer::{Installer, InstallerOptions};
use vs_plugin_api::{AvailableVersion, EnvKey, Plugin, PluginBackendKind};
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
    pub(crate) runtime_settings: RuntimeSettings,
    pub(crate) registry: RegistryService,
    pub(crate) installer: Installer,
    #[cfg(feature = "lua")]
    pub(crate) lua_backend: LuaBackend,
    #[cfg(feature = "wasi")]
    pub(crate) wasi_backend: WasiBackend,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeSettings {
    runtime_root: PathBuf,
    proxy_url: Option<String>,
    legacy_enabled: bool,
    legacy_strategy: String,
    available_hook_cache_ttl: Option<Duration>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct AvailableVersionsCacheEntry {
    cached_at_epoch_secs: u64,
    versions: Vec<AvailableVersion>,
}

impl RuntimeSettings {
    fn from_config(home: &Path, config: &AppConfig) -> Self {
        let runtime_root = normalize_runtime_root(home, &config.storage.sdk_path);
        let proxy_url = config
            .proxy
            .enable
            .then(|| config.proxy.url.trim())
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        let available_hook_cache_ttl = parse_duration_spec(&config.cache.available_hook_duration);

        Self {
            runtime_root,
            proxy_url,
            legacy_enabled: config.legacy_version_file.enable,
            legacy_strategy: normalize_legacy_strategy(&config.legacy_version_file.strategy),
            available_hook_cache_ttl,
        }
    }
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
        let config = read_app_config(&home_layout.active_home)?;
        let runtime_settings = RuntimeSettings::from_config(&home_layout.active_home, &config);
        let registry = RegistryService::new(home_layout.active_home.clone());
        let installer = Installer::with_options(
            home_layout.active_home.clone(),
            InstallerOptions {
                runtime_root: Some(runtime_settings.runtime_root.clone()),
                proxy_url: runtime_settings.proxy_url.clone(),
            },
        );
        let app = Self {
            home_layout,
            cwd,
            session_id,
            runtime_settings,
            registry,
            installer,
            #[cfg(feature = "lua")]
            lua_backend: LuaBackend::with_proxy(configured_proxy_url(&config)),
            #[cfg(feature = "wasi")]
            wasi_backend: WasiBackend,
        };
        app.ensure_home_layout()?;
        Ok(app)
    }

    pub(crate) fn home(&self) -> &Path {
        &self.home_layout.active_home
    }

    pub(crate) fn runtime_root(&self) -> &Path {
        &self.runtime_settings.runtime_root
    }

    pub(crate) fn proxy_url(&self) -> Option<&str> {
        self.runtime_settings.proxy_url.as_deref()
    }

    pub(crate) fn legacy_strategy(&self) -> &str {
        &self.runtime_settings.legacy_strategy
    }

    pub(crate) fn home_paths(&self) -> HomePaths {
        home_paths(self.home(), self.runtime_root())
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
        fs::create_dir_all(&layout.runtime_dir)?;
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
        let mut tools = self
            .collect_known_tool_names()?
            .into_iter()
            .map(|plugin| {
                self.resolve_configured_tool_version(&plugin)
                    .map(|resolved| {
                        resolved.map(|resolved| CurrentTool {
                            plugin: resolved.plugin,
                            version: resolved.version,
                            scope: resolved.scope,
                            source: resolved.source,
                        })
                    })
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        tools.sort_by(|left, right| left.plugin.cmp(&right.plugin));
        Ok(tools)
    }

    pub(crate) fn resolve_configured_tool_version(
        &self,
        plugin: &str,
    ) -> Result<Option<ResolvedToolVersion>, CoreError> {
        if let Some(resolved) = self.resolve_project_tool_version_internal(plugin)? {
            return Ok(Some(resolved));
        }

        if let Some(session_id) = self.session_id.as_deref() {
            let path = session_tools_file(self.home(), session_id);
            if path.exists() {
                let versions = read_tool_versions(&path)?;
                if let Some(version) = versions.tools.get(plugin) {
                    return Ok(Some(ResolvedToolVersion {
                        plugin: plugin.to_string(),
                        version: version.clone(),
                        scope: Scope::Session,
                        source: path,
                    }));
                }
            }
        }

        let path = global_tools_file(self.home());
        if path.exists() {
            let versions = read_tool_versions(&path)?;
            if let Some(version) = versions.tools.get(plugin) {
                return Ok(Some(ResolvedToolVersion {
                    plugin: plugin.to_string(),
                    version: version.clone(),
                    scope: Scope::Global,
                    source: path,
                }));
            }
        }

        Ok(None)
    }

    pub(crate) fn resolve_project_tool_version_internal(
        &self,
        plugin: &str,
    ) -> Result<Option<ResolvedToolVersion>, CoreError> {
        if let Some(path) = find_project_file(&self.cwd) {
            let versions = read_tool_versions(&path)?;
            if let Some(version) = versions.tools.get(plugin) {
                return Ok(Some(ResolvedToolVersion {
                    plugin: plugin.to_string(),
                    version: version.clone(),
                    scope: Scope::Project,
                    source: path,
                }));
            }
        }

        self.resolve_legacy_tool_version(plugin)
    }

    fn collect_known_tool_names(&self) -> Result<BTreeSet<String>, CoreError> {
        let mut names = BTreeSet::new();
        if let Some(path) = find_project_file(&self.cwd) {
            names.extend(read_tool_versions(&path)?.tools.into_keys());
        }

        if self.runtime_settings.legacy_enabled {
            names.extend(self.collect_generic_legacy_tool_names()?);
            names.extend(self.added_plugins()?.into_iter().map(|entry| entry.name));
            names.extend(
                self.list_installed_versions()?
                    .into_iter()
                    .map(|installed| installed.plugin),
            );
        }

        let session_path = self
            .session_id
            .as_deref()
            .map(|session_id| session_tools_file(self.home(), session_id));
        if let Some(path) = session_path.as_deref()
            && path.exists()
        {
            names.extend(read_tool_versions(path)?.tools.into_keys());
        }

        let global_path = global_tools_file(self.home());
        if global_path.exists() {
            names.extend(read_tool_versions(&global_path)?.tools.into_keys());
        }

        Ok(names)
    }

    fn collect_generic_legacy_tool_names(&self) -> Result<BTreeSet<String>, CoreError> {
        let mut names = BTreeSet::new();
        for directory in self.cwd.ancestors() {
            for file_name in supported_legacy_files() {
                let path = directory.join(file_name);
                if !path.exists() {
                    continue;
                }
                let content = fs::read_to_string(&path)?;
                names.extend(parse_generic_legacy_tool_names(file_name, &content));
            }
        }
        Ok(names)
    }

    fn resolve_legacy_tool_version(
        &self,
        plugin: &str,
    ) -> Result<Option<ResolvedToolVersion>, CoreError> {
        if !self.runtime_settings.legacy_enabled {
            return Ok(None);
        }

        let installed_versions = self
            .installed_versions_for_plugin(plugin)?
            .into_iter()
            .map(|installed| installed.version)
            .collect::<Vec<_>>();
        let plugin_impl = self.load_added_plugin_for_legacy(plugin)?;

        for directory in self.cwd.ancestors() {
            for file_name in legacy_candidate_file_names(plugin_impl.as_deref()) {
                let path = directory.join(&file_name);
                if !path.exists() {
                    continue;
                }
                let content = fs::read_to_string(&path)?;
                if let Some(version) = self.parse_legacy_tool_version(
                    plugin,
                    plugin_impl.as_deref(),
                    &file_name,
                    &path,
                    &content,
                    &installed_versions,
                )? {
                    return Ok(Some(ResolvedToolVersion {
                        plugin: plugin.to_string(),
                        version,
                        scope: Scope::Project,
                        source: path,
                    }));
                }
            }
        }

        Ok(None)
    }

    fn load_added_plugin_for_legacy(
        &self,
        plugin: &str,
    ) -> Result<Option<Box<dyn Plugin>>, CoreError> {
        let entry = self
            .added_plugins()?
            .into_iter()
            .find(|entry| entry.matches(plugin));
        match entry {
            Some(entry) => self.load_plugin(&entry).map(Some),
            None => Ok(None),
        }
    }

    fn parse_legacy_tool_version(
        &self,
        plugin_name: &str,
        plugin: Option<&dyn Plugin>,
        file_name: &str,
        file_path: &Path,
        content: &str,
        installed_versions: &[String],
    ) -> Result<Option<String>, CoreError> {
        if let Some(plugin) = plugin
            && let Some(version) = plugin.parse_legacy_file(
                file_name,
                file_path,
                content,
                installed_versions,
                self.legacy_strategy(),
            )?
        {
            return Ok(Some(version));
        }

        self.parse_generic_legacy_tool_version(plugin_name, file_name, content, installed_versions)
    }

    fn parse_generic_legacy_tool_version(
        &self,
        plugin_name: &str,
        file_name: &str,
        content: &str,
        installed_versions: &[String],
    ) -> Result<Option<String>, CoreError> {
        let parsed = match file_name {
            ".tool-versions" => parse_tool_versions_content(content)
                .remove(plugin_name)
                .filter(|value| !value.is_empty()),
            ".nvmrc" | ".node-version" if plugin_name == "nodejs" => {
                let version = content.trim();
                (!version.is_empty()).then(|| version.to_string())
            }
            ".sdkmanrc" => parse_sdkmanrc_content(content)
                .remove(plugin_name)
                .filter(|value| !value.is_empty()),
            _ => None,
        };

        let Some(parsed) = parsed else {
            return Ok(None);
        };

        match self.legacy_strategy() {
            "latest_installed" => {
                Ok(select_matching_version(&parsed, installed_versions).or(Some(parsed)))
            }
            "latest_available" => {
                let available = self
                    .cached_available_versions(plugin_name, &[])
                    .unwrap_or_default()
                    .into_iter()
                    .map(|version| version.version)
                    .collect::<Vec<_>>();
                Ok(select_matching_version(&parsed, &available).or(Some(parsed)))
            }
            _ => Ok(Some(parsed)),
        }
    }

    pub(crate) fn effective_runtime_dir(&self, current: &CurrentTool) -> PathBuf {
        match current.scope {
            Scope::Project => {
                let linked = project_sdk_dir(&self.cwd, &current.plugin);
                if linked.exists() {
                    linked
                } else {
                    install_dir(self.runtime_root(), &current.plugin, &current.version)
                }
            }
            Scope::Global => {
                let linked = global_current_dir(self.runtime_root(), &current.plugin);
                if linked.exists() {
                    linked
                } else {
                    install_dir(self.runtime_root(), &current.plugin, &current.version)
                }
            }
            Scope::Session | Scope::System => {
                install_dir(self.runtime_root(), &current.plugin, &current.version)
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
        let base_path = std::env::var_os("__VS_ORIG_PATH").or_else(|| std::env::var_os("PATH"));
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
        // On the very first call __VS_ORIG_PATH is not yet set.  Capture the
        // current (clean) PATH so we can freeze it as __VS_ORIG_PATH.
        let orig_path_needs_export = std::env::var_os("__VS_ORIG_PATH").is_none();
        let orig_path_value = std::env::var("__VS_ORIG_PATH")
            .or_else(|_| std::env::var("PATH"))
            .unwrap_or_default();

        let delta = self.build_env()?;
        let path_value = self.path_with_delta(&delta)?;
        let state_hash = compute_env_state_hash(&delta, &path_value);
        let prev_hash = std::env::var("__VS_STATE_HASH").unwrap_or_default();
        if !prev_hash.is_empty() && state_hash == prev_hash {
            return Ok(String::new());
        }

        // Determine which env-var keys the previous hook-env call exported so
        // that we can unset any that are no longer relevant (e.g. after leaving
        // a project directory).
        let prev_keys: Vec<String> = std::env::var("__VS_VARS")
            .unwrap_or_default()
            .split(':')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        Ok(render_shell_env_lines(
            shell,
            orig_path_needs_export,
            &orig_path_value,
            &prev_keys,
            &delta,
            &path_value,
            &state_hash,
        )
        .join("\n"))
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

    pub(crate) fn cached_available_versions(
        &self,
        plugin_name: &str,
        args: &[String],
    ) -> Result<Vec<AvailableVersion>, CoreError> {
        if let Some(versions) = self.read_available_versions_cache(plugin_name, args)? {
            return Ok(versions);
        }

        let entry = self.resolve_registry_entry(plugin_name)?;
        let plugin = self.load_plugin(&entry)?;
        let versions = plugin.available_versions(args)?;
        self.write_available_versions_cache(plugin_name, args, &versions)?;
        Ok(versions)
    }

    fn available_versions_cache_path(&self, plugin_name: &str, args: &[String]) -> PathBuf {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        plugin_name.hash(&mut hasher);
        args.hash(&mut hasher);
        let key = format!("{:x}", hasher.finish());
        self.home()
            .join("cache")
            .join("available-hooks")
            .join(plugin_name)
            .join(format!("{key}.json"))
    }

    fn read_available_versions_cache(
        &self,
        plugin_name: &str,
        args: &[String],
    ) -> Result<Option<Vec<AvailableVersion>>, CoreError> {
        let Some(ttl) = self.runtime_settings.available_hook_cache_ttl else {
            return Ok(None);
        };
        let path = self.available_versions_cache_path(plugin_name, args);
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path)?;
        let entry =
            serde_json::from_str::<AvailableVersionsCacheEntry>(&content).map_err(|error| {
                CoreError::Unsupported(format!("failed to parse available cache: {error}"))
            })?;
        let cached_at = UNIX_EPOCH + Duration::from_secs(entry.cached_at_epoch_secs);
        let is_fresh = SystemTime::now()
            .duration_since(cached_at)
            .map(|age| age <= ttl)
            .unwrap_or(false);
        if is_fresh {
            Ok(Some(entry.versions))
        } else {
            Ok(None)
        }
    }

    fn write_available_versions_cache(
        &self,
        plugin_name: &str,
        args: &[String],
        versions: &[AvailableVersion],
    ) -> Result<(), CoreError> {
        if self.runtime_settings.available_hook_cache_ttl.is_none() {
            return Ok(());
        }

        let path = self.available_versions_cache_path(plugin_name, args);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let cached_at_epoch_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let rendered = serde_json::to_string_pretty(&AvailableVersionsCacheEntry {
            cached_at_epoch_secs,
            versions: versions.to_vec(),
        })
        .map_err(|error| {
            CoreError::Unsupported(format!("failed to render available cache: {error}"))
        })?;
        fs::write(path, rendered)?;
        Ok(())
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

fn configured_proxy_url(config: &AppConfig) -> Option<String> {
    config
        .proxy
        .enable
        .then(|| config.proxy.url.trim())
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn normalize_runtime_root(home: &Path, configured_path: &str) -> PathBuf {
    let trimmed = configured_path.trim();
    if trimmed.is_empty() {
        return home.join("cache");
    }

    let path = PathBuf::from(trimmed);
    if path.is_absolute() {
        path
    } else {
        home.join(path)
    }
}

fn normalize_legacy_strategy(strategy: &str) -> String {
    match strategy {
        "latest_installed" | "latest_available" => strategy.to_string(),
        _ => String::from("specified"),
    }
}

fn parse_duration_spec(value: &str) -> Option<Duration> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "0" {
        return None;
    }

    let split_at = trimmed
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(trimmed.len());
    let (amount, unit) = trimmed.split_at(split_at);
    let amount = amount.parse::<u64>().ok()?;
    if amount == 0 {
        return None;
    }

    let seconds = match unit {
        "" | "s" => amount,
        "m" => amount.saturating_mul(60),
        "h" => amount.saturating_mul(60 * 60),
        "d" => amount.saturating_mul(60 * 60 * 24),
        _ => return None,
    };
    Some(Duration::from_secs(seconds))
}

fn legacy_candidate_file_names(plugin: Option<&dyn Plugin>) -> Vec<String> {
    let mut candidates = supported_legacy_files()
        .iter()
        .map(|file_name| (*file_name).to_string())
        .collect::<Vec<_>>();
    if let Some(plugin) = plugin {
        for file_name in &plugin.manifest().legacy_filenames {
            if !candidates.iter().any(|existing| existing == file_name) {
                candidates.push(file_name.clone());
            }
        }
    }
    candidates
}

fn parse_generic_legacy_tool_names(file_name: &str, content: &str) -> BTreeSet<String> {
    match file_name {
        ".tool-versions" => parse_tool_versions_content(content).into_keys().collect(),
        ".nvmrc" | ".node-version" => {
            let mut names = BTreeSet::new();
            if !content.trim().is_empty() {
                names.insert(String::from("nodejs"));
            }
            names
        }
        ".sdkmanrc" => parse_sdkmanrc_content(content).into_keys().collect(),
        _ => BTreeSet::new(),
    }
}

fn parse_tool_versions_content(content: &str) -> std::collections::BTreeMap<String, String> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            Some((parts.next()?.to_string(), parts.next()?.to_string()))
        })
        .collect()
}

fn parse_sdkmanrc_content(content: &str) -> std::collections::BTreeMap<String, String> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let (plugin, version) = line.split_once('=')?;
            Some((plugin.trim().to_string(), version.trim().to_string()))
        })
        .collect()
}

fn select_matching_version(selector: &str, candidates: &[String]) -> Option<String> {
    if candidates.is_empty() {
        return None;
    }
    let selector = selector.trim();
    if selector.is_empty() {
        return candidates.first().cloned();
    }

    if let Some(candidate) = candidates
        .iter()
        .find(|candidate| candidate.as_str() == selector)
    {
        return Some(candidate.clone());
    }

    let prefix = format!("{selector}.");
    candidates
        .iter()
        .find(|candidate| candidate.starts_with(&prefix))
        .cloned()
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

fn compute_env_state_hash(delta: &EnvDelta, path_value: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    path_value.hash(&mut hasher);
    delta.vars.hash(&mut hasher);
    delta.path_entries.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn render_shell_env_lines(
    shell: ShellKind,
    orig_path_needs_export: bool,
    orig_path_value: &str,
    prev_keys: &[String],
    delta: &EnvDelta,
    path_value: &str,
    state_hash: &str,
) -> Vec<String> {
    let new_keys: Vec<&str> = delta.vars.iter().map(|(key, _)| key.as_str()).collect();
    let stale_keys: Vec<&String> = prev_keys
        .iter()
        .filter(|key| !new_keys.contains(&key.as_str()))
        .collect();
    let new_keys_joined = new_keys.join(":");
    let mut lines = Vec::new();

    match shell {
        ShellKind::Bash | ShellKind::Zsh => {
            if orig_path_needs_export {
                lines.push(format!(
                    "export __VS_ORIG_PATH='{}'",
                    orig_path_value.replace('\'', "'\"'\"'")
                ));
            }
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
            if orig_path_needs_export {
                lines.push(format!(
                    "set -gx __VS_ORIG_PATH '{}'",
                    orig_path_value.replace('\'', "\\'")
                ));
            }
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
            if orig_path_needs_export {
                lines.push(serde_json::json!({ "__VS_ORIG_PATH": orig_path_value }).to_string());
            }
            for key in &stale_keys {
                lines.push(serde_json::json!({ "__VS_UNSET": key }).to_string());
            }
            for (key, value) in &delta.vars {
                lines.push(serde_json::json!({ key: value }).to_string());
            }
            lines.push(serde_json::json!({ "PATH": path_value }).to_string());
            lines.push(serde_json::json!({ "__VS_VARS": new_keys_joined }).to_string());
            lines.push(serde_json::json!({ "__VS_STATE_HASH": state_hash }).to_string());
        }
        ShellKind::Pwsh => {
            if orig_path_needs_export {
                lines.push(format!(
                    "$env:__VS_ORIG_PATH = '{}'",
                    orig_path_value.replace('\'', "''")
                ));
            }
            for key in &stale_keys {
                lines.push(format!(
                    "Remove-Item Env:\\{key} -ErrorAction SilentlyContinue"
                ));
            }
            for (key, value) in &delta.vars {
                lines.push(format!("$env:{key} = '{}'", value.replace('\'', "''")));
            }
            lines.push(format!("$env:PATH = '{}'", path_value.replace('\'', "''")));
            lines.push(format!("$env:__VS_VARS = '{new_keys_joined}'"));
            lines.push(format!("$env:__VS_STATE_HASH = '{state_hash}'"));
        }
        ShellKind::Clink => {
            if orig_path_needs_export {
                lines.push(format!("set __VS_ORIG_PATH={orig_path_value}"));
            }
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

    lines
}

#[cfg(test)]
mod tests {
    use vs_shell::{EnvDelta, ShellKind};

    use super::{compute_env_state_hash, render_shell_env_lines};

    #[test]
    fn env_state_hash_should_change_when_env_changes() {
        let first = compute_env_state_hash(
            &EnvDelta {
                vars: vec![(String::from("NODEJS_HOME"), String::from("/a"))],
                path_entries: Vec::new(),
            },
            "/a/bin:/usr/bin",
        );
        let second = compute_env_state_hash(
            &EnvDelta {
                vars: vec![(String::from("NODEJS_HOME"), String::from("/b"))],
                path_entries: Vec::new(),
            },
            "/b/bin:/usr/bin",
        );

        assert_ne!(first, second);
    }

    #[test]
    fn nushell_rendering_should_emit_unset_markers_for_stale_vars() {
        let lines = render_shell_env_lines(
            ShellKind::Nushell,
            false,
            "",
            &[String::from("OLD_HOME"), String::from("KEEP_HOME")],
            &EnvDelta {
                vars: vec![(String::from("KEEP_HOME"), String::from("/tool"))],
                path_entries: Vec::new(),
            },
            "/tool/bin:/usr/bin",
            "hash",
        );

        assert!(
            lines
                .iter()
                .any(|line| line.contains("\"__VS_UNSET\":\"OLD_HOME\""))
        );
        assert!(!lines.iter().any(|line| line.contains("\"OLD_HOME\":\"\"")));
    }
}
