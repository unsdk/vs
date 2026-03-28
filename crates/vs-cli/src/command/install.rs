use clap::Args;

/// Installs a plugin version.
#[derive(Debug, Args)]
pub struct InstallArgs {
    /// Tool spec in the form `plugin` or `plugin@version`.
    pub spec: String,
}
