use clap::Args;

/// Installs the latest available version for a plugin.
#[derive(Debug, Args)]
pub struct UpgradeArgs {
    /// Plugin name.
    pub plugin: String,
}
