#!/usr/bin/env bash
set -euo pipefail

git tag "$TAG"
git push origin "HEAD:${GITHUB_REF_NAME}"
git push origin "$TAG"
