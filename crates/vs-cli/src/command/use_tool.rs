use clap::Args;
use vs_core::UseScope;

/// Activates a tool version for a selected scope.
#[derive(Debug, Args)]
pub struct UseArgs {
    /// Tool spec in the form `plugin@version`.
    pub spec: String,
    /// Write the version globally.
    #[arg(short = 'g', long, conflicts_with_all = ["project", "session"])]
    pub global: bool,
    /// Write the version to the current project.
    #[arg(short = 'p', long, conflicts_with_all = ["global", "session"])]
    pub project: bool,
    /// Write the version to the active session.
    #[arg(short = 's', long, conflicts_with_all = ["global", "project"])]
    pub session: bool,
    /// Avoid creating the project link when using project scope.
    #[arg(long)]
    pub unlink: bool,
}

impl UseArgs {
    /// Resolves the selected scope.
    pub fn scope(&self) -> UseScope {
        if self.project {
            UseScope::Project
        } else if self.session {
            UseScope::Session
        } else {
            UseScope::Global
        }
    }
}
