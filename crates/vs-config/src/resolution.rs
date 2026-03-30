//! Result types and helpers for resolving active tool versions.

use std::path::{Path, PathBuf};

use crate::ConfigError;
use crate::config_file::read_tool_versions;
use crate::legacy::{find_legacy_file, read_legacy_versions};
use crate::project::find_project_file;
use crate::types::{ResolvedToolVersion, Scope};

fn session_file(home: &Path, session_id: &str) -> PathBuf {
    home.join("sessions").join(format!("{session_id}.toml"))
}

fn global_file(home: &Path) -> PathBuf {
    home.join("global").join("tools.toml")
}

/// Returns the canonical global tools file path.
pub fn global_tools_file(home: &Path) -> PathBuf {
    global_file(home)
}

/// Returns the canonical session tools file path.
pub fn session_tools_file(home: &Path, session_id: &str) -> PathBuf {
    session_file(home, session_id)
}

/// Resolves a tool version using `Project > Session > Global > System` precedence.
pub fn resolve_tool_version(
    home: &Path,
    cwd: &Path,
    session_id: Option<&str>,
    plugin: &str,
) -> Result<Option<ResolvedToolVersion>, ConfigError> {
    if let Some(path) = find_project_file(cwd) {
        let versions = read_tool_versions(&path)?;
        if let Some(version) = versions.tools.get(plugin) {
            return Ok(Some(ResolvedToolVersion {
                plugin: plugin.to_string(),
                version: version.clone(),
                scope: Scope::Project,
                source: path,
            }));
        }
    }

    if let Some(path) = find_legacy_file(cwd) {
        let versions = read_legacy_versions(&path)?;
        if let Some(version) = versions.tools.get(plugin) {
            return Ok(Some(ResolvedToolVersion {
                plugin: plugin.to_string(),
                version: version.clone(),
                scope: Scope::Project,
                source: path,
            }));
        }
    }

    if let Some(session_id) = session_id {
        let path = session_file(home, session_id);
        if path.exists() {
            let versions = read_tool_versions(&path)?;
            if let Some(version) = versions.tools.get(plugin) {
                return Ok(Some(ResolvedToolVersion {
                    plugin: plugin.to_string(),
                    version: version.clone(),
                    scope: Scope::Session,
                    source: path,
                }));
            }
        }
    }

    let path = global_file(home);
    if path.exists() {
        let versions = read_tool_versions(&path)?;
        if let Some(version) = versions.tools.get(plugin) {
            return Ok(Some(ResolvedToolVersion {
                plugin: plugin.to_string(),
                version: version.clone(),
                scope: Scope::Global,
                source: path,
            }));
        }
    }

    Ok(None)
}
