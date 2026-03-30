//! Argument definitions for the `vs version` subcommand.

use clap::Args;

/// Prints the current binary version and build metadata.
#[derive(Debug, Args)]
pub struct VersionArgs;
