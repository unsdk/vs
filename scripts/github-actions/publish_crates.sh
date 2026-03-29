#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${CARGO_REGISTRY_TOKEN:-}" && -n "${CARGO_REGISTRY_TOKEN_FALLBACK:-}" ]]; then
  export CARGO_REGISTRY_TOKEN="$CARGO_REGISTRY_TOKEN_FALLBACK"
fi

if [[ -z "${CARGO_REGISTRY_TOKEN:-}" ]]; then
  echo "CARGO_REGISTRY_TOKEN is not configured; skipping crates.io publish."
  exit 0
fi

publish_with_retry() {
  local crate="$1"
  local max_attempts=10
  local attempt=1

  while [[ "$attempt" -le "$max_attempts" ]]; do
    echo "Publishing $crate (attempt $attempt/$max_attempts)..."
    set +e
    local output
    output="$(cargo publish -p "$crate" --locked 2>&1)"
    local status=$?
    set -e
    echo "$output"

    if [[ "$status" -eq 0 ]]; then
      return 0
    fi

    if grep -qiE "already uploaded|already exists|has already been uploaded" <<<"$output"; then
      echo "$crate is already published, skipping."
      return 0
    fi

    if [[ "$attempt" -eq "$max_attempts" ]]; then
      echo "Publishing $crate failed after $max_attempts attempts."
      return "$status"
    fi

    echo "Retrying $crate in 10 minutes..."
    sleep 600
    attempt=$((attempt + 1))
  done
}

crates=(
  vs-plugin-api
  vs-shell
  vs-config
  vs-plugin-sdk
  vs-registry
  vs-installer
  vs-plugin-lua
  vs-plugin-wasi
  vs-test-support
  vs-core
  vs-cli
)

for crate in "${crates[@]}"; do
  publish_with_retry "$crate"
done
