use std::path::PathBuf;

use thiserror::Error;

/// Errors returned by the application layer.
#[derive(Debug, Error)]
pub enum CoreError {
    /// A filesystem operation failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// An HTTP request failed.
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    /// An archive could not be extracted.
    #[error(transparent)]
    Archive(#[from] zip::result::ZipError),
    /// Configuration error.
    #[error(transparent)]
    Config(#[from] vs_config::ConfigError),
    /// Registry error.
    #[error(transparent)]
    Registry(#[from] vs_registry::RegistryError),
    /// Installer error.
    #[error(transparent)]
    Installer(#[from] vs_installer::InstallerError),
    /// Shell error.
    #[error(transparent)]
    Shell(#[from] vs_shell::ShellError),
    /// Plugin error.
    #[error(transparent)]
    Plugin(#[from] vs_plugin_api::PluginError),
    /// A required plugin is not registered.
    #[error("plugin {0} is not known to the registry or local home")]
    UnknownPlugin(String),
    /// A feature is unavailable in this build.
    #[error("{0}")]
    Unsupported(String),
    /// The current command requires a session id.
    #[error("session scope requires VS_SESSION_ID to be set")]
    MissingSessionId,
    /// A requested tool version is not currently active.
    #[error("tool {0} is not currently active")]
    InactiveTool(String),
    /// A command could not parse a registry source file.
    #[error("failed to parse registry source at {path}: {message}")]
    RegistrySource { path: PathBuf, message: String },
    /// An external process failed to execute.
    #[error("failed to execute command {command}: {message}")]
    CommandExecution { command: String, message: String },
    /// A migration source could not be found.
    #[error("no migration source home is available")]
    MissingMigrationSource,
    /// The selected backend is not compiled into this build.
    #[error(
        "plugin backend {backend} is not enabled in this build; rebuild with the `{feature}` feature"
    )]
    UnsupportedBackend {
        backend: &'static str,
        feature: &'static str,
    },
}
