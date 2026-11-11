#!/usr/bin/env bash
# Build the Rust side of the Swift UI experiment and wrap it for SwiftPM:
#   1. cargo builds `wormhole-ffi` (crates/wormhole-ffi; release, one arch) into its own target dir;
#   2. uniffi-bindgen (the package next to it, at the library's version) reads the library and writes
#      the Swift bindings and the C header and module map;
#   3. the static library and the header become `WormholeFFI.xcframework`.
# Both results land in `macos/dist/` (git-ignored): `WormholeFFI.xcframework` and
# `WormholeBindings.swift`, plus `ffi-bench/` and any other package that lists a destination
# (`scripts/build-ffi.sh <package dir>...` also links them into those: see FFI-CONTRACT.md).
# Run from anywhere; `CARGO_BUILD_JOBS` defaults to 4.
set -euo pipefail

macos="$(cd "$(dirname "$0")/.." && pwd)"
# The bridge crate and its bindings generator: a cargo workspace of its own, beside the app's crates.
bridge="$(cd "$macos/../crates/wormhole-ffi" && pwd)"
cd "$macos"

# The same deployment target for cargo's objects and for Swift.
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-26.0}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"

# Two invocations: the library alone has no generator in it (smaller, and cargo's feature
# unification would otherwise turn `uniffi/cli` on for it); the generator is a package of its own.
(cd "$bridge" && cargo build --release -p wormhole-ffi && cargo build --release -p uniffi-bindgen)

# Cargo's output may live elsewhere (a target dir shared by worktrees); what is generated per worktree
# (bindings, headers) stays in the bridge's own `target/`. A relative CARGO_TARGET_DIR is cargo's, read from
# the bridge's folder.
built="${CARGO_TARGET_DIR:-target}"
case "$built" in /*) ;; *) built="$bridge/$built" ;; esac
lib="$built/release/libwormhole_ffi.a"
bindings="$bridge/target/bindings"
headers="$bridge/target/xcframework-headers"
dist="dist"
rm -rf "$bindings" "$headers" "$dist/WormholeFFI.xcframework"
mkdir -p "$bindings" "$headers" "$dist"

# From the bridge's folder: the generator asks cargo there for the crate's `uniffi.toml`, which names the module
# and the header; anywhere else it finds none and writes them under its default names.
(cd "$bridge" && "$built/release/uniffi-bindgen" generate --library "$lib" --language swift --out-dir "$bindings")

cp "$bindings/WormholeFFI.h" "$headers/"
# SwiftPM finds a binary target's module through a file named exactly `module.modulemap`.
cp "$bindings/WormholeFFI.modulemap" "$headers/module.modulemap"
cp "$bindings/WormholeBindings.swift" "$dist/WormholeBindings.swift"
xcodebuild -create-xcframework -library "$lib" -headers "$headers" -output "$dist/WormholeFFI.xcframework"
echo "built $lib ($(du -h "$lib" | cut -f1)) -> $macos/$dist/"

# The packages that consume it get symlinks (SwiftPM wants a binary target and sources inside the
# package's own folder): `<package>/WormholeFFI.xcframework` and `<package>/Sources/WormholeBindings/`.
for package in ffi-bench "$@"; do
  package="$(cd "$package" && pwd)"
  mkdir -p "$package/Sources/WormholeBindings"
  ln -sfn "$macos/$dist/WormholeFFI.xcframework" "$package/WormholeFFI.xcframework"
  ln -sf "$macos/$dist/WormholeBindings.swift" "$package/Sources/WormholeBindings/WormholeBindings.swift"
  # The intro's shader is the GPUI app's own, one source for both. It is copied in, not linked: SwiftPM does not
  # follow a link to a folder outside the package when it gathers resources (Swift 6.3 left the bundle without it).
  if [ -d "$package/Sources/AuxWindows" ]; then
    shader="$package/Sources/AuxWindows/Resources/shader"
    rm -rf "$shader"
    mkdir -p "$shader"
    cp "$macos/../crates/wormhole/src/platform/macos/intro/shader/"*.metal "$shader/"
  fi
  echo "linked into $package"
done
