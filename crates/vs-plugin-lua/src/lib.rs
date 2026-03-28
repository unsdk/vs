//! Lua-compatible plugin runtime for `vs`.

mod backend;
mod loader;

pub use backend::LuaBackend;
pub use loader::{LuaPlugin, lua_library_dir};

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::path::PathBuf;

    use vs_plugin_api::Plugin;

    use crate::LuaPlugin;

    #[test]
    fn lua_plugin_should_load_fixture_versions() -> Result<(), Box<dyn Error>> {
        let root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/plugins/nodejs-lua");
        let plugin = LuaPlugin::load(&root)?;
        let versions = plugin.available_versions()?;
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].version, "20.11.1");
        Ok(())
    }
}
