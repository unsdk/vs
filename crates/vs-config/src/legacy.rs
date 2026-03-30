//! Compatibility helpers for legacy `vfox` home layouts.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ConfigError;
use crate::types::ToolVersions;

const LEGACY_FILES: [&str; 4] = [".tool-versions", ".nvmrc", ".node-version", ".sdkmanrc"];

/// Supported legacy file names.
pub fn supported_legacy_files() -> &'static [&'static str] {
    &LEGACY_FILES
}

/// Reads a legacy version file into the shared tool model.
pub fn read_legacy_versions(path: &Path) -> Result<ToolVersions, ConfigError> {
    let content = fs::read_to_string(path)?;
    parse_legacy_versions(path.file_name().and_then(std::ffi::OsStr::to_str), &content)
        .ok_or_else(|| ConfigError::UnsupportedLegacyFile(path.to_path_buf()))
}

/// Parses the contents of a supported legacy file.
pub fn parse_legacy_versions(file_name: Option<&str>, content: &str) -> Option<ToolVersions> {
    match file_name? {
        ".tool-versions" => Some(parse_tool_versions(content)),
        ".nvmrc" | ".node-version" => Some(single_tool("nodejs", content.trim())),
        ".sdkmanrc" => Some(parse_sdkmanrc(content)),
        _ => None,
    }
}

fn parse_tool_versions(content: &str) -> ToolVersions {
    let tools = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let plugin = parts.next()?;
            let version = parts.next()?;
            Some((plugin.to_string(), version.to_string()))
        })
        .collect::<BTreeMap<_, _>>();
    ToolVersions { tools }
}

fn parse_sdkmanrc(content: &str) -> ToolVersions {
    let tools = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            Some((key.trim().to_string(), value.trim().to_string()))
        })
        .collect::<BTreeMap<_, _>>();
    ToolVersions { tools }
}

fn single_tool(tool: &str, version: &str) -> ToolVersions {
    let mut tools = BTreeMap::new();
    if !version.is_empty() {
        tools.insert(tool.to_string(), version.to_string());
    }
    ToolVersions { tools }
}

/// Finds the nearest legacy file in the directory hierarchy.
pub fn find_legacy_file(cwd: &Path) -> Option<PathBuf> {
    cwd.ancestors()
        .flat_map(|directory| {
            supported_legacy_files()
                .iter()
                .map(move |file_name| directory.join(file_name))
        })
        .find(|candidate| candidate.exists())
}
