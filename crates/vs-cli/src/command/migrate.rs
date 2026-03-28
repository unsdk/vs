use clap::Args;

/// Migrates state from an old `vfox` home.
#[derive(Debug, Args)]
pub struct MigrateArgs {
    /// Optional source home directory.
    #[arg(long)]
    pub source: Option<String>,
}
