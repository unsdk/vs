//! Argument definitions for the `vs cd` subcommand.

use clap::Args;

/// Launches a shell in `VS_HOME`, the plugin source directory, or the active runtime directory.
#[derive(Debug, Args)]
pub struct CdArgs {
    /// Plugin name.
    pub plugin: Option<String>,
    /// Print the plugin source directory instead of the active runtime directory.
    #[arg(short = 'p', long = "plugin")]
    pub plugin_dir: bool,
}
