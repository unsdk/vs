//! Shared plugin types and runtime traits.

mod error;
mod host;
mod model;

pub use error::{PluginError, PluginResult};
pub use host::Plugin;
pub use model::{AvailableVersion, EnvKey, InstallPlan, PluginBackendKind, PluginManifest};
