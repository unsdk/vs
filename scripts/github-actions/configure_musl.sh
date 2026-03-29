#!/usr/bin/env bash
set -euo pipefail

{
  echo "CC_x86_64_unknown_linux_musl=musl-gcc"
  echo "CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc"
} >>"$GITHUB_ENV"
