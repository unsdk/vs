use clap::Args;

/// Searches the available registry index.
#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Search query.
    pub query: String,
}
