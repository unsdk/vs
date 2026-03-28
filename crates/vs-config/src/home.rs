use std::path::PathBuf;

use dirs::home_dir;

use crate::ConfigError;
use crate::types::HomeLayout;

/// Resolves the active `vs` home directory from the process environment.
pub fn resolve_home() -> Result<HomeLayout, ConfigError> {
    resolve_home_with(
        std::env::var_os("VS_HOME").map(PathBuf::from),
        home_dir().ok_or(ConfigError::HomeDirectoryUnavailable)?,
    )
}

/// Resolves the active `vs` home directory using explicit inputs.
pub fn resolve_home_with(
    vs_home: Option<PathBuf>,
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
            migration_candidates: Vec::new(),
        });
    }

    Ok(HomeLayout {
        active_home: default_vs_home,
        migration_candidates: Vec::new(),
    })
}
