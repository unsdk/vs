//! Argument definitions for the `vs unuse` subcommand.

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
    /// Remove the session pin (default).
    #[arg(short = 's', long, conflicts_with_all = ["global", "project"])]
    pub session: bool,
}

impl UnuseArgs {
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
    use super::UnuseArgs;
    use vs_core::UseScope;

    #[test]
    fn scope_should_default_to_session() {
        let args = UnuseArgs {
            plugin: String::from("nodejs"),
            global: false,
            project: false,
            session: false,
        };

        assert_eq!(args.scope(), UseScope::Session);
    }
}
