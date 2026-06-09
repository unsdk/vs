//! UI-agnostic view-model types and pure transforms.

use vs_core::UseScope;
use vs_plugin_api::PluginBackendKind;

/// A tool (added plugin) shown in the sidebar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolRow {
    pub name: String,
    pub current_version: Option<String>,
}

/// A version row in the Installed or Available section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionRow {
    pub version: String,
    pub installed: bool,
    pub current: bool,
}

/// The scope a `use` action applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeChoice {
    Project,
    Global,
    Session,
}

impl ScopeChoice {
    pub fn all() -> [ScopeChoice; 3] {
        [
            ScopeChoice::Project,
            ScopeChoice::Global,
            ScopeChoice::Session,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            ScopeChoice::Project => "Project",
            ScopeChoice::Global => "Global",
            ScopeChoice::Session => "Session",
        }
    }

    pub fn to_use_scope(self) -> UseScope {
        match self {
            ScopeChoice::Project => UseScope::Project,
            ScopeChoice::Global => UseScope::Global,
            ScopeChoice::Session => UseScope::Session,
        }
    }
}

/// Plugin backend chosen in the "Add from source" form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendChoice {
    Lua,
    Wasi,
}

impl BackendChoice {
    pub fn to_kind(self) -> PluginBackendKind {
        match self {
            BackendChoice::Lua => PluginBackendKind::Lua,
            BackendChoice::Wasi => PluginBackendKind::Wasi,
        }
    }
}

/// How the user wants to add a tool in the Add-tool dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AddSource {
    Registry {
        name: String,
    },
    Source {
        source: String,
        alias: Option<String>,
        backend: BackendChoice,
    },
}

/// Merge the list of added plugin names with their current-version statuses,
/// producing sidebar rows sorted by name.
pub fn merge_tool_rows(
    names: Vec<String>,
    statuses: Vec<(String, Option<String>)>,
) -> Vec<ToolRow> {
    let mut rows: Vec<ToolRow> = names
        .into_iter()
        .map(|name| {
            let current_version = statuses
                .iter()
                .find(|(plugin, _)| *plugin == name)
                .and_then(|(_, version)| version.clone());
            ToolRow {
                name,
                current_version,
            }
        })
        .collect();
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    rows
}

/// Build the Installed-section rows, flagging the active version.
pub fn installed_rows(installed: Vec<String>, current: Option<&str>) -> Vec<VersionRow> {
    installed
        .into_iter()
        .map(|version| {
            let is_current = current == Some(version.as_str());
            VersionRow {
                version,
                installed: true,
                current: is_current,
            }
        })
        .collect()
}

/// Build the Available-section rows, marking versions already installed so the
/// UI can disable their Install button.
pub fn available_rows(found: Vec<String>, installed: &[String]) -> Vec<VersionRow> {
    found
        .into_iter()
        .map(|version| {
            let already = installed.iter().any(|v| v == &version);
            VersionRow {
                version,
                installed: already,
                current: false,
            }
        })
        .collect()
}

/// Case-insensitive substring filter for the sidebar tool list.
pub fn filter_tool_rows(rows: &[ToolRow], query: &str) -> Vec<ToolRow> {
    if query.is_empty() {
        return rows.to_vec();
    }
    let needle = query.to_lowercase();
    rows.iter()
        .filter(|row| row.name.to_lowercase().contains(&needle))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_choice_maps_to_use_scope_and_all_lists_three() {
        assert_eq!(ScopeChoice::Project.to_use_scope(), UseScope::Project);
        assert_eq!(ScopeChoice::Global.to_use_scope(), UseScope::Global);
        assert_eq!(ScopeChoice::Session.to_use_scope(), UseScope::Session);
        assert_eq!(ScopeChoice::all().len(), 3);
        assert_eq!(ScopeChoice::Project.label(), "Project");
    }

    #[test]
    fn backend_choice_maps_to_plugin_backend_kind() {
        assert_eq!(BackendChoice::Lua.to_kind(), PluginBackendKind::Lua);
        assert_eq!(BackendChoice::Wasi.to_kind(), PluginBackendKind::Wasi);
    }

    #[test]
    fn merge_tool_rows_pairs_names_with_current_versions_sorted() {
        let names = vec!["python".to_string(), "nodejs".to_string()];
        let statuses = vec![
            ("nodejs".to_string(), Some("20.11.1".to_string())),
            ("python".to_string(), None),
        ];
        let rows = merge_tool_rows(names, statuses);
        assert_eq!(
            rows,
            vec![
                ToolRow {
                    name: "nodejs".into(),
                    current_version: Some("20.11.1".into())
                },
                ToolRow {
                    name: "python".into(),
                    current_version: None
                },
            ]
        );
    }

    #[test]
    fn installed_rows_flag_the_current_version() {
        let rows = installed_rows(
            vec!["20.11.1".to_string(), "18.19.0".to_string()],
            Some("20.11.1"),
        );
        assert_eq!(
            rows,
            vec![
                VersionRow {
                    version: "20.11.1".into(),
                    installed: true,
                    current: true
                },
                VersionRow {
                    version: "18.19.0".into(),
                    installed: true,
                    current: false
                },
            ]
        );
    }

    #[test]
    fn available_rows_mark_already_installed_versions() {
        let rows = available_rows(
            vec!["21.6.0".to_string(), "20.11.1".to_string()],
            &["20.11.1".to_string()],
        );
        assert_eq!(
            rows,
            vec![
                VersionRow {
                    version: "21.6.0".into(),
                    installed: false,
                    current: false
                },
                VersionRow {
                    version: "20.11.1".into(),
                    installed: true,
                    current: false
                },
            ]
        );
    }

    #[test]
    fn filter_tool_rows_is_case_insensitive_substring() {
        let rows = vec![
            ToolRow {
                name: "nodejs".into(),
                current_version: None,
            },
            ToolRow {
                name: "python".into(),
                current_version: None,
            },
        ];
        let filtered = filter_tool_rows(&rows, "PY");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "python");
        // Empty query returns everything.
        assert_eq!(filter_tool_rows(&rows, "").len(), 2);
    }
}
