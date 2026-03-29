use clap::Args;

/// Update specified plugin, use --all/-a to update all installed plugins.
#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// Update all added plugins.
    #[arg(short = 'a', long)]
    pub all: bool,
    /// Plugin name.
    pub plugin: Option<String>,
}
