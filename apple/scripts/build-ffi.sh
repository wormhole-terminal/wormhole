#!/usr/bin/env bash
# Build the Rust side of the Swift UI experiment and wrap it for SwiftPM:
#   1. cargo builds `wormhole-ffi` (release, one arch) into apple/target;
#   2. uniffi-bindgen (the package next to it, at the library's version) reads the library and writes
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

# Two invocations: the library alone has no generator in it (smaller, and cargo's feature
# unification would otherwise turn `uniffi/cli` on for it); the generator is a package of its own.
cargo build --release -p wormhole-ffi
cargo build --release -p uniffi-bindgen

# Cargo's output may live elsewhere (a target dir shared by worktrees); what is generated per worktree
# (bindings, headers) stays in this one's own `target/`.
built="${CARGO_TARGET_DIR:-target}"
lib="$built/release/libwormhole_ffi.a"
bindings="target/bindings"
headers="target/xcframework-headers"
dist="dist"
rm -rf "$bindings" "$headers" "$dist/WormholeFFI.xcframework"
mkdir -p "$bindings" "$headers" "$dist"

"$built/release/uniffi-bindgen" generate --library "$lib" --language swift --out-dir "$bindings"

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
  # The intro's shader is the GPUI app's own, one source for both. It is copied in, not linked: SwiftPM does not
  # follow a link to a folder outside the package when it gathers resources (Swift 6.3 left the bundle without it).
  if [ -d "$package/Sources/AuxWindows" ]; then
    shader="$package/Sources/AuxWindows/Resources/shader"
    rm -rf "$shader"
    mkdir -p "$shader"
    cp "$apple/../crates/wormhole/src/platform/macos/intro/shader/"*.metal "$shader/"
  fi
  echo "linked into $package"
done
