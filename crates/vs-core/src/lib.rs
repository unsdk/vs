//! Application orchestration for the `vs` CLI.

mod app;
mod error;
mod models;
mod service;

pub use app::App;
pub use error::CoreError;
pub use models::{CurrentTool, InstalledVersion, MigrateSummary, PluginInfo, UseScope};

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fs;

    use tempfile::TempDir;
    use vs_config::HomeLayout;
    use vs_plugin_api::PluginBackendKind;

    use crate::{App, UseScope};

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
            "nodejs",
            Some(source.display().to_string()),
            Some(PluginBackendKind::Lua),
        )?;

        app.use_tool("nodejs", "20.11.1", UseScope::Project, false)?;

        let config = fs::read_to_string(cwd.join(".vs.toml"))?;
        assert!(config.contains("nodejs = \"20.11.1\""));
        Ok(())
    }

    fn write_lua_fixture(root: &std::path::Path) {
        if let Err(error) = fs::create_dir_all(root.join("hooks")) {
            panic!("failed to create hooks directory: {error}");
        }
        if let Err(error) = fs::create_dir_all(root.join("packages/20.11.1/bin")) {
            panic!("failed to create package directory: {error}");
        }
        fs::write(
            root.join("metadata.lua"),
            "return { name = 'nodejs', legacy_filenames = { '.nvmrc' } }",
        )
        .unwrap_or_else(|error| panic!("failed to write metadata fixture: {error}"));
        fs::write(
            root.join("hooks/available.lua"),
            "return { { version = '20.11.1' } }",
        )
        .unwrap_or_else(|error| panic!("failed to write available fixture: {error}"));
        fs::write(
            root.join("hooks/pre_install.lua"),
            "return { ['20.11.1'] = { source = 'packages/20.11.1' } }",
        )
        .unwrap_or_else(|error| panic!("failed to write pre_install fixture: {error}"));
        fs::write(
            root.join("hooks/env_keys.lua"),
            "return { { key = 'NODEJS_HOME', value = '{install_dir}' } }",
        )
        .unwrap_or_else(|error| panic!("failed to write env_keys fixture: {error}"));
    }
}
