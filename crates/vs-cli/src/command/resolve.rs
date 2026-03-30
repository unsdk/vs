//! Argument definitions for the internal `vs __resolve` command.

use clap::Args;

/// Hidden helper that resolves the active runtime path.
#[derive(Debug, Args)]
pub struct ResolveArgs {
    /// Plugin name.
    pub plugin: String,
}
