//! Artifact download and runtime installation helpers.

use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use indicatif::{ProgressBar, ProgressStyle};
use md5::Md5;
use reqwest::blocking::Client;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};
use tar::Archive;
use tempfile::Builder;
use vs_plugin_api::{
    Checksum, InstallArtifact, InstallPlan, InstallSource, InstalledArtifact, InstalledRuntime,
};
use xz2::read::XzDecoder;
use zip::ZipArchive;

use crate::InstallerError;
use crate::fs::copy_dir_all;
use crate::receipt::InstallReceipt;

/// Handles transactional runtime installs.
#[derive(Debug, Clone)]
pub struct Installer {
    home: PathBuf,
}

impl Installer {
    /// Creates a new installer rooted at the active home.
    pub fn new(home: impl Into<PathBuf>) -> Self {
        Self { home: home.into() }
    }

    fn versions_root(&self, plugin: &str) -> PathBuf {
        self.home.join("cache").join(plugin).join("versions")
    }

    fn receipt_path(install_dir: &Path) -> PathBuf {
        install_dir.join(".vs-receipt.json")
    }

    /// Returns the final install directory for a plugin version.
    pub fn install_dir(&self, plugin: &str, version: &str) -> PathBuf {
        self.versions_root(plugin).join(version)
    }

    /// Lists installed versions for a plugin.
    pub fn installed_versions(&self, plugin: &str) -> Result<Vec<String>, InstallerError> {
        let root = self.versions_root(plugin);
        if !root.exists() {
            return Ok(Vec::new());
        }
        let mut versions = fs::read_dir(root)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let file_type = entry.file_type().ok()?;
                if file_type.is_dir() {
                    entry.file_name().into_string().ok()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        versions.sort();
        Ok(versions)
    }

    /// Installs a version using a staging directory and atomic rename.
    pub fn install(&self, plan: &InstallPlan) -> Result<InstalledRuntime, InstallerError> {
        let destination = self.install_dir(&plan.plugin, &plan.version);
        if destination.exists() {
            return self
                .read_receipt(&plan.plugin, &plan.version)?
                .ok_or_else(|| {
                    InstallerError::Validation(String::from("install receipt is missing"))
                });
        }

        println!("Preinstalling {}@{}...", plan.plugin, plan.version);

        let staging_root = self.home.join("cache").join(&plan.plugin).join(".staging");
        fs::create_dir_all(&staging_root)?;
        let temp_dir = Builder::new().prefix("install-").tempdir_in(staging_root)?;
        let staged_install = temp_dir.path().join("runtime");
        fs::create_dir_all(&staged_install)?;

        let main = self.materialize_artifact(&plan.main, &staged_install, true)?;
        let mut additions = Vec::new();
        for artifact in &plan.additions {
            additions.push(self.materialize_artifact(artifact, &staged_install, false)?);
        }
        self.validate_staged_install(&staged_install)?;

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&staged_install, &destination)?;

        let receipt = InstallReceipt {
            plugin: plan.plugin.clone(),
            version: plan.version.clone(),
            root_dir: destination.clone(),
            main: InstalledArtifact {
                name: main.name,
                version: main.version,
                path: destination.join(main.relative_path),
                note: main.note,
            },
            additions: additions
                .into_iter()
                .map(|artifact| InstalledArtifact {
                    name: artifact.name,
                    version: artifact.version,
                    path: destination.join(artifact.relative_path),
                    note: artifact.note,
                })
                .collect(),
        };
        self.write_receipt(&destination, &receipt)?;
        Ok(receipt)
    }

    /// Uninstalls a version from the local cache.
    pub fn uninstall(&self, plugin: &str, version: &str) -> Result<bool, InstallerError> {
        let path = self.install_dir(plugin, version);
        if !path.exists() {
            return Ok(false);
        }
        fs::remove_dir_all(path)?;
        Ok(true)
    }

    /// Reads the install receipt for a version.
    pub fn read_receipt(
        &self,
        plugin: &str,
        version: &str,
    ) -> Result<Option<InstallReceipt>, InstallerError> {
        let path = Self::receipt_path(&self.install_dir(plugin, version));
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path)?;
        let receipt = serde_json::from_str(&content).map_err(|error| InstallerError::Json {
            path,
            message: error.to_string(),
        })?;
        Ok(Some(receipt))
    }

    fn materialize_artifact(
        &self,
        artifact: &InstallArtifact,
        version_root: &Path,
        is_main: bool,
    ) -> Result<ArtifactPlacement, InstallerError> {
        let relative_path = runtime_dir_name(artifact, is_main);
        let target_path = version_root.join(&relative_path);

        match &artifact.source {
            InstallSource::Directory { path } => {
                if !path.exists() {
                    return Err(InstallerError::MissingSource(path.clone()));
                }
                copy_dir_all(path, &target_path)?;
            }
            InstallSource::File { path } => {
                if !path.exists() {
                    return Err(InstallerError::MissingSource(path.clone()));
                }
                self.install_from_file(path, artifact.checksum.as_ref(), &target_path)?;
            }
            InstallSource::Url { url, headers } => {
                let bytes = download_bytes(url, headers)?;
                let temp_dir = self.home.join("downloads");
                fs::create_dir_all(&temp_dir)?;
                let temp_file = Builder::new().prefix("artifact-").tempfile_in(temp_dir)?;
                fs::write(temp_file.path(), &bytes)?;
                if let Some(checksum) = artifact.checksum.as_ref() {
                    verify_checksum(temp_file.path(), checksum)?;
                }
                self.install_from_download(url, &bytes, &target_path)?;
            }
        }

        Ok(ArtifactPlacement {
            name: artifact.name.clone(),
            version: artifact.version.clone(),
            relative_path,
            note: artifact.note.clone(),
        })
    }

    fn validate_staged_install(&self, staged_install: &Path) -> Result<(), InstallerError> {
        let has_failure_marker = walkdir::WalkDir::new(staged_install)
            .into_iter()
            .filter_map(Result::ok)
            .any(|entry| entry.file_name() == ".vs-fail-install");
        if has_failure_marker {
            return Err(InstallerError::Validation(String::from(
                "staged runtime requested a simulated install failure",
            )));
        }
        Ok(())
    }

    fn write_receipt(
        &self,
        install_dir: &Path,
        receipt: &InstallReceipt,
    ) -> Result<(), InstallerError> {
        let path = Self::receipt_path(install_dir);
        let rendered =
            serde_json::to_string_pretty(receipt).map_err(|error| InstallerError::Json {
                path: path.clone(),
                message: error.to_string(),
            })?;
        fs::write(path, rendered)?;
        Ok(())
    }

    fn install_from_file(
        &self,
        source_path: &Path,
        checksum: Option<&Checksum>,
        target_path: &Path,
    ) -> Result<(), InstallerError> {
        if let Some(checksum) = checksum {
            verify_checksum(source_path, checksum)?;
        }
        let bytes = fs::read(source_path)?;
        self.install_from_download(&source_path.display().to_string(), &bytes, target_path)
    }

    fn install_from_download(
        &self,
        source_name: &str,
        bytes: &[u8],
        target_path: &Path,
    ) -> Result<(), InstallerError> {
        match detect_archive_kind(source_name) {
            ArchiveKind::Zip => extract_zip(bytes, target_path)?,
            ArchiveKind::TarGz => extract_tar_gz(bytes, target_path)?,
            ArchiveKind::TarXz => extract_tar_xz(bytes, target_path)?,
            ArchiveKind::Tar => extract_tar(bytes, target_path)?,
            ArchiveKind::PlainFile => {
                fs::create_dir_all(target_path)?;
                let file_name =
                    artifact_file_name(source_name).unwrap_or_else(|| String::from("artifact"));
                fs::write(target_path.join(file_name), bytes)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct ArtifactPlacement {
    name: String,
    version: String,
    relative_path: PathBuf,
    note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArchiveKind {
    Zip,
    TarGz,
    TarXz,
    Tar,
    PlainFile,
}

fn runtime_dir_name(artifact: &InstallArtifact, is_main: bool) -> PathBuf {
    let directory_name = if is_main {
        if artifact.version.is_empty() {
            artifact.name.clone()
        } else {
            format!("{}-{}", artifact.name, artifact.version)
        }
    } else if artifact.version.is_empty() {
        format!("add-{}", artifact.name)
    } else {
        format!("add-{}-{}", artifact.name, artifact.version)
    };
    PathBuf::from(directory_name)
}

fn detect_archive_kind(source_name: &str) -> ArchiveKind {
    let name = archive_name_hint(source_name);
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        ArchiveKind::TarGz
    } else if name.ends_with(".tar.xz") {
        ArchiveKind::TarXz
    } else if name.ends_with(".tar") {
        ArchiveKind::Tar
    } else if name.ends_with(".zip") {
        ArchiveKind::Zip
    } else {
        ArchiveKind::PlainFile
    }
}

fn archive_name_hint(source_name: &str) -> String {
    if let Some((_, fragment)) = source_name.rsplit_once("#/") {
        return fragment.to_string();
    }
    source_name
        .rsplit('/')
        .next()
        .unwrap_or(source_name)
        .to_string()
}

fn artifact_file_name(source_name: &str) -> Option<String> {
    let hint = archive_name_hint(source_name);
    let candidate = hint.split('?').next().unwrap_or(&hint).trim();
    if candidate.is_empty() {
        None
    } else {
        Some(candidate.to_string())
    }
}

fn download_bytes(
    url: &str,
    headers: &std::collections::BTreeMap<String, String>,
) -> Result<Vec<u8>, InstallerError> {
    let client = Client::builder()
        .user_agent(format!("vs/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| InstallerError::Download(error.to_string()))?;
    let mut request = client.get(url);
    for (key, value) in headers {
        request = request.header(key, value);
    }
    let response = request
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| InstallerError::Download(error.to_string()))?;
    let total_size = response.content_length();
    let progress_bar = create_download_progress_bar(total_size);
    let mut response = response;
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let read = response
            .read(&mut buffer)
            .map_err(|error| InstallerError::Download(error.to_string()))?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        progress_bar.inc(read as u64);
    }

    progress_bar.finish_and_clear();
    Ok(bytes)
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

fn verify_checksum(path: &Path, checksum: &Checksum) -> Result<(), InstallerError> {
    println!("Verifying checksum {}...", checksum.value);
    let bytes = fs::read(path)?;
    let actual = match checksum.algorithm.as_str() {
        "sha256" => format!("{:x}", Sha256::digest(&bytes)),
        "sha512" => format!("{:x}", Sha512::digest(&bytes)),
        "sha1" => format!("{:x}", Sha1::digest(&bytes)),
        "md5" => format!("{:x}", Md5::digest(&bytes)),
        other => {
            return Err(InstallerError::Validation(format!(
                "unsupported checksum algorithm: {other}"
            )));
        }
    };
    if actual.eq_ignore_ascii_case(&checksum.value) {
        Ok(())
    } else {
        Err(InstallerError::Validation(format!(
            "checksum mismatch for {}",
            path.display()
        )))
    }
}

fn extract_zip(bytes: &[u8], target_path: &Path) -> Result<(), InstallerError> {
    println!("Unpacking {}...", target_path.display());
    fs::create_dir_all(target_path)?;
    let mut archive = ZipArchive::new(Cursor::new(bytes))?;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let Some(relative_path) = file.enclosed_name() else {
            continue;
        };
        let output_path = target_path.join(relative_path);
        if file.name().ends_with('/') {
            fs::create_dir_all(&output_path)?;
            continue;
        }
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = fs::File::create(output_path)?;
        std::io::copy(&mut file, &mut output)?;
    }
    flatten_extracted_root(target_path)?;
    Ok(())
}

fn extract_tar(bytes: &[u8], target_path: &Path) -> Result<(), InstallerError> {
    println!("Unpacking {}...", target_path.display());
    fs::create_dir_all(target_path)?;
    extract_tar_archive(Archive::new(Cursor::new(bytes)), target_path)
}

fn extract_tar_gz(bytes: &[u8], target_path: &Path) -> Result<(), InstallerError> {
    println!("Unpacking {}...", target_path.display());
    fs::create_dir_all(target_path)?;
    let decoder = flate2::read::GzDecoder::new(Cursor::new(bytes));
    extract_tar_archive(Archive::new(decoder), target_path)
}

fn extract_tar_xz(bytes: &[u8], target_path: &Path) -> Result<(), InstallerError> {
    println!("Unpacking {}...", target_path.display());
    fs::create_dir_all(target_path)?;
    let decoder = XzDecoder::new(Cursor::new(bytes));
    extract_tar_archive(Archive::new(decoder), target_path)
}

fn extract_tar_archive<R: Read>(
    mut archive: Archive<R>,
    target_path: &Path,
) -> Result<(), InstallerError> {
    for entry in archive.entries()? {
        let mut entry = entry?;
        entry.unpack_in(target_path)?;
    }
    flatten_extracted_root(target_path)?;
    Ok(())
}

fn flatten_extracted_root(target_path: &Path) -> Result<(), InstallerError> {
    let mut entries = fs::read_dir(target_path)?.collect::<Result<Vec<_>, _>>()?;
    if entries.len() != 1 {
        return Ok(());
    }

    let root = entries.swap_remove(0);
    if !root.file_type()?.is_dir() {
        return Ok(());
    }

    let root_path = root.path();
    let root_name = root.file_name();
    if !should_flatten_archive_root(root_name.to_string_lossy().as_ref()) {
        return Ok(());
    }

    for child in fs::read_dir(&root_path)? {
        let child = child?;
        let destination = target_path.join(child.file_name());
        fs::rename(child.path(), destination)?;
    }
    fs::remove_dir(&root_path)?;
    Ok(())
}

fn should_flatten_archive_root(root_name: &str) -> bool {
    !matches!(
        root_name,
        "bin"
            | "lib"
            | "lib64"
            | "include"
            | "share"
            | "etc"
            | "usr"
            | "opt"
            | "Scripts"
            | "script"
            | "cmd"
            | "completions"
    )
}
