#!/usr/bin/env bash
set -euxo pipefail

build_tool="${BUILD_TOOL:-cargo}"

if [[ "$build_tool" == "cross" ]]; then
  cross build -p vs-cli --profile "$BUILD_PROFILE" --locked --no-default-features --features "$FEATURE_FLAGS" --target "$BUILD_TARGET"
elif [[ -n "${BUILD_TARGET:-}" ]]; then
  cargo build -p vs-cli --profile "$BUILD_PROFILE" --locked --no-default-features --features "$FEATURE_FLAGS" --target "$BUILD_TARGET"
else
  cargo build -p vs-cli --profile "$BUILD_PROFILE" --locked --no-default-features --features "$FEATURE_FLAGS"
fi
