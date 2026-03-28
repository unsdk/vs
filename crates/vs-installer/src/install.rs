use std::fs;
use std::path::{Path, PathBuf};

use tempfile::Builder;
use vs_plugin_api::InstallPlan;

use crate::InstallerError;
use crate::fs::copy_dir_all;
use crate::receipt::InstallReceipt;

/// Handles transactional runtime installs.
#[derive(Debug, Clone)]
pub struct Installer {
    home: PathBuf,
}

impl Installer {
    /// Creates a new installer rooted at the active home.
    pub fn new(home: impl Into<PathBuf>) -> Self {
        Self { home: home.into() }
    }

    fn versions_root(&self, plugin: &str) -> PathBuf {
        self.home.join("cache").join(plugin).join("versions")
    }

    fn receipt_path(install_dir: &Path) -> PathBuf {
        install_dir.join(".vs-receipt.json")
    }

    /// Returns the final install directory for a plugin version.
    pub fn install_dir(&self, plugin: &str, version: &str) -> PathBuf {
        self.versions_root(plugin).join(version)
    }

    /// Lists installed versions for a plugin.
    pub fn installed_versions(&self, plugin: &str) -> Result<Vec<String>, InstallerError> {
        let root = self.versions_root(plugin);
        if !root.exists() {
            return Ok(Vec::new());
        }
        let mut versions = fs::read_dir(root)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let file_type = entry.file_type().ok()?;
                if file_type.is_dir() {
                    entry.file_name().into_string().ok()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        versions.sort();
        Ok(versions)
    }

    /// Installs a version using a staging directory and atomic rename.
    pub fn install(&self, plan: &InstallPlan) -> Result<PathBuf, InstallerError> {
        if !plan.source_dir.exists() {
            return Err(InstallerError::MissingSource(plan.source_dir.clone()));
        }

        let destination = self.install_dir(&plan.plugin, &plan.version);
        if destination.exists() {
            return Ok(destination);
        }

        let staging_root = self.home.join("cache").join(&plan.plugin).join(".staging");
        fs::create_dir_all(&staging_root)?;
        let temp_dir = Builder::new().prefix("install-").tempdir_in(staging_root)?;
        let staged_install = temp_dir.path().join("runtime");

        copy_dir_all(&plan.source_dir, &staged_install)?;
        self.validate_staged_install(&staged_install)?;
        self.write_receipt(
            &staged_install,
            &InstallReceipt {
                plugin: plan.plugin.clone(),
                version: plan.version.clone(),
                source: plan.source_dir.display().to_string(),
            },
        )?;

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&staged_install, &destination)?;
        Ok(destination)
    }

    /// Uninstalls a version from the local cache.
    pub fn uninstall(&self, plugin: &str, version: &str) -> Result<bool, InstallerError> {
        let path = self.install_dir(plugin, version);
        if !path.exists() {
            return Ok(false);
        }
        fs::remove_dir_all(path)?;
        Ok(true)
    }

    /// Reads the install receipt for a version.
    pub fn read_receipt(
        &self,
        plugin: &str,
        version: &str,
    ) -> Result<Option<InstallReceipt>, InstallerError> {
        let path = Self::receipt_path(&self.install_dir(plugin, version));
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path)?;
        let receipt = serde_json::from_str(&content).map_err(|error| InstallerError::Json {
            path,
            message: error.to_string(),
        })?;
        Ok(Some(receipt))
    }

    fn validate_staged_install(&self, staged_install: &Path) -> Result<(), InstallerError> {
        if staged_install.join(".vs-fail-install").exists() {
            return Err(InstallerError::Validation(String::from(
                "staged runtime requested a simulated install failure",
            )));
        }
        Ok(())
    }

    fn write_receipt(
        &self,
        install_dir: &Path,
        receipt: &InstallReceipt,
    ) -> Result<(), InstallerError> {
        let path = Self::receipt_path(install_dir);
        let rendered =
            serde_json::to_string_pretty(receipt).map_err(|error| InstallerError::Json {
                path: path.clone(),
                message: error.to_string(),
            })?;
        fs::write(path, rendered)?;
        Ok(())
    }
}
