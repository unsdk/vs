//! Services for discovering and applying self-upgrades.

use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use serde::Deserialize;
use tar::Archive;
use tempfile::Builder;
use zip::ZipArchive;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinuxLibc {
    Gnu,
    Musl,
}

impl App {
    /// Returns the current and latest available self-upgrade versions.
    pub fn self_upgrade_summary(&self) -> Result<SelfUpgradeSummary, CoreError> {
        let current_version = format!("v{}", env!("CARGO_PKG_VERSION"));
        let latest_version = fetch_latest_release()?.tag_name;
        Ok(SelfUpgradeSummary {
            updated: current_version != latest_version,
            current_version,
            latest_version,
        })
    }

    /// Upgrades the running `vs` binary to the provided published release version.
    pub fn upgrade_self_to(&self, latest_version: &str) -> Result<SelfUpgradeSummary, CoreError> {
        let current_version = format!("v{}", env!("CARGO_PKG_VERSION"));
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

        let release = fetch_release_by_tag(latest_version)?;
        let asset = select_release_asset(&release)?;
        println!(
            "Resolved release asset {} for target {} with feature {}.",
            asset.name,
            release_target_triple()?,
            release_feature_label()?
        );

        let archive_name = &asset.name;
        let archive_path = temp_dir.path().join(archive_name);
        println!(
            "Downloading {} to {}...",
            asset.browser_download_url,
            archive_path.display()
        );
        download_release_archive(&asset.browser_download_url, &archive_path)?;

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

fn fetch_latest_release() -> Result<ReleaseMetadata, CoreError> {
    fetch_release_from_endpoint(&format!(
        "https://api.github.com/repos/{RELEASE_REPOSITORY}/releases/latest"
    ))
}

fn fetch_release_by_tag(tag: &str) -> Result<ReleaseMetadata, CoreError> {
    fetch_release_from_endpoint(&format!(
        "https://api.github.com/repos/{RELEASE_REPOSITORY}/releases/tags/{tag}"
    ))
}

fn fetch_release_from_endpoint(url: &str) -> Result<ReleaseMetadata, CoreError> {
    let response = github_client()?.get(url).send()?.error_for_status()?;
    response.json::<ReleaseMetadata>().map_err(Into::into)
}

fn github_client() -> Result<Client, CoreError> {
    Client::builder()
        .user_agent(format!("vs/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(Into::into)
}

fn select_release_asset(release: &ReleaseMetadata) -> Result<&ReleaseAsset, CoreError> {
    let target = release_target_triple()?;
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

fn download_release_archive(url: &str, archive_path: &Path) -> Result<(), CoreError> {
    let mut response = github_client()?.get(url).send()?.error_for_status()?;
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
    let target = release_target_triple()?;
    let feature = release_feature_label()?;
    Ok(format!(
        "vs-{tag}-{target}-{feature}.{}",
        release_archive_extension(target)
    ))
}

fn release_target_triple() -> Result<&'static str, CoreError> {
    let linux_libc = if std::env::consts::OS == "linux" {
        Some(detect_linux_libc())
    } else {
        None
    };

    release_target_triple_for(
        std::env::consts::OS,
        std::env::consts::ARCH,
        linux_libc,
        cfg!(target_endian = "little"),
    )
    .ok_or_else(|| {
        CoreError::Unsupported(format!(
            "no published release asset matches target {}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ))
    })
}

fn release_archive_extension(target: &str) -> &'static str {
    if target.contains("windows") {
        "zip"
    } else {
        "tar.gz"
    }
}

fn release_feature_label() -> Result<&'static str, CoreError> {
    #[cfg(any(feature = "full", all(feature = "lua", feature = "wasi")))]
    {
        Ok("full")
    }
    #[cfg(all(feature = "lua", not(feature = "wasi")))]
    {
        Ok("lua")
    }
    #[cfg(all(feature = "wasi", not(feature = "lua")))]
    {
        Ok("wasi")
    }
    #[cfg(not(any(feature = "lua", feature = "wasi")))]
    {
        Err(CoreError::Unsupported(String::from(
            "self-upgrade is unavailable for bare builds because releases only publish lua, wasi, and full binaries",
        )))
    }
}

fn release_target_triple_for(
    os: &str,
    arch: &str,
    linux_libc: Option<LinuxLibc>,
    little_endian: bool,
) -> Option<&'static str> {
    match (os, arch, linux_libc) {
        ("linux", "x86_64", Some(LinuxLibc::Gnu)) => Some("x86_64-unknown-linux-gnu"),
        ("linux", "x86_64", Some(LinuxLibc::Musl)) => Some("x86_64-unknown-linux-musl"),
        ("linux", "x86", Some(LinuxLibc::Gnu)) => Some("i686-unknown-linux-gnu"),
        ("linux", "x86", Some(LinuxLibc::Musl)) => Some("i686-unknown-linux-musl"),
        ("linux", "aarch64", Some(LinuxLibc::Gnu)) => Some("aarch64-unknown-linux-gnu"),
        ("linux", "aarch64", Some(LinuxLibc::Musl)) => Some("aarch64-unknown-linux-musl"),
        ("linux", "arm", Some(LinuxLibc::Gnu)) => Some("armv7-unknown-linux-gnueabihf"),
        ("linux", "arm", Some(LinuxLibc::Musl)) => Some("armv7-unknown-linux-musleabihf"),
        ("linux", "powerpc64", Some(LinuxLibc::Gnu)) if little_endian => {
            Some("powerpc64le-unknown-linux-gnu")
        }
        ("linux", "riscv64", Some(LinuxLibc::Gnu)) => Some("riscv64gc-unknown-linux-gnu"),
        ("linux", "s390x", Some(LinuxLibc::Gnu)) => Some("s390x-unknown-linux-gnu"),
        ("macos", "aarch64", _) => Some("aarch64-apple-darwin"),
        ("macos", "x86_64", _) => Some("x86_64-apple-darwin"),
        ("windows", "x86_64", _) => Some("x86_64-pc-windows-msvc"),
        ("windows", "x86", _) => Some("i686-pc-windows-msvc"),
        ("windows", "aarch64", _) => Some("aarch64-pc-windows-msvc"),
        _ => None,
    }
}

fn detect_linux_libc() -> LinuxLibc {
    if Path::new("/etc/alpine-release").exists() {
        return LinuxLibc::Musl;
    }

    let output = Command::new("ldd").arg("--version").output();
    let mut combined = String::new();
    if let Ok(output) = output {
        combined.push_str(&String::from_utf8_lossy(&output.stdout));
        combined.push_str(&String::from_utf8_lossy(&output.stderr));
    }

    if combined.to_ascii_lowercase().contains("musl") {
        LinuxLibc::Musl
    } else {
        LinuxLibc::Gnu
    }
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
        LinuxLibc, ReleaseAsset, ReleaseMetadata, release_archive_name, release_feature_label,
        release_target_triple, release_target_triple_for, select_release_asset,
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
        let target = match release_target_triple() {
            Ok(target) => target,
            Err(error) => panic!("target triple should resolve: {error}"),
        };
        assert!(archive_name.contains(target));
    }

    #[test]
    fn release_target_triple_should_support_linux_musl_variants() {
        let triple = release_target_triple_for("linux", "x86_64", Some(LinuxLibc::Musl), true);
        assert_eq!(triple, Some("x86_64-unknown-linux-musl"));
    }

    #[test]
    fn release_target_triple_should_reject_unpublished_linux_musl_targets() {
        let triple = release_target_triple_for("linux", "riscv64", Some(LinuxLibc::Musl), true);
        assert_eq!(triple, None);
    }
}
