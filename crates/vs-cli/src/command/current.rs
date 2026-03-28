use clap::Args;

/// Shows the active version for one or more tools.
#[derive(Debug, Args)]
pub struct CurrentArgs {
    /// Optional plugin name.
    pub plugin: Option<String>,
}
