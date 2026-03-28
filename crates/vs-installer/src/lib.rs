//! Transactional runtime installs for `vs`.

mod fs;
mod install;
mod receipt;

use std::path::PathBuf;

use thiserror::Error;

pub use install::Installer;
pub use receipt::InstallReceipt;

/// Errors returned by installer services.
#[derive(Debug, Error)]
pub enum InstallerError {
    /// An I/O operation failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A directory walk failed.
    #[error("failed to walk directory tree: {0}")]
    Walk(String),
    /// The source directory was not found.
    #[error("install source does not exist: {0}")]
    MissingSource(PathBuf),
    /// JSON data could not be parsed.
    #[error("failed to parse JSON file at {path}: {message}")]
    Json { path: PathBuf, message: String },
    /// The install validation step failed.
    #[error("install validation failed: {0}")]
    Validation(String),
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fs;

    use tempfile::TempDir;
    use vs_plugin_api::InstallPlan;

    use super::Installer;

    #[test]
    fn install_should_rollback_when_validation_fails() -> Result<(), Box<dyn Error>> {
        let temp_dir = TempDir::new()?;
        let source = temp_dir.path().join("source");
        fs::create_dir_all(&source)?;
        fs::write(source.join(".vs-fail-install"), "")?;

        let installer = Installer::new(temp_dir.path().join("home"));
        let plan = InstallPlan {
            plugin: String::from("nodejs"),
            version: String::from("20.11.1"),
            source_dir: source,
            legacy_filenames: Vec::new(),
        };

        let error = match installer.install(&plan) {
            Ok(_) => {
                return Err(Box::new(std::io::Error::other(
                    "install unexpectedly succeeded",
                )));
            }
            Err(error) => error,
        };
        assert!(error.to_string().contains("validation failed"));
        assert!(!installer.install_dir("nodejs", "20.11.1").exists());
        Ok(())
    }
}
