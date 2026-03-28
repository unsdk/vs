use reqwest::blocking::Client;
use serde::Deserialize;
use vs_plugin_api::PluginBackendKind;
use vs_registry::RegistryEntry;

use crate::CoreError;

/// Official vfox Lua plugin registry index.
#[cfg(any(feature = "lua", test))]
pub const DEFAULT_VFOX_REGISTRY_SOURCE: &str = "https://version-fox.github.io/vfox-plugins";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VfoxRegistryEntry {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    desc: Option<String>,
    #[serde(default, alias = "download_url")]
    download_url: Option<String>,
    #[serde(default)]
    homepage: Option<String>,
    #[serde(default)]
    aliases: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryPluginManifest {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, alias = "download_url")]
    pub download_url: String,
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

/// Builds the registry index URL from an address.
pub fn registry_index_url(address: &str) -> String {
    if address.ends_with(".json") {
        address.to_string()
    } else {
        format!("{}/index.json", address.trim_end_matches('/'))
    }
}

/// Builds the manifest URL for a plugin from a registry address.
pub fn registry_manifest_url(address: &str, plugin_name: &str) -> String {
    if address.ends_with("index.json") {
        let base = address.trim_end_matches("index.json").trim_end_matches('/');
        format!("{base}/{plugin_name}.json")
    } else if address.ends_with(".json") {
        let path = std::path::Path::new(address);
        path.with_file_name(format!("{plugin_name}.json"))
            .display()
            .to_string()
    } else {
        format!("{}/{}.json", address.trim_end_matches('/'), plugin_name)
    }
}

/// Fetches a registry plugin manifest.
pub fn fetch_plugin_manifest(
    address: &str,
    plugin_name: &str,
) -> Result<RegistryPluginManifest, CoreError> {
    let url = registry_manifest_url(address, plugin_name);
    let content = if is_remote_registry_source(&url) {
        fetch_url_text(&url)?
    } else {
        std::fs::read_to_string(&url)?
    };
    serde_json::from_str(&content).map_err(|error| CoreError::RegistrySource {
        path: url.into(),
        message: error.to_string(),
    })
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
                    entry
                        .download_url
                        .or(entry.homepage)
                        .map(|source| RegistryEntry {
                            name: entry.name,
                            source,
                            backend: PluginBackendKind::Lua,
                            description: entry.description.or(entry.desc),
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

    use super::{
        DEFAULT_VFOX_REGISTRY_SOURCE, is_remote_registry_source, parse_registry_entries,
        registry_index_url, registry_manifest_url,
    };

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
    fn parse_registry_entries_should_accept_real_vfox_index_shape() -> Result<(), Box<dyn Error>> {
        let entries = parse_registry_entries(
            r#"
            [
              {
                "name": "nodejs",
                "desc": "Node.js runtime environment.",
                "homepage": "https://github.com/version-fox/vfox-nodejs"
              }
            ]
            "#,
        )?;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "nodejs");
        assert_eq!(
            entries[0].source,
            "https://github.com/version-fox/vfox-nodejs"
        );
        assert_eq!(
            entries[0].description.as_deref(),
            Some("Node.js runtime environment.")
        );
        Ok(())
    }

    #[test]
    fn default_registry_source_should_be_remote() -> Result<(), Box<dyn Error>> {
        assert!(is_remote_registry_source(DEFAULT_VFOX_REGISTRY_SOURCE));
        Ok(())
    }

    #[test]
    fn registry_index_url_should_append_index_json() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            registry_index_url("https://version-fox.github.io/vfox-plugins"),
            "https://version-fox.github.io/vfox-plugins/index.json"
        );
        Ok(())
    }

    #[test]
    fn registry_manifest_url_should_build_plugin_manifest_path() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            registry_manifest_url("https://version-fox.github.io/vfox-plugins", "nodejs"),
            "https://version-fox.github.io/vfox-plugins/nodejs.json"
        );
        Ok(())
    }
}
