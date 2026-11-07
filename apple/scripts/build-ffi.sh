#!/usr/bin/env bash
# Build the Rust side of the Swift UI experiment and wrap it for SwiftPM:
#   1. cargo builds `wormhole-ffi` (release, one arch) into apple/target;
#   2. uniffi-bindgen (this crate's own binary, at the library's version) reads the library and writes
#      the Swift bindings and the C header and module map;
#   3. the static library and the header become `WormholeFFI.xcframework`.
# Both results land in `apple/dist/` (git-ignored): `WormholeFFI.xcframework` and
# `WormholeBindings.swift`, plus `ffi-bench/` and any other package that lists a destination
# (`scripts/build-ffi.sh <package dir>...` also links them into those: see FFI-CONTRACT.md).
# Run from anywhere; `CARGO_BUILD_JOBS` defaults to 4.
set -euo pipefail

apple="$(cd "$(dirname "$0")/.." && pwd)"
cd "$apple"

# The same deployment target for cargo's objects and for Swift.
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-26.0}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"

cargo build --release -p wormhole-ffi

lib="target/release/libwormhole_ffi.a"
bindings="target/bindings"
headers="target/xcframework-headers"
dist="dist"
rm -rf "$bindings" "$headers" "$dist/WormholeFFI.xcframework"
mkdir -p "$bindings" "$headers" "$dist"

target/release/uniffi-bindgen generate --library "$lib" --language swift --out-dir "$bindings"

cp "$bindings/WormholeFFI.h" "$headers/"
# SwiftPM finds a binary target's module through a file named exactly `module.modulemap`.
cp "$bindings/WormholeFFI.modulemap" "$headers/module.modulemap"
cp "$bindings/WormholeBindings.swift" "$dist/WormholeBindings.swift"
xcodebuild -create-xcframework -library "$lib" -headers "$headers" -output "$dist/WormholeFFI.xcframework"
echo "built $lib ($(du -h "$lib" | cut -f1)) -> $apple/$dist/"

# The packages that consume it get symlinks (SwiftPM wants a binary target and sources inside the
# package's own folder): `<package>/WormholeFFI.xcframework` and `<package>/Sources/WormholeBindings/`.
for package in ffi-bench "$@"; do
  package="$(cd "$package" && pwd)"
  mkdir -p "$package/Sources/WormholeBindings"
  ln -sfn "$apple/$dist/WormholeFFI.xcframework" "$package/WormholeFFI.xcframework"
  ln -sf "$apple/$dist/WormholeBindings.swift" "$package/Sources/WormholeBindings/WormholeBindings.swift"
  echo "linked into $package"
done
