use clap::Args;

/// Removes a plugin from the local home.
#[derive(Debug, Args)]
pub struct RemoveArgs {
    /// Plugin name.
    pub name: String,
    /// Skip confirmation prompt.
    #[arg(short = 'y', long)]
    pub yes: bool,
}
