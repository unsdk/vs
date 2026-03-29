use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use serde::Deserialize;
use tar::Archive;
use tempfile::Builder;
use zip::ZipArchive;

use crate::{App, CoreError, SelfUpgradeSummary};

const RELEASE_REPOSITORY: &str = "unsdk/vs";

#[derive(Debug, Deserialize)]
struct LatestRelease {
    tag_name: String,
}

impl App {
    /// Upgrades the running `vs` binary to the latest published release.
    pub fn upgrade_self(&self) -> Result<SelfUpgradeSummary, CoreError> {
        let current_version = format!("v{}", env!("CARGO_PKG_VERSION"));
        let latest_version = fetch_latest_release_tag()?;
        if current_version == latest_version {
            return Ok(SelfUpgradeSummary {
                current_version,
                latest_version,
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
        let archive_name = release_archive_name(&latest_version);
        let archive_path = temp_dir.path().join(&archive_name);
        let download_url = release_asset_url(&latest_version);

        let bytes = download_release_bytes(&download_url)?;
        fs::write(&archive_path, bytes)?;

        let replacement = if cfg!(windows) {
            extract_zip_binary(&archive_path, temp_dir.path())?
        } else {
            extract_tar_gz_binary(&archive_path, temp_dir.path())?
        };

        replace_running_executable(&executable, &replacement)?;

        Ok(SelfUpgradeSummary {
            current_version,
            latest_version,
            updated: true,
        })
    }
}

fn fetch_latest_release_tag() -> Result<String, CoreError> {
    let client = Client::builder()
        .user_agent(format!("vs/{}", env!("CARGO_PKG_VERSION")))
        .build()?;
    let response = client
        .get(format!(
            "https://api.github.com/repos/{RELEASE_REPOSITORY}/releases/latest"
        ))
        .send()?
        .error_for_status()?;
    let release = response.json::<LatestRelease>()?;
    Ok(release.tag_name)
}

fn download_release_bytes(url: &str) -> Result<Vec<u8>, CoreError> {
    let client = Client::builder()
        .user_agent(format!("vs/{}", env!("CARGO_PKG_VERSION")))
        .build()?;
    let response = client.get(url).send()?.error_for_status()?;
    response
        .bytes()
        .map(|bytes| bytes.to_vec())
        .map_err(Into::into)
}

fn release_asset_url(tag: &str) -> String {
    format!(
        "https://github.com/{RELEASE_REPOSITORY}/releases/download/{tag}/{}",
        release_archive_name(tag)
    )
}

fn release_archive_name(tag: &str) -> String {
    format!(
        "vs-{tag}-{}-{}.{}",
        release_target_triple(),
        release_feature_label(),
        release_archive_extension()
    )
}

fn release_target_triple() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "x86") => "i686-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("linux", "arm") => "armv7-unknown-linux-gnueabihf",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        ("windows", "x86") => "i686-pc-windows-msvc",
        ("windows", "aarch64") => "aarch64-pc-windows-msvc",
        _ => "x86_64-unknown-linux-gnu",
    }
}

fn release_archive_extension() -> &'static str {
    if cfg!(windows) { "zip" } else { "tar.gz" }
}

fn release_feature_label() -> &'static str {
    #[cfg(feature = "full")]
    {
        "full"
    }
    #[cfg(all(feature = "lua", not(feature = "wasi")))]
    {
        "lua"
    }
    #[cfg(all(feature = "wasi", not(feature = "lua")))]
    {
        "wasi"
    }
    #[cfg(not(any(feature = "lua", feature = "wasi")))]
    {
        "bare"
    }
}

fn extract_tar_gz_binary(archive_path: &Path, destination: &Path) -> Result<PathBuf, CoreError> {
    let bytes = fs::read(archive_path)?;
    let cursor = Cursor::new(bytes);
    let decoder = GzDecoder::new(cursor);
    let mut archive = Archive::new(decoder);
    archive.unpack(destination)?;
    let binary_name = executable_name();
    let extracted = destination.join(binary_name);
    if extracted.exists() {
        return Ok(extracted);
    }
    Err(CoreError::Unsupported(format!(
        "failed to find extracted binary {}",
        extracted.display()
    )))
}

fn extract_zip_binary(archive_path: &Path, destination: &Path) -> Result<PathBuf, CoreError> {
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
        fs::rename(executable, &backup)?;
        fs::rename(replacement, executable)?;

        let cleanup_script = executable.with_extension("cleanup.bat");
        let script = format!(
            ":Repeat\r\ndel \"{}\"\r\nif exist \"{}\" goto Repeat\r\ndel \"{}\"\r\n",
            backup.display(),
            backup.display(),
            cleanup_script.display()
        );
        fs::write(&cleanup_script, script)?;
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
        fs::rename(replacement, executable)?;
        let mut permissions = fs::metadata(executable)?.permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            permissions.set_mode(0o755);
        }
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
        release_archive_name, release_asset_url, release_feature_label, release_target_triple,
    };

    #[test]
    fn release_asset_url_should_reference_current_repository() {
        let url = release_asset_url("v1.2.3");
        assert!(url.contains("github.com/unsdk/vs/releases/download/v1.2.3/"));
        assert!(url.contains("v1.2.3"));
    }

    #[test]
    fn release_archive_name_should_include_feature_variant() {
        let archive_name = release_archive_name("v1.2.3");
        assert!(archive_name.contains(release_feature_label()));
    }

    #[test]
    fn release_archive_name_should_include_target_triple() {
        let archive_name = release_archive_name("v1.2.3");
        assert!(archive_name.contains(release_target_triple()));
    }
}
