#!/usr/bin/env bash
#
# Stamp a single version across every binding so releases stay in sync.
#
# Updates the [package] version of each crate, the Python project version, and
# the Swift manifest's releaseVersion. Used by the release workflow (and safe to
# run by hand). The release *checksum* in Package.swift is handled separately by
# the release flow, not here.
#
#   scripts/set-version.sh 0.2.0
set -euo pipefail

VERSION="${1:?usage: set-version.sh <version>   e.g. set-version.sh 0.2.0}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# `sed -i.bak` is the portable form that works on both GNU (Linux CI) and BSD
# (macOS) sed. The `^version` anchor matches only the [package]/[project]
# version line, never dependency `version = ...` entries inside tables.
stamp_toml() {
  local file="$1"
  sed -i.bak -E "s/^version = \".*\"/version = \"${VERSION}\"/" "${file}"
  rm -f "${file}.bak"
}

for crate in crates/core bindings/wasm bindings/python bindings/swift/ffi; do
  stamp_toml "${ROOT}/${crate}/Cargo.toml"
done
stamp_toml "${ROOT}/bindings/python/pyproject.toml"

# Swift manifest: the release version constant.
sed -i.bak -E "s/(let releaseVersion = \")[^\"]*(\")/\1${VERSION}\2/" "${ROOT}/Package.swift"
rm -f "${ROOT}/Package.swift.bak"

echo "Set version to ${VERSION}"
