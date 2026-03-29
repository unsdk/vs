#!/usr/bin/env bash
set -euxo pipefail

if [[ -n "${BUILD_TARGET:-}" ]]; then
  binary_path="target/$BUILD_TARGET/$BUILD_PROFILE/vs"
else
  binary_path="target/$BUILD_PROFILE/vs"
fi

archive_name="vs-v${VERSION}-${PLATFORM_LABEL}-${ARTIFACT_VARIANT}.tar.gz"
staging_dir="$(mktemp -d)"
cp "$binary_path" "$staging_dir/vs"
tar -C "$staging_dir" -czf "$archive_name" vs
echo "ARCHIVE_PATH=$archive_name" >>"$GITHUB_ENV"
