use serde::{Deserialize, Serialize};

/// Metadata persisted after a successful install.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallReceipt {
    /// Plugin identifier.
    pub plugin: String,
    /// Installed version.
    pub version: String,
    /// Original source directory used for the install.
    pub source: String,
}
