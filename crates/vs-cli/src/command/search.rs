//! Argument definitions for the `vs search` subcommand.

use clap::Args;

/// Searches available SDK versions for a plugin.
#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Plugin name.
    pub plugin: String,
    /// Additional arguments passed through to the plugin.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}
