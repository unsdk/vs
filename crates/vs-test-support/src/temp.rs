use tempfile::{TempDir, tempdir};

/// Creates a fresh temporary workspace.
pub fn temp_workspace() -> TempDir {
    match tempdir() {
        Ok(directory) => directory,
        Err(error) => panic!("failed to create temporary workspace: {error}"),
    }
}
