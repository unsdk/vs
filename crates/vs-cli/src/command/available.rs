//! Argument definitions for the `vs available` subcommand.

use clap::Args;

/// Lists plugins available from the registry index.
#[derive(Debug, Args)]
pub struct AvailableArgs;
