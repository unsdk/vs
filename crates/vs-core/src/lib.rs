//! Application orchestration for the `vs` CLI.

mod app;
mod error;
mod models;
mod plugin_source;
mod registry_source;
mod service;

pub use app::App;
pub use error::CoreError;
pub use models::{
    CurrentTool, InstalledVersion, MigrateSummary, PluginInfo, SelfUpgradeSummary, UseScope,
};

#[cfg(test)]
mod tests {
    #[cfg(feature = "lua")]
    use std::error::Error;
    #[cfg(feature = "lua")]
    use std::fs;

    #[cfg(feature = "lua")]
    use tempfile::TempDir;
    #[cfg(feature = "lua")]
    use vs_config::{AppConfig, HomeLayout, RegistryConfig, write_app_config};
    #[cfg(feature = "lua")]
    use vs_plugin_api::PluginBackendKind;

    #[cfg(feature = "lua")]
    use crate::{App, UseScope};

    #[cfg(feature = "lua")]
    #[test]
    fn use_tool_should_write_project_config() -> Result<(), Box<dyn Error>> {
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
        write_lua_fixture(&source);
        app.add_plugin(
            Some("nodejs"),
            Some(source.display().to_string()),
            Some(PluginBackendKind::Lua),
            None,
        )?;
        app.install_plugin_version("nodejs", Some("20.11.1"))?;

        app.use_tool("nodejs", "20.11.1", UseScope::Project, false)?;

        let config = fs::read_to_string(cwd.join(".vs.toml"))?;
        assert!(config.contains("nodejs = \"20.11.1\""));
        Ok(())
    }

    #[cfg(feature = "lua")]
    #[test]
    fn available_plugins_should_bootstrap_registry_when_source_is_configured()
    -> Result<(), Box<dyn Error>> {
        let temp_dir = TempDir::new()?;
        let home = temp_dir.path().join("home");
        let cwd = temp_dir.path().join("project");
        fs::create_dir_all(&cwd)?;
        let registry_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/registry/index.json");

        write_app_config(
            &home,
            &AppConfig {
                proxy: Default::default(),
                storage: Default::default(),
                registry: RegistryConfig {
                    address: registry_path.display().to_string(),
                },
                legacy_version_file: Default::default(),
                cache: Default::default(),
            },
        )?;

        let app = App::new(
            HomeLayout {
                active_home: home,
                migration_candidates: Vec::new(),
            },
            cwd,
            Some(String::from("session")),
        )?;

        let entries = app.available_plugins()?;
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|entry| entry.name == "nodejs"));
        Ok(())
    }

    #[cfg(feature = "lua")]
    fn write_lua_fixture(root: &std::path::Path) {
        if let Err(error) = fs::create_dir_all(root.join("hooks")) {
            panic!("failed to create hooks directory: {error}");
        }
        if let Err(error) = fs::create_dir_all(root.join("packages/20.11.1/bin")) {
            panic!("failed to create package directory: {error}");
        }
        fs::write(
            root.join("metadata.lua"),
            "PLUGIN = {}\nPLUGIN.name = 'nodejs'\nPLUGIN.version = '0.1.0'\nPLUGIN.legacyFilenames = { '.nvmrc' }\n",
        )
        .unwrap_or_else(|error| panic!("failed to write metadata fixture: {error}"));
        fs::write(
            root.join("hooks/available.lua"),
            "function PLUGIN:Available(ctx)\n  return { { version = '20.11.1' } }\nend\n",
        )
        .unwrap_or_else(|error| panic!("failed to write available fixture: {error}"));
        fs::write(
            root.join("hooks/pre_install.lua"),
            "function PLUGIN:PreInstall(ctx)\n  return { version = '20.11.1', url = 'packages/20.11.1' }\nend\n",
        )
        .unwrap_or_else(|error| panic!("failed to write pre_install fixture: {error}"));
        fs::write(
            root.join("hooks/env_keys.lua"),
            "function PLUGIN:EnvKeys(ctx)\n  return { { key = 'NODEJS_HOME', value = ctx.path }, { key = 'PATH', value = ctx.path .. '/bin' } }\nend\n",
        )
        .unwrap_or_else(|error| panic!("failed to write env_keys fixture: {error}"));
    }
}
