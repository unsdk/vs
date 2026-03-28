//! Shared plugin types and runtime traits.

mod error;
mod host;
mod model;

pub use error::{PluginError, PluginResult};
pub use host::Plugin;
pub use model::{
    AvailableAddition, AvailableVersion, Checksum, EnvKey, InstallArtifact, InstallPlan,
    InstallSource, InstalledArtifact, InstalledRuntime, PluginBackendKind, PluginManifest,
};
