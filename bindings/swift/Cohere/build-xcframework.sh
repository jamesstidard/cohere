#!/usr/bin/env bash
#
# Build Cohere.xcframework from the cohere-swift staticlib.
#
# Produces slices for whichever Apple platforms have their Rust target
# installed: macOS, iOS device, and iOS simulator. Missing targets are skipped
# with a warning (run `rustup target add <triple>` to include them).
#
# Usage:
#   ./swift/build-xcframework.sh          # release build
#   PROFILE=debug ./swift/build-xcframework.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Script lives at bindings/swift/Cohere; the workspace root is three levels up
# and the Rust FFI crate is the sibling ffi/ directory.
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
CRATE_DIR="${SCRIPT_DIR}/../ffi"
LIB_NAME="libcohere_swift.a"
HEADERS_DIR="${CRATE_DIR}/include"
PROFILE="${PROFILE:-release}"
OUT="${SCRIPT_DIR}/Cohere.xcframework"

BUILD_DIR="${SCRIPT_DIR}/.build-xcframework"
rm -rf "${BUILD_DIR}" "${OUT}"
mkdir -p "${BUILD_DIR}"

CARGO_PROFILE_FLAG=""
PROFILE_DIR="debug"
if [ "${PROFILE}" = "release" ]; then
  CARGO_PROFILE_FLAG="--release"
  PROFILE_DIR="release"
fi

# Build the staticlib for one Rust target triple. Returns the .a path, or empty
# if the target is not installed and cannot be added.
build_target() {
  local triple="$1"
  if ! rustup target list --installed | grep -qx "${triple}"; then
    echo "  target ${triple} not installed; attempting 'rustup target add'..." >&2
    if ! rustup target add "${triple}" >/dev/null 2>&1; then
      echo "  WARNING: could not install ${triple}; skipping this slice." >&2
      return 1
    fi
  fi
  echo "  building ${triple}..." >&2
  ( cd "${REPO_ROOT}" && cargo build -p cohere-swift ${CARGO_PROFILE_FLAG} --target "${triple}" >&2 )
  echo "${REPO_ROOT}/target/${triple}/${PROFILE_DIR}/${LIB_NAME}"
}

# lipo together the slices that built successfully into one fat archive.
# Args: <output-name> <triple>... ; prints the fat lib path or empty.
make_slice() {
  local name="$1"; shift
  local libs=()
  for triple in "$@"; do
    local lib
    if lib="$(build_target "${triple}")"; then
      libs+=("${lib}")
    fi
  done
  if [ "${#libs[@]}" -eq 0 ]; then
    return 1
  fi
  # xcframework requires static libraries to be lib-prefixed.
  local out="${BUILD_DIR}/lib${name}.a"
  if [ "${#libs[@]}" -eq 1 ]; then
    cp "${libs[0]}" "${out}"
  else
    lipo -create "${libs[@]}" -output "${out}"
  fi
  echo "${out}"
}

echo "Building Cohere.xcframework (profile: ${PROFILE})..."

XCFRAMEWORK_ARGS=()

echo "macOS slice:"
if macos_lib="$(make_slice macos aarch64-apple-darwin x86_64-apple-darwin)"; then
  XCFRAMEWORK_ARGS+=(-library "${macos_lib}" -headers "${HEADERS_DIR}")
fi

echo "iOS device slice:"
if ios_lib="$(make_slice ios aarch64-apple-ios)"; then
  XCFRAMEWORK_ARGS+=(-library "${ios_lib}" -headers "${HEADERS_DIR}")
fi

echo "iOS simulator slice:"
if ios_sim_lib="$(make_slice ios-sim aarch64-apple-ios-sim x86_64-apple-ios)"; then
  XCFRAMEWORK_ARGS+=(-library "${ios_sim_lib}" -headers "${HEADERS_DIR}")
fi

if [ "${#XCFRAMEWORK_ARGS[@]}" -eq 0 ]; then
  echo "ERROR: no platform slices were built." >&2
  exit 1
fi

xcodebuild -create-xcframework "${XCFRAMEWORK_ARGS[@]}" -output "${OUT}"
rm -rf "${BUILD_DIR}"
echo "Created ${OUT}"
echo
echo "To build/test the Swift package against this local framework, set:"
echo "  export COHERE_LOCAL_XCFRAMEWORK=1"
echo "then run e.g. 'swift test' from the repo root."
