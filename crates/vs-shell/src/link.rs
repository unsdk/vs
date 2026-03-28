use std::fs;
use std::path::Path;

use crate::ShellError;

/// Creates a fresh directory symlink.
pub fn link_directory(target: &Path, link: &Path) -> Result<(), ShellError> {
    if link.exists() || link.is_symlink() {
        remove_existing(link)?;
    }
    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent)?;
    }
    create_symlink(target, link)?;
    Ok(())
}

/// Removes a symlink or directory recursively.
pub fn remove_existing(path: &Path) -> Result<(), ShellError> {
    if !path.exists() && !path.is_symlink() {
        return Ok(());
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || metadata.is_file() {
        fs::remove_file(path)?;
    } else {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

#[cfg(unix)]
fn create_symlink(target: &Path, link: &Path) -> Result<(), ShellError> {
    std::os::unix::fs::symlink(target, link)?;
    Ok(())
}

#[cfg(windows)]
fn create_symlink(target: &Path, link: &Path) -> Result<(), ShellError> {
    std::os::windows::fs::symlink_dir(target, link)?;
    Ok(())
}
