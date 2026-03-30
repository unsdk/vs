//! Argument definitions for the `vs add` subcommand.

use clap::{Args, ValueEnum};
use vs_plugin_api::PluginBackendKind;

/// Add a plugin or plugins.
#[derive(Debug, Args)]
pub struct AddArgs {
    /// Plugin name or names.
    pub names: Vec<String>,
    /// Plugin source path.
    #[arg(short = 's', long)]
    pub source: Option<String>,
    /// Plugin alias used as the local command name.
    #[arg(long)]
    pub alias: Option<String>,
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
