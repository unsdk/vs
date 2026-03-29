use clap::Args;

/// Show plugin info or SDK path.
#[derive(Debug, Args)]
pub struct InfoArgs {
    /// Plugin name or `plugin@version`.
    pub spec: String,
    /// Format the output using placeholders such as `{{.Name}}`.
    #[arg(short = 'f', long)]
    pub format: Option<String>,
}
