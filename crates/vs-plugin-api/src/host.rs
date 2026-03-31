//! Host-facing traits implemented by plugin backends.

use std::path::Path;

use crate::error::PluginResult;
use crate::model::{AvailableVersion, EnvKey, InstallPlan, InstalledRuntime, PluginManifest};

/// Common runtime interface used by the application layer.
pub trait Plugin {
    /// Returns the immutable plugin manifest.
    fn manifest(&self) -> &PluginManifest;

    /// Lists versions that the plugin can install.
    fn available_versions(&self, args: &[String]) -> PluginResult<Vec<AvailableVersion>>;

    /// Builds an install plan for the requested version.
    fn install_plan(&self, version: &str) -> PluginResult<InstallPlan>;

    /// Runs optional post-install logic after artifacts are materialized.
    fn post_install(&self, _runtime: &InstalledRuntime) -> PluginResult<()> {
        Ok(())
    }

    /// Returns environment keys that should be exported for the installed version.
    fn env_keys(&self, runtime: &InstalledRuntime) -> PluginResult<Vec<EnvKey>>;

    /// Allows a plugin to map the requested version before activation.
    fn pre_use(
        &self,
        _requested_version: &str,
        _scope: &str,
        _cwd: &Path,
        _previous_version: Option<&str>,
        _installed: &[InstalledRuntime],
    ) -> PluginResult<Option<String>> {
        Ok(None)
    }

    /// Attempts to map a legacy file content to a version.
    fn parse_legacy_file(
        &self,
        file_name: &str,
        file_path: &Path,
        content: &str,
    ) -> PluginResult<Option<String>>;

    /// Runs optional pre-uninstall logic before a version is removed.
    fn pre_uninstall(&self, _runtime: &InstalledRuntime) -> PluginResult<()> {
        Ok(())
    }
}
