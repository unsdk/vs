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
    CurrentTool, InstalledVersion, MigrateSummary, PluginInfo, SelfUpgradeSummary, UninstallResult,
    UseScope, VersionInfo,
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
    use vs_config::{
        AppConfig, CacheConfig, HomeLayout, RegistryConfig, StorageConfig, write_app_config,
    };
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
    #[test]
    fn available_plugins_should_fallback_to_cached_registry_when_refresh_fails()
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
                active_home: home.clone(),
                migration_candidates: Vec::new(),
            },
            cwd.clone(),
            Some(String::from("session")),
        )?;
        assert!(!app.available_plugins()?.is_empty());

        write_app_config(
            &home,
            &AppConfig {
                proxy: Default::default(),
                storage: Default::default(),
                registry: RegistryConfig {
                    address: temp_dir
                        .path()
                        .join("missing/index.json")
                        .display()
                        .to_string(),
                },
                legacy_version_file: Default::default(),
                cache: Default::default(),
            },
        )?;

        let fallback = App::new(
            HomeLayout {
                active_home: home,
                migration_candidates: Vec::new(),
            },
            cwd,
            Some(String::from("session")),
        )?;
        let entries = fallback.available_plugins()?;
        assert!(entries.iter().any(|entry| entry.name == "nodejs"));
        Ok(())
    }

    #[cfg(feature = "lua")]
    #[test]
    fn add_plugin_should_fallback_to_cached_registry_when_refresh_fails()
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
                active_home: home.clone(),
                migration_candidates: Vec::new(),
            },
            cwd.clone(),
            Some(String::from("session")),
        )?;
        assert!(!app.available_plugins()?.is_empty());

        write_app_config(
            &home,
            &AppConfig {
                proxy: Default::default(),
                storage: Default::default(),
                registry: RegistryConfig {
                    address: temp_dir
                        .path()
                        .join("missing/index.json")
                        .display()
                        .to_string(),
                },
                legacy_version_file: Default::default(),
                cache: Default::default(),
            },
        )?;

        let fallback = App::new(
            HomeLayout {
                active_home: home,
                migration_candidates: Vec::new(),
            },
            cwd,
            Some(String::from("session")),
        )?;
        let entry = fallback.add_plugin(Some("nodejs"), None, None, None)?;
        assert_eq!(entry.name, "nodejs");
        Ok(())
    }

    #[cfg(feature = "lua")]
    #[test]
    fn storage_sdk_path_should_redirect_runtime_installs() -> Result<(), Box<dyn Error>> {
        let temp_dir = TempDir::new()?;
        let home = temp_dir.path().join("home");
        let storage_root = temp_dir.path().join("runtime-root");
        let cwd = temp_dir.path().join("project");
        let default_runtime_root = home.join("cache");
        fs::create_dir_all(&cwd)?;

        write_app_config(
            &home,
            &AppConfig {
                storage: StorageConfig {
                    sdk_path: storage_root.display().to_string(),
                },
                ..AppConfig::default()
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

        let source = temp_dir.path().join("nodejs-lua");
        write_lua_fixture(&source);
        app.add_plugin(
            Some("nodejs"),
            Some(source.display().to_string()),
            Some(PluginBackendKind::Lua),
            None,
        )?;
        let installed = app.install_plugin_version("nodejs", Some("20.11.1"))?;

        assert!(installed.install_dir.starts_with(&storage_root));
        assert!(!installed.install_dir.starts_with(default_runtime_root));
        Ok(())
    }

    #[cfg(feature = "lua")]
    #[test]
    fn project_tool_version_for_use_should_resolve_legacy_file() -> Result<(), Box<dyn Error>> {
        let temp_dir = TempDir::new()?;
        let home = temp_dir.path().join("home");
        let cwd = temp_dir.path().join("project");
        fs::create_dir_all(&cwd)?;
        fs::write(cwd.join(".nvmrc"), "20.11.1\n")?;

        let app = App::new(
            HomeLayout {
                active_home: home,
                migration_candidates: Vec::new(),
            },
            cwd,
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

        assert_eq!(
            app.project_tool_version_for_use("nodejs")?,
            Some(String::from("20.11.1"))
        );
        Ok(())
    }

    #[cfg(feature = "lua")]
    #[test]
    fn legacy_latest_installed_should_pick_the_newest_matching_runtime()
    -> Result<(), Box<dyn Error>> {
        let temp_dir = TempDir::new()?;
        let home = temp_dir.path().join("home");
        let cwd = temp_dir.path().join("project");
        fs::create_dir_all(&cwd)?;
        fs::write(cwd.join(".nvmrc"), "20\n")?;
        write_app_config(
            &home,
            &AppConfig {
                legacy_version_file: vs_config::LegacyVersionFileConfig {
                    enable: true,
                    strategy: String::from("latest_installed"),
                },
                ..AppConfig::default()
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

        let source = temp_dir.path().join("nodejs-lua");
        write_multi_version_lua_fixture(&source)?;
        app.add_plugin(
            Some("nodejs"),
            Some(source.display().to_string()),
            Some(PluginBackendKind::Lua),
            None,
        )?;
        app.install_plugin_version("nodejs", Some("20.9.0"))?;
        app.install_plugin_version("nodejs", Some("20.11.1"))?;

        assert_eq!(
            app.project_tool_version_for_use("nodejs")?,
            Some(String::from("20.11.1"))
        );
        Ok(())
    }

    #[cfg(feature = "lua")]
    #[test]
    fn available_hook_cache_should_return_cached_versions_when_enabled()
    -> Result<(), Box<dyn Error>> {
        let temp_dir = TempDir::new()?;
        let home = temp_dir.path().join("home");
        let cwd = temp_dir.path().join("project");
        fs::create_dir_all(&cwd)?;
        write_app_config(
            &home,
            &AppConfig {
                cache: CacheConfig {
                    available_hook_duration: String::from("12h"),
                },
                ..AppConfig::default()
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

        let source = temp_dir.path().join("nodejs-lua");
        write_multi_version_lua_fixture(&source)?;
        app.add_plugin(
            Some("nodejs"),
            Some(source.display().to_string()),
            Some(PluginBackendKind::Lua),
            None,
        )?;

        let versions = app.search_versions("nodejs", &[])?;
        fs::remove_dir_all(&source)?;
        let cached = app.search_versions("nodejs", &[])?;

        assert_eq!(versions, cached);
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

    #[cfg(feature = "lua")]
    fn write_multi_version_lua_fixture(root: &std::path::Path) -> Result<(), Box<dyn Error>> {
        fs::create_dir_all(root.join("hooks"))?;
        for version in ["20.9.0", "20.11.1"] {
            fs::create_dir_all(root.join(format!("packages/{version}/bin")))?;
        }
        fs::write(
            root.join("metadata.lua"),
            "PLUGIN = {}\nPLUGIN.name = 'nodejs'\nPLUGIN.version = '0.1.0'\nPLUGIN.legacyFilenames = { '.nvmrc' }\n",
        )?;
        fs::write(
            root.join("hooks/available.lua"),
            "function PLUGIN:Available(ctx)\n  return { { version = '20.11.1' }, { version = '20.9.0' } }\nend\n",
        )?;
        fs::write(
            root.join("hooks/pre_install.lua"),
            "function PLUGIN:PreInstall(ctx)\n  return { version = ctx.version, url = 'packages/' .. ctx.version }\nend\n",
        )?;
        fs::write(
            root.join("hooks/env_keys.lua"),
            "function PLUGIN:EnvKeys(ctx)\n  return { { key = 'PATH', value = ctx.path .. '/bin' } }\nend\n",
        )?;
        Ok(())
    }
}
