use clap::Args;

/// Prints the active runtime directory for a tool.
#[derive(Debug, Args)]
pub struct CdArgs {
    /// Plugin name.
    pub plugin: String,
}
