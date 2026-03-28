use crate::{App, CoreError, CurrentTool};

impl App {
    /// Resolves the active version for a specific plugin.
    pub fn current_tool(&self, plugin_name: &str) -> Result<Option<CurrentTool>, CoreError> {
        Ok(vs_config::resolve_tool_version(
            self.home(),
            &self.cwd,
            self.session_id.as_deref(),
            plugin_name,
        )?
        .map(|resolved| CurrentTool {
            plugin: resolved.plugin,
            version: resolved.version,
            scope: resolved.scope,
            source: resolved.source,
        }))
    }

    /// Resolves all active tools for the current context.
    pub fn current_tools(&self) -> Result<Vec<CurrentTool>, CoreError> {
        self.collect_current_tools()
    }
}
