//! Argument definitions for the `vs use` subcommand.

use clap::Args;
use vs_core::UseScope;

/// Activates a tool version for a selected scope.
#[derive(Debug, Args)]
pub struct UseArgs {
    /// Tool spec in the form `plugin` or `plugin@version`.
    pub spec: String,
    /// Write the version globally.
    #[arg(short = 'g', long, conflicts_with_all = ["project", "session"])]
    pub global: bool,
    /// Write the version to the current project.
    #[arg(short = 'p', long, conflicts_with_all = ["global", "session"])]
    pub project: bool,
    /// Write the version to the active session (default).
    #[arg(short = 's', long, conflicts_with_all = ["global", "project"])]
    pub session: bool,
    /// Avoid creating the project link when using project scope.
    #[arg(long)]
    pub unlink: bool,
}

impl UseArgs {
    /// Resolves the selected scope.
    pub fn scope(&self) -> UseScope {
        if self.global {
            UseScope::Global
        } else if self.project {
            UseScope::Project
        } else {
            UseScope::Session
        }
    }
}

#[cfg(test)]
mod tests {
    use super::UseArgs;
    use vs_core::UseScope;

    #[test]
    fn scope_should_default_to_session() {
        let args = UseArgs {
            spec: String::from("nodejs"),
            global: false,
            project: false,
            session: false,
            unlink: false,
        };

        assert_eq!(args.scope(), UseScope::Session);
    }
}
