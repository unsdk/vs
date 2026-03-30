//! Services for reporting version and build metadata.

use crate::{App, CoreError, VersionInfo};

const BUILD_TARGET_TRIPLE: &str = env!("VS_BUILD_TARGET");
const BUILD_RELEASE_VARIANT: &str = env!("VS_BUILD_VARIANT");
const BUILD_RELEASE_ARCHIVE_EXT: &str = env!("VS_BUILD_ARCHIVE_EXT");

impl App {
    /// Returns version and build metadata for the current binary.
    pub fn version_info(&self) -> Result<VersionInfo, CoreError> {
        Ok(VersionInfo {
            current_version: format!("v{}", env!("CARGO_PKG_VERSION")),
            build_target: release_target_triple().to_string(),
            build_variant: release_feature_label()?.to_string(),
            archive_extension: release_archive_extension().to_string(),
        })
    }
}

pub(crate) fn release_target_triple() -> &'static str {
    BUILD_TARGET_TRIPLE
}

pub(crate) fn release_archive_extension() -> &'static str {
    BUILD_RELEASE_ARCHIVE_EXT
}

pub(crate) fn release_feature_label() -> Result<&'static str, CoreError> {
    match BUILD_RELEASE_VARIANT {
        "full" => Ok("full"),
        "lua" => Ok("lua"),
        "wasi" => Ok("wasi"),
        "bare" => Err(CoreError::Unsupported(String::from(
            "self-upgrade is unavailable for bare builds because releases only publish lua, wasi, and full binaries",
        ))),
        other => Err(CoreError::Unsupported(format!(
            "self-upgrade was built with an unknown release variant: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BUILD_RELEASE_VARIANT, BUILD_TARGET_TRIPLE, release_feature_label, release_target_triple,
    };

    #[test]
    fn build_metadata_should_be_injected_at_compile_time() {
        assert_eq!(release_target_triple(), BUILD_TARGET_TRIPLE);
        match BUILD_RELEASE_VARIANT {
            "full" | "lua" | "wasi" | "bare" => {}
            other => panic!("unexpected build variant {other}"),
        }
    }

    #[test]
    fn build_variant_should_map_to_supported_labels() {
        let label = release_feature_label();
        match BUILD_RELEASE_VARIANT {
            "bare" => assert!(label.is_err()),
            _ => assert!(label.is_ok()),
        }
    }
}
