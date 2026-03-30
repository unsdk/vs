//! Argument definitions for the `vs exec` subcommand.

use clap::Args;

/// Executes a command with the resolved runtime environment.
#[derive(Debug, Args)]
pub struct ExecArgs {
    /// Tool spec in the form `plugin` or `plugin@version`.
    pub spec: String,
    /// Command to execute.
    pub command: String,
    /// Remaining command arguments.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}
