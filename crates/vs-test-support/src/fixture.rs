use std::path::PathBuf;

/// Returns the repository fixture root.
pub fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// Returns a plugin fixture by name.
pub fn plugin_fixture(name: &str) -> PathBuf {
    fixture_root().join("plugins").join(name)
}
