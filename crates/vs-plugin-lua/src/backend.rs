//! Backend adapter for loading Lua plugins.

use std::path::Path;

use vs_plugin_api::{Plugin, PluginError};

use crate::loader::LuaPlugin;

/// Loads Lua-backed plugins.
#[derive(Debug, Default, Clone)]
pub struct LuaBackend {
    proxy_url: Option<String>,
}

impl LuaBackend {
    /// Creates a Lua backend with an explicit HTTP proxy.
    pub fn with_proxy(proxy_url: Option<String>) -> Self {
        Self { proxy_url }
    }

    /// Loads a Lua plugin from disk.
    pub fn load(&self, source: &Path) -> Result<Box<dyn Plugin>, PluginError> {
        Ok(Box::new(LuaPlugin::load(
            source,
            self.proxy_url.as_deref(),
        )?))
    }
}
