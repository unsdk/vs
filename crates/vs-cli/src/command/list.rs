use clap::Args;

/// List all versions of the target SDK.
#[derive(Debug, Args)]
pub struct ListArgs {
    /// Optional plugin name.
    pub plugin: Option<String>,
}
