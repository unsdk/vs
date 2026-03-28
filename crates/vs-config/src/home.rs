use std::path::{Path, PathBuf};

use dirs::home_dir;

use crate::ConfigError;
use crate::types::HomeLayout;

fn existing_paths(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths.iter().filter(|path| path.exists()).cloned().collect()
}

/// Resolves the active `vs` home directory from the process environment.
pub fn resolve_home() -> Result<HomeLayout, ConfigError> {
    resolve_home_with(
        std::env::var_os("VS_HOME").map(PathBuf::from),
        std::env::var_os("VFOX_HOME").map(PathBuf::from),
        home_dir().ok_or(ConfigError::HomeDirectoryUnavailable)?,
    )
}

/// Resolves the active `vs` home directory using explicit inputs.
pub fn resolve_home_with(
    vs_home: Option<PathBuf>,
    vfox_home: Option<PathBuf>,
    user_home: PathBuf,
) -> Result<HomeLayout, ConfigError> {
    if let Some(path) = vs_home {
        return Ok(HomeLayout {
            active_home: path,
            migration_candidates: Vec::new(),
        });
    }

    let default_vs_home = user_home.join(".vs");
    if default_vs_home.exists() {
        return Ok(HomeLayout {
            active_home: default_vs_home,
            migration_candidates: existing_paths(&legacy_homes(&user_home, vfox_home.as_deref())),
        });
    }

    Ok(HomeLayout {
        active_home: default_vs_home,
        migration_candidates: existing_paths(&legacy_homes(&user_home, vfox_home.as_deref())),
    })
}

fn legacy_homes(user_home: &Path, vfox_home: Option<&Path>) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(path) = vfox_home {
        paths.push(path.to_path_buf());
    }
    paths.push(user_home.join(".vfox"));
    paths.push(user_home.join(".version-fox"));
    paths
}
