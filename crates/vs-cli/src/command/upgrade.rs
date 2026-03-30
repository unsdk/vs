//! Argument definitions for the `vs upgrade` subcommand.

use clap::Args;

/// Upgrade `vs` to the latest version.
#[derive(Debug, Args)]
pub struct UpgradeArgs {
    /// Skip the upgrade confirmation prompt.
    #[arg(short = 'y', long)]
    pub yes: bool,
}
