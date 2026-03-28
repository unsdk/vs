use clap::Args;

/// Reads or mutates application config values.
#[derive(Debug, Args)]
pub struct ConfigArgs {
    /// List config values.
    #[arg(long)]
    pub list: bool,
    /// Unset a config value.
    #[arg(long)]
    pub unset: bool,
    /// Config key.
    pub key: Option<String>,
    /// Config value.
    pub value: Option<String>,
}
