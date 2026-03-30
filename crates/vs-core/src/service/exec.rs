//! Services for executing commands with resolved runtime environments.

use std::env::split_paths;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use vs_plugin_api::InstalledRuntime;
use vs_shell::{EnvDelta, bin_dir};

use crate::{App, CoreError};

impl App {
    /// Executes a command with the resolved runtime environment for a specific SDK.
    pub fn exec(
        &self,
        plugin_name: &str,
        requested_version: Option<&str>,
        command: &str,
        args: &[String],
    ) -> Result<i32, CoreError> {
        let runtime = self.resolve_exec_runtime(plugin_name, requested_version)?;
        let entry = self.resolve_registry_entry(plugin_name)?;
        let plugin = self.load_plugin(&entry)?;
        let mut delta = EnvDelta::default();
        delta.path_entries.push(bin_dir(runtime.main_path()));
        apply_exec_env_keys(&mut delta, plugin.env_keys(&runtime)?);
        let path_value = self.path_with_delta(&delta)?;
        let resolved_command =
            resolve_command_path(command, &delta.path_entries).ok_or_else(|| {
                CoreError::CommandExecution {
                    command: command.to_string(),
                    message: format!("command not found in {} environment", runtime.plugin),
                }
            })?;

        let mut child = Command::new(resolved_command);
        child.args(args);
        child.env("PATH", path_value);
        for (key, value) in delta.vars {
            child.env(key, value);
        }

        let status = child
            .status()
            .map_err(|error| CoreError::CommandExecution {
                command: command.to_string(),
                message: error.to_string(),
            })?;

        Ok(status.code().unwrap_or(1))
    }

    fn resolve_exec_runtime(
        &self,
        plugin_name: &str,
        requested_version: Option<&str>,
    ) -> Result<InstalledRuntime, CoreError> {
        let version = if let Some(version) = requested_version {
            if let Some(runtime) = self.load_installed_runtime(plugin_name, version)? {
                return Ok(runtime);
            }
            let installed = self.install_plugin_version(plugin_name, Some(version))?;
            installed.version
        } else {
            self.current_tool(plugin_name)?
                .map(|current| current.version)
                .ok_or_else(|| {
                    CoreError::Unsupported(format!(
                        "no version configured for {plugin_name}. Please use `vs use` first"
                    ))
                })?
        };

        self.load_installed_runtime(plugin_name, &version)?
            .ok_or_else(|| {
                CoreError::Unsupported(format!(
                    "failed to load installed runtime for {plugin_name}@{version}"
                ))
            })
    }
}

fn resolve_command_path(command: &str, preferred_paths: &[PathBuf]) -> Option<PathBuf> {
    let command_path = Path::new(command);
    if command_path.is_absolute() || command.contains(std::path::MAIN_SEPARATOR) {
        return Some(command_path.to_path_buf());
    }
    #[cfg(windows)]
    if command.contains('/') || command.contains('\\') {
        return Some(command_path.to_path_buf());
    }

    let mut search_paths = preferred_paths.to_vec();
    if let Some(path) = std::env::var_os("PATH") {
        search_paths.extend(split_paths(&path));
    }

    #[cfg(windows)]
    {
        let pathext = std::env::var_os("PATHEXT")
            .map(|extensions| {
                extensions
                    .to_string_lossy()
                    .split(';')
                    .filter(|extension| !extension.is_empty())
                    .map(|extension| extension.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| {
                [".COM", ".EXE", ".BAT", ".CMD"]
                    .into_iter()
                    .map(String::from)
                    .collect()
            });

        for directory in search_paths {
            if command_path.extension().is_some() {
                let direct_match = directory.join(command);
                if direct_match.is_file() {
                    return Some(direct_match);
                }
                continue;
            }

            for extension in &pathext {
                let candidate = directory.join(format!("{command}{extension}"));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    #[cfg(not(windows))]
    {
        for directory in search_paths {
            let candidate = directory.join(command);
            if is_executable_file(&candidate) {
                return Some(candidate);
            }
        }
    }

    None
}

fn apply_exec_env_keys(delta: &mut EnvDelta, env_keys: Vec<vs_plugin_api::EnvKey>) {
    for env_key in env_keys {
        if env_key.key == "PATH" {
            delta.path_entries.push(PathBuf::from(env_key.value));
        } else {
            delta.vars.push((env_key.key, env_key.value));
        }
    }
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    path.is_file()
        && fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fs;
    use std::path::Path;

    use tempfile::TempDir;

    use super::resolve_command_path;

    #[test]
    fn resolve_command_path_should_prefer_runtime_bin_directory() -> Result<(), Box<dyn Error>> {
        let temp_dir = TempDir::new()?;
        let preferred = temp_dir.path().join("preferred");
        let fallback = temp_dir.path().join("fallback");
        fs::create_dir_all(&preferred)?;
        fs::create_dir_all(&fallback)?;

        #[cfg(windows)]
        let preferred_script = preferred.join("node");
        #[cfg(windows)]
        let preferred_binary = preferred.join("node.cmd");
        #[cfg(not(windows))]
        let preferred_binary = preferred.join("node");

        #[cfg(windows)]
        let fallback_script = fallback.join("node");
        #[cfg(windows)]
        let fallback_binary = fallback.join("node.cmd");
        #[cfg(not(windows))]
        let fallback_binary = fallback.join("node");

        #[cfg(windows)]
        fs::write(&preferred_script, "fixture preferred script")?;
        #[cfg(not(windows))]
        make_executable(&preferred_binary, b"#!/bin/sh\necho preferred\n")?;
        #[cfg(windows)]
        fs::write(&preferred_binary, "fixture preferred")?;
        #[cfg(windows)]
        fs::write(&fallback_script, "fixture fallback script")?;
        #[cfg(not(windows))]
        make_executable(&fallback_binary, b"#!/bin/sh\necho fallback\n")?;
        #[cfg(windows)]
        fs::write(&fallback_binary, "fixture fallback")?;

        let resolved = resolve_command_path("node", &[preferred, fallback])
            .ok_or_else(|| std::io::Error::other("missing resolved command"))?;

        assert_path_matches(&resolved, &preferred_binary);
        Ok(())
    }

    #[cfg(windows)]
    fn assert_path_matches(actual: &Path, expected: &Path) {
        assert!(
            actual
                .to_string_lossy()
                .eq_ignore_ascii_case(expected.to_string_lossy().as_ref()),
            "assertion failed: actual path {actual:?} does not match expected path {expected:?}",
        );
    }

    #[cfg(not(windows))]
    fn assert_path_matches(actual: &Path, expected: &Path) {
        assert_eq!(actual, expected);
    }

    #[cfg(unix)]
    #[test]
    fn resolve_command_path_should_skip_non_executable_candidates() -> Result<(), Box<dyn Error>> {
        let temp_dir = TempDir::new()?;
        let preferred = temp_dir.path().join("preferred");
        let fallback = temp_dir.path().join("fallback");
        fs::create_dir_all(&preferred)?;
        fs::create_dir_all(&fallback)?;

        fs::write(preferred.join("node"), "not executable")?;
        make_executable(&fallback.join("node"), b"#!/bin/sh\necho fallback\n")?;

        let resolved = resolve_command_path("node", &[preferred, fallback])
            .ok_or_else(|| std::io::Error::other("missing resolved command"))?;

        assert_eq!(resolved, temp_dir.path().join("fallback/node"));
        Ok(())
    }

    #[cfg(unix)]
    fn make_executable(path: &Path, contents: &[u8]) -> Result<(), Box<dyn Error>> {
        use std::os::unix::fs::PermissionsExt;

        fs::write(path, contents)?;
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)?;
        Ok(())
    }
}
