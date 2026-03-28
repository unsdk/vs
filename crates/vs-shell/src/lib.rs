//! Shell activation and path helpers for `vs`.

mod activate;
mod env;
mod link;
mod path;

use thiserror::Error;

pub use activate::{ShellKind, render_activation};
pub use env::EnvDelta;
pub use link::{link_directory, remove_existing};
pub use path::{HomePaths, bin_dir, global_current_dir, home_paths, install_dir, project_sdk_dir};

/// Errors returned by shell utilities.
#[derive(Debug, Error)]
pub enum ShellError {
    /// An I/O operation failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The shell name is unsupported.
    #[error("unknown shell: {0}")]
    UnknownShell(String),
}

#[cfg(test)]
mod tests {
    use super::{ShellKind, render_activation};

    #[test]
    fn activation_script_should_reference_hidden_hook_command() {
        let script = render_activation(ShellKind::Bash);
        assert!(script.contains("vs __hook-env bash"));
        assert!(script.contains("VS_SESSION_ID"));
    }
}
