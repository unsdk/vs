use std::path::{Path, PathBuf};

const PROJECT_FILES: [&str; 4] = [".vs.toml", "vs.toml", ".vfox.toml", "vfox.toml"];

/// Finds the nearest project config file, walking upwards from the current directory.
pub fn find_project_file(cwd: &Path) -> Option<PathBuf> {
    cwd.ancestors()
        .flat_map(|directory| {
            PROJECT_FILES
                .iter()
                .map(move |file_name| directory.join(file_name))
        })
        .find(|candidate| candidate.exists())
}

/// Returns the preferred project config path for writes.
pub fn preferred_project_file(cwd: &Path) -> PathBuf {
    cwd.join(".vs.toml")
}
