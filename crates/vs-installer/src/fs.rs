use std::fs;
use std::path::Path;

use walkdir::WalkDir;

use crate::InstallerError;

/// Recursively copies a directory tree.
pub fn copy_dir_all(source: &Path, destination: &Path) -> Result<(), InstallerError> {
    for entry in WalkDir::new(source) {
        let entry = entry.map_err(|error| InstallerError::Walk(error.to_string()))?;
        let path = entry.path();
        let relative = path
            .strip_prefix(source)
            .map_err(|error| InstallerError::Walk(error.to_string()))?;
        let target = destination.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(path, &target)?;
        }
    }
    Ok(())
}
