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
    /// A download failed.
    #[error("artifact download failed: {0}")]
    Download(String),
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
    /// An archive could not be unpacked.
    #[error(transparent)]
    Archive(#[from] zip::result::ZipError),
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fs;

    use tempfile::TempDir;
    use vs_plugin_api::{InstallArtifact, InstallPlan, InstallSource};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

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
            main: InstallArtifact {
                name: String::from("nodejs"),
                version: String::from("20.11.1"),
                source: InstallSource::Directory { path: source },
                note: None,
                checksum: None,
            },
            additions: Vec::new(),
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

    #[test]
    fn install_should_preserve_flat_archive_layouts() -> Result<(), Box<dyn Error>> {
        let temp_dir = TempDir::new()?;
        let archive = write_zip(
            &temp_dir,
            "flat.zip",
            &[("bin/node", b"#!/bin/sh\necho node\n".as_slice())],
        )?;

        let installer = Installer::new(temp_dir.path().join("home"));
        let plan = install_plan_from_archive(&archive);
        let installed = installer.install(&plan)?;

        assert!(installed.main.path.join("bin/node").exists());
        Ok(())
    }

    #[test]
    fn install_should_collapse_single_wrapped_archive_root() -> Result<(), Box<dyn Error>> {
        let temp_dir = TempDir::new()?;
        let archive = write_zip(
            &temp_dir,
            "wrapped.zip",
            &[("package/bin/node", b"#!/bin/sh\necho node\n".as_slice())],
        )?;

        let installer = Installer::new(temp_dir.path().join("home"));
        let plan = install_plan_from_archive(&archive);
        let installed = installer.install(&plan)?;

        assert!(installed.main.path.join("bin/node").exists());
        assert!(!installed.main.path.join("package").exists());
        Ok(())
    }

    fn install_plan_from_archive(path: &std::path::Path) -> InstallPlan {
        InstallPlan {
            plugin: String::from("nodejs"),
            version: String::from("20.11.1"),
            main: InstallArtifact {
                name: String::from("nodejs"),
                version: String::from("20.11.1"),
                source: InstallSource::File {
                    path: path.to_path_buf(),
                },
                note: None,
                checksum: None,
            },
            additions: Vec::new(),
            legacy_filenames: Vec::new(),
        }
    }

    fn write_zip(
        temp_dir: &TempDir,
        file_name: &str,
        entries: &[(&str, &[u8])],
    ) -> Result<std::path::PathBuf, Box<dyn Error>> {
        let path = temp_dir.path().join(file_name);
        let file = fs::File::create(&path)?;
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        for (name, contents) in entries {
            zip.start_file(name, options)?;
            std::io::Write::write_all(&mut zip, contents)?;
        }

        zip.finish()?;
        Ok(path)
    }
}
