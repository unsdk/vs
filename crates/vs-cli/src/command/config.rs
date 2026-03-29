use clap::Args;

/// Setup, view config.
#[derive(Debug, Args)]
pub struct ConfigArgs {
    /// List config values.
    #[arg(short = 'l', long)]
    pub list: bool,
    /// Unset a config value.
    #[arg(long, visible_alias = "un")]
    pub unset: bool,
    /// Config key.
    pub key: Option<String>,
    /// Config value.
    pub value: Option<String>,
}
