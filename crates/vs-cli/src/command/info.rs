use clap::Args;

/// Shows plugin metadata and known versions.
#[derive(Debug, Args)]
pub struct InfoArgs {
    /// Plugin name.
    pub name: String,
}
