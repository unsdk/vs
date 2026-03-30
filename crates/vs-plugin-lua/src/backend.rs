//! Backend adapter for loading Lua plugins.

use std::path::Path;

use vs_plugin_api::{Plugin, PluginError};

use crate::loader::LuaPlugin;

/// Loads Lua-backed plugins.
#[derive(Debug, Default, Clone, Copy)]
pub struct LuaBackend;

impl LuaBackend {
    /// Loads a Lua plugin from disk.
    pub fn load(&self, source: &Path) -> Result<Box<dyn Plugin>, PluginError> {
        Ok(Box::new(LuaPlugin::load(source)?))
    }
}
