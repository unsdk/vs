use clap::Args;
use vs_core::UseScope;

/// Removes an active tool version from a selected scope.
#[derive(Debug, Args)]
pub struct UnuseArgs {
    /// Plugin name.
    pub plugin: String,
    /// Remove the global pin.
    #[arg(short = 'g', long, conflicts_with_all = ["project", "session"])]
    pub global: bool,
    /// Remove the project pin.
    #[arg(short = 'p', long, conflicts_with_all = ["global", "session"])]
    pub project: bool,
    /// Remove the session pin.
    #[arg(short = 's', long, conflicts_with_all = ["global", "project"])]
    pub session: bool,
}

impl UnuseArgs {
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
