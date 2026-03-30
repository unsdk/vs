//! Persistence helpers for registry state on disk.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Serialize, de::DeserializeOwned};

use crate::{RegistryEntry, RegistryError};

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, RegistryError> {
    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(|error| RegistryError::Json {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<(), RegistryError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let rendered = serde_json::to_string_pretty(value).map_err(|error| RegistryError::Json {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    fs::write(path, rendered)?;
    Ok(())
}

/// Filesystem store for registry metadata.
#[derive(Debug, Clone)]
pub struct RegistryStore {
    home: PathBuf,
}

impl RegistryStore {
    /// Creates a new store rooted at the provided home.
    pub fn new(home: impl Into<PathBuf>) -> Self {
        Self { home: home.into() }
    }

    fn available_path(&self) -> PathBuf {
        self.home.join("registry").join("index.json")
    }

    fn added_path(&self) -> PathBuf {
        self.home.join("plugins").join("entries.json")
    }

    /// Loads the searchable plugin index.
    pub fn load_available(&self) -> Result<Vec<RegistryEntry>, RegistryError> {
        let path = self.available_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        read_json(&path)
    }

    /// Saves the searchable plugin index.
    pub fn save_available(&self, entries: &[RegistryEntry]) -> Result<(), RegistryError> {
        write_json(&self.available_path(), entries)
    }

    /// Loads plugins explicitly added to the local home.
    pub fn load_added(&self) -> Result<Vec<RegistryEntry>, RegistryError> {
        let path = self.added_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        read_json(&path)
    }

    /// Saves plugins explicitly added to the local home.
    pub fn save_added(&self, entries: &[RegistryEntry]) -> Result<(), RegistryError> {
        write_json(&self.added_path(), entries)
    }
}
