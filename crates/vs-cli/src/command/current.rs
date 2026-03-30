//! Argument definitions for the `vs current` subcommand.

use clap::Args;

/// Show current version of the target SDK.
#[derive(Debug, Args)]
pub struct CurrentArgs {
    /// Optional plugin name.
    pub plugin: Option<String>,
}
