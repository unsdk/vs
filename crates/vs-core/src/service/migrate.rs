use crate::{App, CoreError, MigrateSummary};

impl App {
    /// Copies compatible state from a legacy home into the active home.
    pub fn migrate(&self, source: Option<String>) -> Result<MigrateSummary, CoreError> {
        let source_home = source
            .map(|source| self.normalize_source_path(&source))
            .or_else(|| self.home_layout.migration_candidates.first().cloned())
            .ok_or(CoreError::MissingMigrationSource)?;

        let mut copied_roots = 0;
        for relative in ["config.yaml", "global", "registry", "plugins", "cache"] {
            let source_path = source_home.join(relative);
            let destination_path = self.home().join(relative);
            if !source_path.exists() {
                continue;
            }
            if source_path.is_dir() {
                self.copy_tree(&source_path, &destination_path)?;
            } else {
                if let Some(parent) = destination_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(&source_path, &destination_path)?;
            }
            copied_roots += 1;
        }

        Ok(MigrateSummary {
            source_home,
            copied_roots,
        })
    }
}
