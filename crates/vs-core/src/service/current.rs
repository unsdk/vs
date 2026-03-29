use crate::{App, CoreError, CurrentTool};

impl App {
    /// Resolves the active version for a specific plugin.
    pub fn current_tool(&self, plugin_name: &str) -> Result<Option<CurrentTool>, CoreError> {
        let resolved = vs_config::resolve_tool_version(
            self.home(),
            &self.cwd,
            self.session_id.as_deref(),
            plugin_name,
        )?;
        let Some(resolved) = resolved else {
            return Ok(None);
        };
        if self
            .load_installed_runtime(&resolved.plugin, &resolved.version)?
            .is_none()
        {
            return Ok(None);
        }
        Ok(Some(CurrentTool {
            plugin: resolved.plugin,
            version: resolved.version,
            scope: resolved.scope,
            source: resolved.source,
        }))
    }

    /// Resolves all active tools for the current context.
    pub fn current_tools(&self) -> Result<Vec<CurrentTool>, CoreError> {
        let mut current = Vec::new();
        for configured in self.collect_current_tools()? {
            if self
                .load_installed_runtime(&configured.plugin, &configured.version)?
                .is_some()
            {
                current.push(configured);
            }
        }
        Ok(current)
    }

    /// Returns all known plugins with their current version when available.
    pub fn current_tool_statuses(&self) -> Result<Vec<(String, Option<String>)>, CoreError> {
        let mut names = self
            .added_plugins()?
            .into_iter()
            .map(|entry| entry.name)
            .collect::<std::collections::BTreeSet<_>>();
        names.extend(
            self.list_installed_versions()?
                .into_iter()
                .map(|installed| installed.plugin),
        );

        let mut statuses = Vec::new();
        for plugin in names {
            let current = self.current_tool(&plugin)?;
            statuses.push((plugin, current.map(|tool| tool.version)));
        }
        Ok(statuses)
    }
}
