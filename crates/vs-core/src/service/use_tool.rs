//! Services for activating a tool version in a selected scope.

use std::fs;
use std::io::Write;
use std::path::Path;

use vs_plugin_api::InstalledRuntime;
use vs_shell::{global_current_dir, link_directory, project_sdk_dir};

use crate::{App, CoreError, InstalledVersion, UseScope};

impl App {
    /// Activates an installed tool version for a given scope.
    pub fn use_tool(
        &self,
        plugin_name: &str,
        version: &str,
        scope: UseScope,
        unlink: bool,
    ) -> Result<InstalledVersion, CoreError> {
        // Verify hook environment is available (session ID proves shell hooks are loaded).
        let scope = self.verify_hook_env(scope)?;

        let entry = self.resolve_registry_entry(plugin_name)?;
        let plugin = self.load_plugin(&entry)?;
        let previous_version = self
            .current_tool(plugin_name)?
            .map(|current| current.version);
        let installed_runtimes = self.load_installed_runtimes(plugin_name)?;

        let requested_version =
            self.resolve_requested_use_version(&*plugin, plugin_name, version)?;
        let hook_version = plugin.pre_use(
            &requested_version,
            scope.as_str(),
            &self.cwd,
            previous_version.as_deref(),
            &installed_runtimes,
        )?;
        let resolved_version = match hook_version {
            Some(v) => v,
            None => self.fuzzy_match_version(plugin_name, &requested_version, &installed_runtimes),
        };

        let runtime = self
            .load_installed_runtime(plugin_name, &resolved_version)?
            .ok_or_else(|| {
                CoreError::Unsupported(format!(
                    "{plugin_name}@{resolved_version} is not installed. Please run `vs install {plugin_name}@{resolved_version}` first"
                ))
            })?;
        let installed = InstalledVersion {
            plugin: plugin_name.to_string(),
            version: runtime.version.clone(),
            install_dir: runtime.root_dir.clone(),
        };

        match scope {
            UseScope::Global => {
                self.write_tool_assignment(
                    &vs_config::global_tools_file(self.home()),
                    plugin_name,
                    Some(&runtime.version),
                )?;
                link_directory(
                    &installed.install_dir,
                    &global_current_dir(self.home(), plugin_name),
                )?;
            }
            UseScope::Project => {
                self.write_tool_assignment(
                    &self.preferred_project_file(),
                    plugin_name,
                    Some(&runtime.version),
                )?;
                if !unlink {
                    link_directory(
                        &installed.install_dir,
                        &project_sdk_dir(&self.cwd, plugin_name),
                    )?;
                }
                ensure_vs_in_gitignore(&self.cwd);
            }
            UseScope::Session => {
                let session_file = self.session_file()?;
                self.write_tool_assignment(&session_file, plugin_name, Some(&runtime.version))?;
            }
        }
        Ok(installed)
    }

    /// Resolves the version from the project config file when no version is
    /// specified on the CLI (empty string). Returns `None` when no project
    /// config is found.
    pub fn project_tool_version_for_use(
        &self,
        plugin_name: &str,
    ) -> Result<Option<String>, CoreError> {
        Ok(vs_config::find_project_file(&self.cwd)
            .map(|path| vs_config::read_tool_versions(&path))
            .transpose()?
            .and_then(|tools| tools.tools.get(plugin_name).cloned()))
    }

    /// Returns installed versions for a single plugin, sorted from newest-looking to oldest-looking.
    pub fn installed_versions_for_plugin(
        &self,
        plugin_name: &str,
    ) -> Result<Vec<InstalledVersion>, CoreError> {
        let mut installed = self
            .list_installed_versions()?
            .into_iter()
            .filter(|installed| installed.plugin == plugin_name)
            .collect::<Vec<_>>();
        installed.sort_by(|left, right| right.version.cmp(&left.version));
        Ok(installed)
    }

    fn load_installed_runtimes(
        &self,
        plugin_name: &str,
    ) -> Result<Vec<InstalledRuntime>, CoreError> {
        let mut runtimes = Vec::new();
        for installed in self.installed_versions_for_plugin(plugin_name)? {
            if let Some(runtime) = self.load_installed_runtime(plugin_name, &installed.version)? {
                runtimes.push(runtime);
            }
        }
        Ok(runtimes)
    }

    fn resolve_requested_use_version(
        &self,
        plugin: &dyn vs_plugin_api::Plugin,
        plugin_name: &str,
        version: &str,
    ) -> Result<String, CoreError> {
        if version != "latest" {
            return Ok(version.to_string());
        }

        plugin
            .available_versions(&[])?
            .into_iter()
            .next()
            .map(|available| available.version)
            .ok_or_else(|| {
                CoreError::Unsupported(format!(
                    "plugin {plugin_name} does not expose any available versions"
                ))
            })
    }

    /// Fuzzy-matches a version against installed runtimes.
    ///
    /// 1. Exact match — return immediately.
    /// 2. Prefix match — e.g. "20" matches "20.11.1".
    /// 3. No match — return the original version unchanged so the caller
    ///    produces the "not installed" error.
    fn fuzzy_match_version(
        &self,
        plugin_name: &str,
        version: &str,
        installed_runtimes: &[InstalledRuntime],
    ) -> String {
        // Exact match.
        if self
            .load_installed_runtime(plugin_name, version)
            .ok()
            .flatten()
            .is_some()
        {
            return version.to_string();
        }

        // Prefix match — sort installed versions and pick the first match.
        let mut versions: Vec<&str> = installed_runtimes
            .iter()
            .map(|rt| rt.version.as_str())
            .collect();
        versions.sort();

        let prefix = format!("{version}.");
        for v in &versions {
            if *v == version {
                return version.to_string();
            }
            if v.starts_with(&prefix) {
                return (*v).to_string();
            }
        }

        version.to_string()
    }

    /// Verifies the requested scope can be written in the current environment.
    ///
    /// Project and global scopes are persisted on disk and do not require shell
    /// hooks. Session scope depends on `VS_SESSION_ID`, and on Windows we
    /// degrade to global scope when hooks are unavailable.
    fn verify_hook_env(&self, scope: UseScope) -> Result<UseScope, CoreError> {
        if self.session_id.is_some() {
            return Ok(scope);
        }

        match scope {
            UseScope::Global | UseScope::Project => Ok(scope),
            UseScope::Session if cfg!(windows) => {
                eprintln!(
                    "Warning: The current shell lacks hook support. Switching to global scope automatically."
                );
                Ok(UseScope::Global)
            }
            UseScope::Session => Err(CoreError::Unsupported(String::from(
                "vs requires hook support. Please ensure vs is properly initialized with `eval \"$(vs activate <shell>)\"`",
            ))),
        }
    }
}

/// If a `.gitignore` exists in `project_dir` and does not already mention
/// `.vs/` or `.vs`, appends `.vs/` to it.
fn ensure_vs_in_gitignore(project_dir: &Path) {
    let gitignore_path = project_dir.join(".gitignore");
    let Ok(content) = fs::read_to_string(&gitignore_path) else {
        return; // no .gitignore — don't create one
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == ".vs/" || trimmed == ".vs" {
            return; // already present
        }
    }

    let Ok(mut file) = fs::OpenOptions::new().append(true).open(&gitignore_path) else {
        return;
    };
    let entry = if content.ends_with('\n') || content.is_empty() {
        ".vs/\n"
    } else {
        "\n.vs/\n"
    };
    let _ = file.write_all(entry.as_bytes());
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fs;

    use tempfile::TempDir;
    use vs_config::HomeLayout;
    use vs_plugin_api::PluginBackendKind;

    use crate::{App, UseScope};

    fn new_app_without_session(temp_dir: &TempDir) -> Result<App, Box<dyn Error>> {
        let home = temp_dir.path().join("home");
        let cwd = temp_dir.path().join("project");
        fs::create_dir_all(&cwd)?;

        Ok(App::new(
            HomeLayout {
                active_home: home,
                migration_candidates: Vec::new(),
            },
            cwd,
            None,
        )?)
    }

    #[test]
    fn project_scope_should_not_require_shell_hooks() -> Result<(), Box<dyn Error>> {
        let temp_dir = TempDir::new()?;
        let app = new_app_without_session(&temp_dir)?;

        assert_eq!(app.verify_hook_env(UseScope::Project)?, UseScope::Project);
        assert_eq!(app.verify_hook_env(UseScope::Global)?, UseScope::Global);
        Ok(())
    }

    #[cfg(not(windows))]
    #[test]
    fn session_scope_should_require_shell_hooks_on_unix() -> Result<(), Box<dyn Error>> {
        let temp_dir = TempDir::new()?;
        let app = new_app_without_session(&temp_dir)?;

        let error = match app.verify_hook_env(UseScope::Session) {
            Ok(scope) => {
                return Err(Box::new(std::io::Error::other(format!(
                    "session scope unexpectedly succeeded with scope {scope:?}",
                ))));
            }
            Err(error) => error,
        };
        assert!(error.to_string().contains("vs requires hook support"));
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn session_scope_should_fallback_to_global_without_shell_hooks_on_windows()
    -> Result<(), Box<dyn Error>> {
        let temp_dir = TempDir::new()?;
        let app = new_app_without_session(&temp_dir)?;

        assert_eq!(app.verify_hook_env(UseScope::Session)?, UseScope::Global);
        Ok(())
    }

    #[cfg(feature = "lua")]
    #[test]
    fn use_tool_should_apply_pre_use_resolution() -> Result<(), Box<dyn Error>> {
        let temp_dir = TempDir::new()?;
        let home = temp_dir.path().join("home");
        let cwd = temp_dir.path().join("project");
        fs::create_dir_all(&cwd)?;

        let app = App::new(
            HomeLayout {
                active_home: home,
                migration_candidates: Vec::new(),
            },
            cwd.clone(),
            Some(String::from("session")),
        )?;

        let source = temp_dir.path().join("nodejs-lua");
        write_pre_use_fixture(&source)?;
        app.add_plugin(
            Some("nodejs"),
            Some(source.display().to_string()),
            Some(PluginBackendKind::Lua),
            None,
        )?;
        app.install_plugin_version("nodejs", Some("20.11.1"))?;

        let installed = app.use_tool("nodejs", "lts", UseScope::Project, false)?;

        assert_eq!(installed.version, "20.11.1");
        let config = fs::read_to_string(cwd.join(".vs.toml"))?;
        assert!(config.contains("nodejs = \"20.11.1\""));
        Ok(())
    }

    #[cfg(feature = "lua")]
    #[test]
    fn use_tool_should_fail_when_requested_version_is_not_installed() -> Result<(), Box<dyn Error>>
    {
        let temp_dir = TempDir::new()?;
        let home = temp_dir.path().join("home");
        let cwd = temp_dir.path().join("project");
        fs::create_dir_all(&cwd)?;

        let app = App::new(
            HomeLayout {
                active_home: home,
                migration_candidates: Vec::new(),
            },
            cwd,
            Some(String::from("session")),
        )?;

        let source = temp_dir.path().join("nodejs-lua");
        write_pre_use_fixture(&source)?;
        app.add_plugin(
            Some("nodejs"),
            Some(source.display().to_string()),
            Some(PluginBackendKind::Lua),
            None,
        )?;

        let error = match app.use_tool("nodejs", "20.11.1", UseScope::Project, false) {
            Ok(_) => {
                return Err(Box::new(std::io::Error::other(
                    "use should fail without a matching installed runtime",
                )));
            }
            Err(error) => error,
        };
        assert!(
            error
                .to_string()
                .contains("Please run `vs install nodejs@20.11.1` first")
        );
        Ok(())
    }

    #[cfg(feature = "lua")]
    fn write_pre_use_fixture(root: &std::path::Path) -> Result<(), Box<dyn Error>> {
        fs::create_dir_all(root.join("hooks"))?;
        fs::create_dir_all(root.join("packages/20.11.1/bin"))?;
        fs::write(
            root.join("metadata.lua"),
            "PLUGIN = {}\nPLUGIN.name = 'nodejs'\nPLUGIN.version = '0.1.0'\n",
        )?;
        fs::write(
            root.join("hooks/pre_install.lua"),
            "function PLUGIN:PreInstall(ctx)\n  return { version = '20.11.1', url = 'packages/20.11.1' }\nend\n",
        )?;
        fs::write(
            root.join("hooks/available.lua"),
            "function PLUGIN:Available(ctx)\n  return { { version = '20.11.1' } }\nend\n",
        )?;
        fs::write(
            root.join("hooks/env_keys.lua"),
            "function PLUGIN:EnvKeys(ctx)\n  return { { key = 'PATH', value = ctx.path .. '/bin' } }\nend\n",
        )?;
        fs::write(
            root.join("hooks/pre_use.lua"),
            "function PLUGIN:preUse(ctx)\n  if ctx.version == 'lts' then\n    return { version = '20.11.1' }\n  end\n  return { version = ctx.version }\nend\n",
        )?;
        Ok(())
    }
}
