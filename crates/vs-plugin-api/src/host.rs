use std::path::Path;

use crate::error::PluginResult;
use crate::model::{AvailableVersion, EnvKey, InstallPlan, PluginManifest};

/// Common runtime interface used by the application layer.
pub trait Plugin {
    /// Returns the immutable plugin manifest.
    fn manifest(&self) -> &PluginManifest;

    /// Lists versions that the plugin can install.
    fn available_versions(&self) -> PluginResult<Vec<AvailableVersion>>;

    /// Builds an install plan for the requested version.
    fn install_plan(&self, version: &str) -> PluginResult<InstallPlan>;

    /// Returns environment keys that should be exported for the installed version.
    fn env_keys(&self, version: &str, install_dir: &Path) -> PluginResult<Vec<EnvKey>>;

    /// Attempts to map a legacy file content to a version.
    fn parse_legacy_file(&self, file_name: &str, content: &str) -> PluginResult<Option<String>>;
}
