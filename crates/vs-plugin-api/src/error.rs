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
