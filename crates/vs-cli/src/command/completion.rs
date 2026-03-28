use clap::Args;

/// Generates a shell completion script.
#[derive(Debug, Args)]
pub struct CompletionArgs {
    /// Shell name.
    pub shell: String,
}
