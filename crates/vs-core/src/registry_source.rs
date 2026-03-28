use reqwest::blocking::Client;
use serde::Deserialize;
use vs_plugin_api::PluginBackendKind;
use vs_registry::RegistryEntry;

use crate::CoreError;

/// Official vfox Lua plugin registry index.
pub const DEFAULT_VFOX_REGISTRY_SOURCE: &str =
    "https://version-fox.github.io/vfox-plugins/index.json";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VfoxRegistryEntry {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default, alias = "download_url")]
    download_url: Option<String>,
    #[serde(default)]
    aliases: Vec<String>,
}

/// Returns whether a registry source is remote.
pub fn is_remote_registry_source(source: &str) -> bool {
    source.starts_with("http://") || source.starts_with("https://")
}

/// Downloads text from a remote source.
pub fn fetch_url_text(url: &str) -> Result<String, CoreError> {
    let client = Client::builder()
        .user_agent(format!("vs/{}", env!("CARGO_PKG_VERSION")))
        .build()?;
    let response = client.get(url).send()?.error_for_status()?;
    response.text().map_err(Into::into)
}

/// Downloads bytes from a remote source.
pub fn fetch_url_bytes(url: &str) -> Result<Vec<u8>, CoreError> {
    let client = Client::builder()
        .user_agent(format!("vs/{}", env!("CARGO_PKG_VERSION")))
        .build()?;
    let response = client.get(url).send()?.error_for_status()?;
    response
        .bytes()
        .map(|bytes| bytes.to_vec())
        .map_err(Into::into)
}

/// Parses registry JSON into `RegistryEntry` values.
pub fn parse_registry_entries(json: &str) -> Result<Vec<RegistryEntry>, serde_json::Error> {
    match serde_json::from_str::<Vec<RegistryEntry>>(json) {
        Ok(entries) => Ok(entries),
        Err(entry_error) => match serde_json::from_str::<Vec<VfoxRegistryEntry>>(json) {
            Ok(entries) => Ok(entries
                .into_iter()
                .filter_map(|entry| {
                    entry.download_url.map(|source| RegistryEntry {
                        name: entry.name,
                        source,
                        backend: PluginBackendKind::Lua,
                        description: entry.description,
                        aliases: entry.aliases,
                    })
                })
                .collect()),
            Err(_) => Err(entry_error),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::{DEFAULT_VFOX_REGISTRY_SOURCE, is_remote_registry_source, parse_registry_entries};

    #[test]
    fn parse_registry_entries_should_accept_vfox_index_shape() -> Result<(), Box<dyn Error>> {
        let entries = parse_registry_entries(
            r#"
            [
              {
                "name": "nodejs",
                "description": "Node.js runtime",
                "downloadUrl": "https://example.com/nodejs.zip"
              }
            ]
            "#,
        )?;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "nodejs");
        assert_eq!(entries[0].source, "https://example.com/nodejs.zip");
        Ok(())
    }

    #[test]
    fn default_registry_source_should_be_remote() -> Result<(), Box<dyn Error>> {
        assert!(is_remote_registry_source(DEFAULT_VFOX_REGISTRY_SOURCE));
        Ok(())
    }
}
