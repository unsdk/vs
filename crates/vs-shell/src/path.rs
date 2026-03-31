//! Path helpers for locating runtime bins and scope-specific links.

use std::path::{Path, PathBuf};

/// Common filesystem paths used by `vs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomePaths {
    /// Root home directory.
    pub home: PathBuf,
    /// Registry state directory.
    pub registry_dir: PathBuf,
    /// Installed plugin metadata directory.
    pub plugins_dir: PathBuf,
    /// Home-local cache directory used for command metadata.
    pub cache_dir: PathBuf,
    /// Installed runtime directory.
    pub runtime_dir: PathBuf,
    /// Global shim directory.
    pub shims_dir: PathBuf,
    /// Session state directory.
    pub sessions_dir: PathBuf,
    /// Global scope state directory.
    pub global_dir: PathBuf,
}

/// Returns the canonical home layout.
pub fn home_paths(home: &Path, runtime_root: &Path) -> HomePaths {
    HomePaths {
        home: home.to_path_buf(),
        registry_dir: home.join("registry"),
        plugins_dir: home.join("plugins"),
        cache_dir: home.join("cache"),
        runtime_dir: runtime_root.to_path_buf(),
        shims_dir: home.join("shims"),
        sessions_dir: home.join("sessions"),
        global_dir: home.join("global"),
    }
}

/// Returns the directory where a plugin version is installed.
pub fn install_dir(runtime_root: &Path, plugin: &str, version: &str) -> PathBuf {
    runtime_root.join(plugin).join("versions").join(version)
}

/// Returns the global `current` directory path for a plugin.
pub fn global_current_dir(runtime_root: &Path, plugin: &str) -> PathBuf {
    runtime_root.join(plugin).join("current")
}

/// Returns the project-local SDK link directory.
pub fn project_sdk_dir(project_root: &Path, plugin: &str) -> PathBuf {
    project_root.join(".vs").join("sdks").join(plugin)
}

/// Returns the default binary directory inside an installed runtime.
pub fn bin_dir(install_dir: &Path) -> PathBuf {
    install_dir.join("bin")
}
