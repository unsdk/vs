#!/usr/bin/env bash
set -euo pipefail

REPOSITORY="${VS_RELEASE_REPOSITORY:-unsdk/vs}"
INSTALL_DIR="${VS_INSTALL_DIR:-$HOME/.local/bin}"
REQUESTED_VERSION="${VS_INSTALL_VERSION:-latest}"
VARIANT="${VS_INSTALL_VARIANT:-full}"
TARGET="${VS_INSTALL_TARGET:-}"
TMP_DIR=""

usage() {
  cat <<'EOF'
Install the `vs` binary from GitHub Releases.

Usage:
  install.sh [--version <latest|vX.Y.Z|X.Y.Z>] [--variant <full|lua|wasi>]
             [--install-dir <path>] [--target <triple>] [--repo <owner/name>]

Environment overrides:
  VS_INSTALL_VERSION
  VS_INSTALL_VARIANT
  VS_INSTALL_DIR
  VS_INSTALL_TARGET
  VS_RELEASE_REPOSITORY

Examples:
  ./scripts/install.sh
  ./scripts/install.sh --version v0.1.0 --variant lua
  VS_INSTALL_DIR=/usr/local/bin ./scripts/install.sh
EOF
}

log() {
  printf '==> %s\n' "$*" >&2
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

cleanup() {
  if [[ -n "${TMP_DIR:-}" && -d "${TMP_DIR:-}" ]]; then
    rm -rf "$TMP_DIR"
  fi
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "required command not found: $1"
}

download_text() {
  local url=$1
  curl --fail --location --silent --show-error \
    -H "Accept: application/vnd.github+json" \
    "$url"
}

download_file() {
  local url=$1
  local output=$2
  curl --fail --location --silent --show-error \
    --output "$output" \
    "$url"
}

normalize_version() {
  local version=$1
  if [[ "$version" == "latest" ]]; then
    printf '%s\n' "$(fetch_latest_tag)"
    return
  fi
  if [[ "$version" == v* ]]; then
    printf '%s\n' "$version"
  else
    printf 'v%s\n' "$version"
  fi
}

fetch_latest_tag() {
  local metadata
  local tag

  metadata="$(download_text "https://api.github.com/repos/${REPOSITORY}/releases/latest")"
  tag="$(printf '%s\n' "$metadata" | sed -nE 's/.*"tag_name":[[:space:]]*"([^"]+)".*/\1/p' | head -n 1)"
  [[ -n "$tag" ]] || die "failed to resolve the latest release tag from GitHub"
  printf '%s\n' "$tag"
}

detect_linux_libc() {
  local ldd_output

  if command -v ldd >/dev/null 2>&1; then
    ldd_output="$(ldd --version 2>&1 || true)"
    if grep -qi 'musl' <<<"$ldd_output"; then
      printf 'musl\n'
      return
    fi
    if grep -Eqi 'glibc|gnu libc|gnu c library' <<<"$ldd_output"; then
      printf 'gnu\n'
      return
    fi
  fi

  if [[ -e /etc/alpine-release ]]; then
    printf 'musl\n'
  else
    printf 'gnu\n'
  fi
}

detect_target() {
  local os
  local arch
  local libc

  if [[ -n "$TARGET" ]]; then
    printf '%s\n' "$TARGET"
    return
  fi

  case "$(uname -s)" in
    Linux)
      os="linux"
      ;;
    Darwin)
      os="darwin"
      ;;
    *)
      die "unsupported operating system: $(uname -s)"
      ;;
  esac

  case "$(uname -m)" in
    x86_64|amd64)
      arch="x86_64"
      ;;
    i386|i686|x86)
      arch="i686"
      ;;
    arm64|aarch64)
      arch="aarch64"
      ;;
    armv7l|armv7hl|armhf)
      arch="armv7"
      ;;
    ppc64le)
      arch="powerpc64le"
      ;;
    riscv64)
      arch="riscv64gc"
      ;;
    s390x)
      arch="s390x"
      ;;
    *)
      die "unsupported architecture: $(uname -m)"
      ;;
  esac

  if [[ "$os" == "darwin" ]]; then
    case "$arch" in
      aarch64)
        printf 'aarch64-apple-darwin\n'
        ;;
      x86_64)
        printf 'x86_64-apple-darwin\n'
        ;;
      *)
        die "unsupported macOS architecture: $arch"
        ;;
    esac
    return
  fi

  libc="$(detect_linux_libc)"
  case "$arch" in
    x86_64|i686|aarch64)
      printf '%s-unknown-linux-%s\n' "$arch" "$libc"
      ;;
    armv7)
      if [[ "$libc" == "musl" ]]; then
        printf 'armv7-unknown-linux-musleabihf\n'
      else
        printf 'armv7-unknown-linux-gnueabihf\n'
      fi
      ;;
    powerpc64le|riscv64gc|s390x)
      if [[ "$libc" == "musl" ]]; then
        die "no published musl release asset for target family: $arch"
      fi
      printf '%s-unknown-linux-gnu\n' "$arch"
      ;;
    *)
      die "unsupported Linux architecture: $arch"
      ;;
  esac
}

archive_extension_for_target() {
  local target=$1
  case "$target" in
    *windows*)
      printf 'zip\n'
      ;;
    *)
      printf 'tar.gz\n'
      ;;
  esac
}

asset_name() {
  local tag=$1
  local target=$2
  local extension

  extension="$(archive_extension_for_target "$target")"
  printf 'vs-%s-%s-%s.%s\n' "$tag" "$target" "$VARIANT" "$extension"
}

install_binary() {
  local source=$1
  local destination_dir=$2
  local destination=$destination_dir/vs
  local install_prefix=()

  if ! mkdir -p "$destination_dir" 2>/dev/null; then
    if command -v sudo >/dev/null 2>&1; then
      install_prefix=(sudo)
      "${install_prefix[@]}" mkdir -p "$destination_dir"
    else
      die "cannot create install directory: $destination_dir"
    fi
  elif [[ ! -w "$destination_dir" ]]; then
    if command -v sudo >/dev/null 2>&1; then
      install_prefix=(sudo)
    else
      die "install directory is not writable: $destination_dir"
    fi
  fi

  if command -v install >/dev/null 2>&1; then
    "${install_prefix[@]}" install -m 0755 "$source" "$destination"
  else
    "${install_prefix[@]}" cp "$source" "$destination"
    "${install_prefix[@]}" chmod 0755 "$destination"
  fi
}

main() {
  local tag
  local resolved_target
  local archive
  local url
  local binary_path

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --version)
        [[ $# -ge 2 ]] || die "--version requires a value"
        REQUESTED_VERSION=$2
        shift 2
        ;;
      --variant)
        [[ $# -ge 2 ]] || die "--variant requires a value"
        VARIANT=$2
        shift 2
        ;;
      --install-dir)
        [[ $# -ge 2 ]] || die "--install-dir requires a value"
        INSTALL_DIR=$2
        shift 2
        ;;
      --target)
        [[ $# -ge 2 ]] || die "--target requires a value"
        TARGET=$2
        shift 2
        ;;
      --repo)
        [[ $# -ge 2 ]] || die "--repo requires a value"
        REPOSITORY=$2
        shift 2
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        die "unknown argument: $1"
        ;;
    esac
  done

  case "$VARIANT" in
    full|lua|wasi) ;;
    *)
      die "unsupported variant: $VARIANT (expected one of: full, lua, wasi)"
      ;;
  esac

  require_cmd curl
  require_cmd tar
  require_cmd mktemp
  require_cmd uname

  tag="$(normalize_version "$REQUESTED_VERSION")"
  resolved_target="$(detect_target)"
  archive="$(asset_name "$tag" "$resolved_target")"
  url="https://github.com/${REPOSITORY}/releases/download/${tag}/${archive}"

  case "$archive" in
    *.zip)
      die "this installer only supports Unix-like targets; resolved Windows asset: $archive"
      ;;
  esac

  TMP_DIR="$(mktemp -d)"
  log "Resolved release ${tag}"
  log "Using target ${resolved_target} (${VARIANT})"
  log "Downloading ${url}"
  download_file "$url" "$TMP_DIR/$archive"

  log "Extracting archive"
  tar -xzf "$TMP_DIR/$archive" -C "$TMP_DIR"
  binary_path="$TMP_DIR/vs"
  [[ -f "$binary_path" ]] || die "archive did not contain a top-level vs binary"

  log "Installing to ${INSTALL_DIR}"
  install_binary "$binary_path" "$INSTALL_DIR"

  log "Installed $("${INSTALL_DIR}/vs" --version 2>/dev/null || printf 'vs')"
  if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
    printf 'Add %s to your PATH if it is not already available there.\n' "$INSTALL_DIR" >&2
  fi
}

main "$@"
