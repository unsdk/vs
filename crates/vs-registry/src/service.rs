use std::path::PathBuf;

use crate::{RegistryEntry, RegistryError, RegistryStore};

/// High-level registry operations.
#[derive(Debug, Clone)]
pub struct RegistryService {
    store: RegistryStore,
}

impl RegistryService {
    /// Creates a registry service rooted at a `vs` home.
    pub fn new(home: impl Into<PathBuf>) -> Self {
        Self {
            store: RegistryStore::new(home),
        }
    }

    /// Returns the searchable plugin index.
    pub fn available_plugins(&self) -> Result<Vec<RegistryEntry>, RegistryError> {
        self.store.load_available()
    }

    /// Replaces the searchable plugin index.
    pub fn replace_available_plugins(
        &self,
        entries: &[RegistryEntry],
    ) -> Result<(), RegistryError> {
        self.store.save_available(entries)
    }

    /// Returns locally added plugins.
    pub fn added_plugins(&self) -> Result<Vec<RegistryEntry>, RegistryError> {
        self.store.load_added()
    }

    /// Adds a plugin from a registry entry or explicit source.
    pub fn add_plugin(&self, entry: RegistryEntry) -> Result<(), RegistryError> {
        let mut entries = self.store.load_added()?;
        if let Some(existing) = entries
            .iter_mut()
            .find(|existing| existing.matches(&entry.name))
        {
            *existing = entry;
        } else {
            entries.push(entry);
        }
        entries.sort_by(|left, right| left.name.cmp(&right.name));
        self.store.save_added(&entries)
    }

    /// Removes a locally added plugin.
    pub fn remove_plugin(&self, name: &str) -> Result<bool, RegistryError> {
        let mut entries = self.store.load_added()?;
        let before = entries.len();
        entries.retain(|entry| !entry.matches(name));
        self.store.save_added(&entries)?;
        Ok(before != entries.len())
    }

    /// Searches the available registry index.
    pub fn search(&self, query: &str) -> Result<Vec<RegistryEntry>, RegistryError> {
        let query = query.to_ascii_lowercase();
        let mut entries = self.store.load_available()?;
        entries.retain(|entry| {
            entry.name.to_ascii_lowercase().contains(&query)
                || entry
                    .description
                    .as_deref()
                    .unwrap_or_default()
                    .to_ascii_lowercase()
                    .contains(&query)
                || entry
                    .aliases
                    .iter()
                    .any(|alias| alias.to_ascii_lowercase().contains(&query))
        });
        Ok(entries)
    }

    /// Finds a plugin by exact name or alias, searching added entries before the index.
    pub fn resolve(&self, name: &str) -> Result<Option<RegistryEntry>, RegistryError> {
        if let Some(entry) = self
            .store
            .load_added()?
            .into_iter()
            .find(|entry| entry.matches(name))
        {
            return Ok(Some(entry));
        }
        Ok(self
            .store
            .load_available()?
            .into_iter()
            .find(|entry| entry.matches(name)))
    }
}
