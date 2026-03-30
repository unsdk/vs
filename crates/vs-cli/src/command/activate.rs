//! Argument definitions for the `vs activate` subcommand.

use clap::Args;

/// Renders a shell activation script.
#[derive(Debug, Args)]
pub struct ActivateArgs {
    /// Shell name.
    pub shell: String,
}
