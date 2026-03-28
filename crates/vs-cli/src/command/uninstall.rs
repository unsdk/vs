use clap::Args;

/// Uninstalls a plugin version.
#[derive(Debug, Args)]
pub struct UninstallArgs {
    /// Tool spec in the form `plugin@version`.
    pub spec: String,
}
