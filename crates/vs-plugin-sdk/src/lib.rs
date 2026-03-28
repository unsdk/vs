//! Helper types for native `vs` plugins.

pub use vs_plugin_api::{AvailableVersion, EnvKey, InstallPlan, PluginBackendKind, PluginManifest};

/// Describes a native plugin contract in a strongly typed way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePluginContract {
    /// Static plugin metadata.
    pub manifest: PluginManifest,
    /// Versions the plugin can expose.
    pub versions: Vec<AvailableVersion>,
}
