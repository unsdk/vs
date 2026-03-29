#!/usr/bin/env bash
set -euo pipefail

version="${INPUT_VERSION#v}"

{
  echo "version=$version"
  echo "tag=v$version"
} >>"$GITHUB_OUTPUT"
