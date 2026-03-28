use std::env::split_paths;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{App, CoreError};

impl App {
    /// Executes a command with the currently resolved runtime environment.
    pub fn exec(&self, command: &str, args: &[String]) -> Result<i32, CoreError> {
        let delta = self.build_env()?;
        let path_value = self.path_with_delta(&delta)?;
        let resolved_command = resolve_command_path(command, &delta.path_entries);

        let mut child = Command::new(resolved_command.unwrap_or_else(|| PathBuf::from(command)));
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
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
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
        fs::write(&preferred_binary, "fixture preferred")?;
        #[cfg(windows)]
        fs::write(&fallback_script, "fixture fallback script")?;
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
}
