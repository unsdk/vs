use std::path::PathBuf;

/// Environment changes emitted by `vs`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvDelta {
    /// Environment variables to export.
    pub vars: Vec<(String, String)>,
    /// Additional PATH entries to prepend.
    pub path_entries: Vec<PathBuf>,
}

impl EnvDelta {
    /// Adds a variable export to the delta.
    pub fn with_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.vars.push((key.into(), value.into()));
        self
    }

    /// Adds a PATH entry to the delta.
    pub fn with_path(mut self, value: PathBuf) -> Self {
        self.path_entries.push(value);
        self
    }
}
