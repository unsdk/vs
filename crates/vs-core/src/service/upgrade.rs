//! Services for discovering and applying self-upgrades.

use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Proxy;
use reqwest::blocking::Client;
use serde::Deserialize;
use tar::Archive;
use tempfile::Builder;
use zip::ZipArchive;

use super::version::{release_archive_extension, release_feature_label, release_target_triple};
use crate::{App, CoreError, SelfUpgradeSummary};

const RELEASE_REPOSITORY: &str = "unsdk/vs";

#[derive(Debug, Deserialize)]
struct ReleaseMetadata {
    tag_name: String,
    #[serde(default)]
    assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

impl App {
    /// Returns the current and latest available self-upgrade versions.
    pub fn self_upgrade_summary(&self) -> Result<SelfUpgradeSummary, CoreError> {
        let current_version = self.version_info()?.current_version;
        let latest_version = fetch_latest_release(self.proxy_url())?.tag_name;
        Ok(SelfUpgradeSummary {
            updated: current_version != latest_version,
            current_version,
            latest_version,
        })
    }

    /// Upgrades the running `vs` binary to the provided published release version.
    pub fn upgrade_self_to(&self, latest_version: &str) -> Result<SelfUpgradeSummary, CoreError> {
        let current_version = self.version_info()?.current_version;
        if current_version == latest_version {
            return Ok(SelfUpgradeSummary {
                current_version,
                latest_version: latest_version.to_string(),
                updated: false,
            });
        }

        let executable = std::env::current_exe()?;
        let executable_dir = executable.parent().ok_or_else(|| {
            CoreError::Unsupported(String::from("failed to resolve executable directory"))
        })?;
        let temp_dir = Builder::new()
            .prefix("vs-upgrade-")
            .tempdir_in(executable_dir)?;
        println!(
            "Preparing upgrade workspace in {}...",
            temp_dir.path().display()
        );

        let release = fetch_release_by_tag(latest_version, self.proxy_url())?;
        let asset = select_release_asset(&release)?;
        println!(
            "Resolved release asset {} for target {} with feature {}.",
            asset.name,
            release_target_triple(),
            release_feature_label()?
        );

        let archive_name = &asset.name;
        let archive_path = temp_dir.path().join(archive_name);
        println!(
            "Downloading {} to {}...",
            asset.browser_download_url,
            archive_path.display()
        );
        download_release_archive(&asset.browser_download_url, &archive_path, self.proxy_url())?;

        let unpack_dir = temp_dir.path().join("unpacked");
        let replacement = if cfg!(windows) {
            extract_zip_binary(&archive_path, &unpack_dir)?
        } else {
            extract_tar_gz_binary(&archive_path, &unpack_dir)?
        };

        replace_running_executable(&executable, &replacement)?;

        Ok(SelfUpgradeSummary {
            current_version,
            latest_version: latest_version.to_string(),
            updated: true,
        })
    }
}

fn fetch_latest_release(proxy_url: Option<&str>) -> Result<ReleaseMetadata, CoreError> {
    fetch_release_from_endpoint(
        &format!("https://api.github.com/repos/{RELEASE_REPOSITORY}/releases/latest"),
        proxy_url,
    )
}

fn fetch_release_by_tag(tag: &str, proxy_url: Option<&str>) -> Result<ReleaseMetadata, CoreError> {
    fetch_release_from_endpoint(
        &format!("https://api.github.com/repos/{RELEASE_REPOSITORY}/releases/tags/{tag}"),
        proxy_url,
    )
}

fn fetch_release_from_endpoint(
    url: &str,
    proxy_url: Option<&str>,
) -> Result<ReleaseMetadata, CoreError> {
    let response = github_client(proxy_url)?
        .get(url)
        .send()?
        .error_for_status()?;
    response.json::<ReleaseMetadata>().map_err(Into::into)
}

fn github_client(proxy_url: Option<&str>) -> Result<Client, CoreError> {
    let mut builder = Client::builder().user_agent(format!("vs/{}", env!("CARGO_PKG_VERSION")));
    if let Some(proxy_url) = proxy_url {
        builder = builder.proxy(Proxy::all(proxy_url)?);
    }
    builder.build().map_err(Into::into)
}

fn select_release_asset(release: &ReleaseMetadata) -> Result<&ReleaseAsset, CoreError> {
    let target = release_target_triple();
    let feature = release_feature_label()?;
    let expected_name = release_archive_name(&release.tag_name)?;
    release.assets.iter().find(|asset| asset.name == expected_name).ok_or_else(|| {
        let available = release
            .assets
            .iter()
            .map(|asset| asset.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        CoreError::Unsupported(format!(
            "no release asset matched target {} with feature {}. expected {}, available assets: {}",
            target,
            feature,
            expected_name,
            available
        ))
    })
}

fn download_release_archive(
    url: &str,
    archive_path: &Path,
    proxy_url: Option<&str>,
) -> Result<(), CoreError> {
    let mut response = github_client(proxy_url)?
        .get(url)
        .send()?
        .error_for_status()?;
    let progress_bar = create_download_progress_bar(response.content_length());
    let mut output = fs::File::create(archive_path)?;
    let mut buffer = [0_u8; 8192];

    loop {
        let read = response.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        std::io::Write::write_all(&mut output, &buffer[..read])?;
        progress_bar.inc(read as u64);
    }

    progress_bar.finish_and_clear();
    Ok(())
}

fn create_download_progress_bar(total_size: Option<u64>) -> ProgressBar {
    let progress_bar = match total_size {
        Some(total_size) => ProgressBar::new(total_size),
        None => ProgressBar::new_spinner(),
    };
    let style = ProgressStyle::with_template(
        "Downloading... {wide_bar} {bytes}/{total_bytes} ({bytes_per_sec})",
    )
    .unwrap_or_else(|_| ProgressStyle::default_bar())
    .progress_chars("=> ");
    progress_bar.set_style(style);
    progress_bar
}

fn release_archive_name(tag: &str) -> Result<String, CoreError> {
    let target = release_target_triple();
    let feature = release_feature_label()?;
    Ok(format!(
        "vs-{tag}-{target}-{feature}.{}",
        release_archive_extension()
    ))
}

fn extract_tar_gz_binary(archive_path: &Path, destination: &Path) -> Result<PathBuf, CoreError> {
    println!(
        "Unpacking {} to {}...",
        archive_path.display(),
        destination.display()
    );
    fs::create_dir_all(destination)?;
    let bytes = fs::read(archive_path)?;
    let cursor = Cursor::new(bytes);
    let decoder = GzDecoder::new(cursor);
    let mut archive = Archive::new(decoder);
    archive.unpack(destination)?;
    let extracted = destination.join(executable_name());
    if extracted.exists() {
        println!("Extracted binary to {}.", extracted.display());
        return Ok(extracted);
    }
    Err(CoreError::Unsupported(format!(
        "failed to find extracted binary {}",
        extracted.display()
    )))
}

fn extract_zip_binary(archive_path: &Path, destination: &Path) -> Result<PathBuf, CoreError> {
    println!(
        "Unpacking {} to {}...",
        archive_path.display(),
        destination.display()
    );
    fs::create_dir_all(destination)?;
    let bytes = fs::read(archive_path)?;
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)?;
    let binary_name = executable_name();

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
        let mut output = fs::File::create(&output_path)?;
        std::io::copy(&mut file, &mut output)?;
        if output_path
            .file_name()
            .is_some_and(|name| name == binary_name)
        {
            println!("Extracted binary to {}.", output_path.display());
            return Ok(output_path);
        }
    }

    Err(CoreError::Unsupported(format!(
        "failed to find extracted binary {binary_name}"
    )))
}

fn replace_running_executable(executable: &Path, replacement: &Path) -> Result<(), CoreError> {
    #[cfg(windows)]
    {
        let backup = executable.with_extension("old.exe");
        if backup.exists() {
            fs::remove_file(&backup)?;
        }
        println!("Moving {} to {}...", executable.display(), backup.display());
        fs::rename(executable, &backup)?;
        println!(
            "Moving {} to {}...",
            replacement.display(),
            executable.display()
        );
        fs::rename(replacement, executable)?;

        let cleanup_script = executable.with_extension("cleanup.bat");
        let script = format!(
            ":Repeat\r\ndel \"{}\"\r\nif exist \"{}\" goto Repeat\r\ndel \"{}\"\r\n",
            backup.display(),
            backup.display(),
            cleanup_script.display()
        );
        println!("Writing cleanup script to {}...", cleanup_script.display());
        fs::write(&cleanup_script, script)?;
        println!("Starting cleanup helper process...");
        std::process::Command::new("cmd.exe")
            .args(["/C", cleanup_script.to_string_lossy().as_ref()])
            .spawn()
            .map_err(|error| CoreError::CommandExecution {
                command: String::from("cmd.exe"),
                message: error.to_string(),
            })?;
        Ok(())
    }

    #[cfg(not(windows))]
    {
        println!(
            "Moving {} to {}...",
            replacement.display(),
            executable.display()
        );
        fs::rename(replacement, executable)?;
        let mut permissions = fs::metadata(executable)?.permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            permissions.set_mode(0o755);
        }
        println!("Updating file permissions for {}...", executable.display());
        fs::set_permissions(executable, permissions)?;
        Ok(())
    }
}

fn executable_name() -> &'static str {
    if cfg!(windows) { "vs.exe" } else { "vs" }
}

#[cfg(test)]
mod tests {
    use super::{
        ReleaseAsset, ReleaseMetadata, release_archive_name, release_feature_label,
        release_target_triple, select_release_asset,
    };

    #[test]
    fn select_release_asset_should_match_expected_archive_name() {
        let archive_name = match release_archive_name("v1.2.3") {
            Ok(name) => name,
            Err(error) => panic!("release archive name should resolve: {error}"),
        };
        let release = ReleaseMetadata {
            tag_name: String::from("v1.2.3"),
            assets: vec![
                ReleaseAsset {
                    name: String::from("vs-v1.2.3-other-target-full.tar.gz"),
                    browser_download_url: String::from("https://example.com/other"),
                },
                ReleaseAsset {
                    name: archive_name,
                    browser_download_url: String::from("https://example.com/match"),
                },
            ],
        };

        let asset = match select_release_asset(&release) {
            Ok(asset) => asset,
            Err(error) => panic!("release asset should match: {error}"),
        };
        assert_eq!(asset.browser_download_url, "https://example.com/match");
    }

    #[test]
    fn release_archive_name_should_include_feature_variant() {
        let archive_name = match release_archive_name("v1.2.3") {
            Ok(name) => name,
            Err(error) => panic!("release archive name should resolve: {error}"),
        };
        let feature = match release_feature_label() {
            Ok(feature) => feature,
            Err(error) => panic!("feature label should resolve: {error}"),
        };
        assert!(archive_name.contains(feature));
    }

    #[test]
    fn release_archive_name_should_include_target_triple() {
        let archive_name = match release_archive_name("v1.2.3") {
            Ok(name) => name,
            Err(error) => panic!("release archive name should resolve: {error}"),
        };
        let target = release_target_triple();
        assert!(archive_name.contains(target));
    }
}
