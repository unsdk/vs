use clap::{Args, ValueEnum};
use vs_plugin_api::PluginBackendKind;

/// Adds a plugin to the local home.
#[derive(Debug, Args)]
pub struct AddArgs {
    /// Plugin name.
    pub name: String,
    /// Plugin source path.
    #[arg(long)]
    pub source: Option<String>,
    /// Backend type when adding from an explicit source.
    #[arg(long, value_enum)]
    pub backend: Option<BackendArg>,
}

/// CLI backend values.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum BackendArg {
    /// Lua-compatible plugin backend.
    Lua,
    /// Native plugin backend.
    Wasi,
}

impl From<BackendArg> for PluginBackendKind {
    fn from(value: BackendArg) -> Self {
        match value {
            BackendArg::Lua => Self::Lua,
            BackendArg::Wasi => Self::Wasi,
        }
    }
}
