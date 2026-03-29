use clap::Args;

/// Install a version of the target SDK.
#[derive(Debug, Args)]
pub struct InstallArgs {
    /// Tool specs in the form `plugin` or `plugin@version`.
    pub specs: Vec<String>,
    /// Install all configured tools.
    #[arg(short = 'a', long)]
    pub all: bool,
    /// Skip confirmation when auto-adding plugins or installing all configured tools.
    #[arg(short = 'y', long)]
    pub yes: bool,
}
