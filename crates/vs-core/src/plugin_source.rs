use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use tempfile::Builder;
use vs_registry::RegistryEntry;
use vs_shell::remove_existing;
use zip::ZipArchive;

use crate::registry_source::fetch_url_bytes;
use crate::{App, CoreError};

/// Returns whether a source string points to a remote HTTP resource.
pub(crate) fn is_remote_source(source: &str) -> bool {
    source.starts_with("http://") || source.starts_with("https://")
}

impl App {
    pub(crate) fn materialize_plugin_entry(
        &self,
        entry: &RegistryEntry,
    ) -> Result<RegistryEntry, CoreError> {
        self.ensure_backend_supported(entry.backend)?;
        if is_remote_source(&entry.source) {
            let local_source = self.download_plugin_archive(&entry.name, &entry.source)?;
            return Ok(RegistryEntry {
                source: local_source.display().to_string(),
                ..entry.clone()
            });
        }

        Ok(RegistryEntry {
            source: PathBuf::from(&entry.source)
                .canonicalize()
                .unwrap_or_else(|_| self.normalize_source_path(&entry.source))
                .display()
                .to_string(),
            ..entry.clone()
        })
    }

    fn plugin_sources_dir(&self) -> PathBuf {
        self.home().join("plugins").join("sources")
    }

    fn download_plugin_archive(&self, name: &str, url: &str) -> Result<PathBuf, CoreError> {
        let final_dir = self.plugin_sources_dir().join(name);
        if final_dir.exists() {
            return Ok(final_dir);
        }

        let archive_url = normalize_remote_plugin_archive_url(url);
        let archive_bytes = fetch_url_bytes(&archive_url)?;
        let staging_root = self.plugin_sources_dir().join(".staging");
        fs::create_dir_all(&staging_root)?;
        let temp_dir = Builder::new().prefix("plugin-").tempdir_in(&staging_root)?;
        let archive_path = temp_dir.path().join("plugin.zip");
        let mut archive_file = fs::File::create(&archive_path)?;
        archive_file.write_all(&archive_bytes)?;

        let unpack_dir = temp_dir.path().join("unpacked");
        fs::create_dir_all(&unpack_dir)?;
        extract_zip_archive(&archive_bytes, &unpack_dir)?;
        let extracted_root = normalize_extracted_root(&unpack_dir)?;

        if let Some(parent) = final_dir.parent() {
            fs::create_dir_all(parent)?;
        }
        remove_existing(&final_dir)?;
        fs::rename(&extracted_root, &final_dir)?;
        Ok(final_dir)
    }
}

fn extract_zip_archive(bytes: &[u8], destination: &Path) -> Result<(), CoreError> {
    let reader = Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader)?;

    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let Some(relative_path) = file.enclosed_name() else {
            continue;
        };
        let output_path = destination.join(relative_path);
        if file.name().ends_with('/') {
            fs::create_dir_all(&output_path)?;
            continue;
        }
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output_file = fs::File::create(&output_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        output_file.write_all(&buffer)?;
    }

    Ok(())
}

fn normalize_extracted_root(destination: &Path) -> Result<PathBuf, CoreError> {
    let mut entries = fs::read_dir(destination)?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    if entries.len() == 1 && entries[0].file_type()?.is_dir() {
        return Ok(entries.swap_remove(0).path());
    }
    Ok(destination.to_path_buf())
}

fn normalize_remote_plugin_archive_url(source: &str) -> String {
    github_repo_zipball_url(source).unwrap_or_else(|| source.to_string())
}

fn github_repo_zipball_url(source: &str) -> Option<String> {
    let source = source.trim_end_matches('/');
    let source = source.strip_suffix(".git").unwrap_or(source);

    let stripped = source
        .strip_prefix("https://github.com/")
        .or_else(|| source.strip_prefix("http://github.com/"))?;
    let mut parts = stripped.split('/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some(format!(
        "https://api.github.com/repos/{owner}/{repo}/zipball"
    ))
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::{github_repo_zipball_url, is_remote_source, normalize_remote_plugin_archive_url};

    #[test]
    fn is_remote_source_should_match_http_and_https() -> Result<(), Box<dyn Error>> {
        assert!(is_remote_source("https://example.com/plugin.zip"));
        assert!(is_remote_source("http://example.com/plugin.zip"));
        assert!(!is_remote_source("/tmp/plugin"));
        assert!(!is_remote_source("../plugin"));
        Ok(())
    }

    #[test]
    fn github_repo_zipball_url_should_convert_repository_homepage() -> Result<(), Box<dyn Error>> {
        let archive_url = github_repo_zipball_url("https://github.com/version-fox/vfox-nodejs")
            .ok_or_else(|| std::io::Error::other("missing github archive url"))?;
        assert_eq!(
            archive_url,
            "https://api.github.com/repos/version-fox/vfox-nodejs/zipball"
        );
        Ok(())
    }

    #[test]
    fn normalize_remote_plugin_archive_url_should_keep_archive_urls() -> Result<(), Box<dyn Error>>
    {
        let archive_url = normalize_remote_plugin_archive_url("https://example.com/plugin.zip");
        assert_eq!(archive_url, "https://example.com/plugin.zip");
        Ok(())
    }
}
