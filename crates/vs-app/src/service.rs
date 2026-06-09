//! Typed, blocking wrappers around `vs_core::App`.

use std::sync::Arc;

use vs_core::{App, CoreError, InstalledVersion, UninstallResult};

use crate::model::{
    AddSource, ScopeChoice, ToolRow, VersionRow, available_rows, installed_rows, merge_tool_rows,
};

/// Blocking, UI-agnostic facade over `vs_core::App`.
///
/// Methods block (network/disk I/O) and MUST be called off the UI thread — the
/// view layer runs them on gpui's background executor.
#[derive(Clone)]
pub struct AppService {
    core: Arc<App>,
}

impl AppService {
    pub fn new(core: App) -> Self {
        Self {
            core: Arc::new(core),
        }
    }

    /// Sidebar: added tools with their current version (active scope).
    pub fn tool_rows(&self) -> Result<Vec<ToolRow>, CoreError> {
        let names = self
            .core
            .added_plugins()?
            .into_iter()
            .map(|entry| entry.name)
            .collect::<Vec<_>>();
        let statuses = self.core.current_tool_statuses()?;
        Ok(merge_tool_rows(names, statuses))
    }

    /// Detail/Installed: installed versions for a tool with the current flag.
    pub fn installed_rows(&self, name: &str) -> Result<Vec<VersionRow>, CoreError> {
        let installed = self
            .core
            .installed_versions_for_plugin(name)?
            .into_iter()
            .map(|v| v.version)
            .collect::<Vec<_>>();
        let current = self.core.current_tool(name)?.map(|c| c.version);
        Ok(installed_rows(installed, current.as_deref()))
    }

    /// Detail/Available: search results, marking already-installed versions.
    pub fn search_available(&self, name: &str, query: &str) -> Result<Vec<VersionRow>, CoreError> {
        let args: Vec<String> = if query.is_empty() {
            Vec::new()
        } else {
            vec![query.to_string()]
        };
        let found = self
            .core
            .search_versions(name, &args)?
            .into_iter()
            .map(|v| v.version)
            .collect::<Vec<_>>();
        let installed = self
            .core
            .installed_versions_for_plugin(name)?
            .into_iter()
            .map(|v| v.version)
            .collect::<Vec<_>>();
        Ok(available_rows(found, &installed))
    }

    /// Install a specific version, reporting download progress via `on_progress`
    /// as `(downloaded_bytes, total_bytes_if_known)`.
    pub fn install_with_progress(
        &self,
        name: &str,
        version: &str,
        on_progress: &vs_core::ProgressFn<'_>,
    ) -> Result<InstalledVersion, CoreError> {
        self.core
            .install_plugin_version(name, Some(version), Some(on_progress))
    }

    /// Switch the active version for a tool in the given scope.
    pub fn use_version(
        &self,
        name: &str,
        version: &str,
        scope: ScopeChoice,
    ) -> Result<InstalledVersion, CoreError> {
        self.core
            .use_tool(name, version, scope.to_use_scope(), false)
    }

    /// Uninstall a specific version.
    pub fn uninstall(&self, name: &str, version: &str) -> Result<UninstallResult, CoreError> {
        self.core.uninstall_plugin_version(name, version)
    }

    /// Registry plugins available to add (names).
    pub fn registry_plugin_names(&self) -> Result<Vec<String>, CoreError> {
        Ok(self
            .core
            .available_plugins()?
            .into_iter()
            .map(|entry| entry.name)
            .collect())
    }

    /// Add a tool, from the registry or from a source URL/path.
    pub fn add(&self, source: AddSource) -> Result<(), CoreError> {
        match source {
            AddSource::Registry { name } => {
                self.core.add_plugin(Some(&name), None, None, None)?;
            }
            AddSource::Source {
                source,
                alias,
                backend,
            } => {
                self.core.add_plugin(
                    None,
                    Some(source),
                    Some(backend.to_kind()),
                    alias.as_deref(),
                )?;
            }
        }
        Ok(())
    }

    /// Update a single plugin to the latest registry definition.
    /// The refreshed `RegistryEntry` is discarded; the UI only needs success/failure.
    pub fn update_plugin(&self, name: &str) -> Result<(), CoreError> {
        self.core.update_plugin(name)?;
        Ok(())
    }

    /// Remove a plugin (and its installed SDKs). Returns whether it existed.
    pub fn remove_plugin(&self, name: &str) -> Result<bool, CoreError> {
        self.core.remove_plugin(name)
    }

    /// Refresh the registry index; returns the number of plugins indexed.
    pub fn refresh_registry(&self) -> Result<usize, CoreError> {
        self.core.update_registry()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The GUI shares `AppService` across gpui's background executor, which
    // requires the wrapped `App` to be Send + Sync. This is a compile-time guard:
    // if `vs_core::App` ever stops being thread-safe, this fails to compile and
    // tells us the worker-thread design must change.
    #[test]
    fn app_service_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AppService>();
    }
}
