//! Error types shared by plugin backends and hosts.

use std::path::PathBuf;

use thiserror::Error;

/// Errors shared by plugin backends.
#[derive(Debug, Error)]
pub enum PluginError {
    /// The plugin source could not be parsed.
    #[error("failed to parse plugin source at {path}: {message}")]
    InvalidSource { path: PathBuf, message: String },
    /// The requested plugin version is not exposed by the plugin.
    #[error("plugin {plugin} does not expose version {version}")]
    VersionNotFound { plugin: String, version: String },
    /// The plugin did not provide a result for the requested hook.
    #[error("plugin did not provide a result")]
    NoResultProvided,
    /// The backend hit an execution error.
    #[error("plugin backend error: {0}")]
    Backend(String),
}

/// Shared result type for plugin operations.
pub type PluginResult<T> = Result<T, PluginError>;

/// Extension trait to convert any `Display` error into [`PluginError::Backend`].
pub trait IntoPluginResult<T> {
    fn into_plugin_result(self) -> Result<T, PluginError>;
}

impl<T, E: std::fmt::Display> IntoPluginResult<T> for Result<T, E> {
    fn into_plugin_result(self) -> Result<T, PluginError> {
        self.map_err(|error| PluginError::Backend(error.to_string()))
    }
}
