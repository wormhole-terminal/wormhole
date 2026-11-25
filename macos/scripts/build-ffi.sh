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
# `WORMHOLE_FFI_PERF_COUNTERS=1` builds the library with its `perf-counters` feature, so the app's `perf-profile` can
# say what reading output cost (the reader's bytes and the screens' lock): a library made to measure. Off by default: the
# shipped library does not count.
#
# `--shipped` builds the library that goes into the beta's app (`macos/scripts/bundle.sh` asks for it) instead of the dev one,
# into `macos/dist/shipped/` and linked into the packages listed, never into ffi-bench:
#   - for both architectures, joined with `lipo` (uniffi-bindgen reads the universal library: its bindings are byte for byte the
#     ones of either slice);
#   - `--no-default-features`, so no `dev-key`: the sealed stores keep their keys in the Keychain and the library opens the
#     beta's data folder (it refuses it with `dev-key`, `crates/wormhole-ffi/src/isolation.rs`);
#   - `--locked`, and the paths of this Mac (the checkout, ~/.cargo, ~/.rustup) remapped in the library, whose panic messages
#     and log callsites would otherwise carry them into a public app;
#   - what the app bakes in comes from the environment, which cargo reads when it compiles: `WORMHOLE_UPDATE_URL`,
#     `WORMHOLE_UPDATE_CHANNEL` and `WORMHOLE_TEAM_ID` (unset: none, as in the dev library).
set -euo pipefail

macos="$(cd "$(dirname "$0")/.." && pwd)"
# The bridge crate and its bindings generator: a cargo workspace of its own, beside the app's crates.
bridge="$(cd "$macos/../crates/wormhole-ffi" && pwd)"
cd "$macos"

# The same deployment target for cargo's objects and for Swift.
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-26.0}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"

shipped=0
if [ "${1:-}" = "--shipped" ]; then shipped=1; shift; fi

# Cargo's output may live elsewhere (a target dir shared by worktrees); what is generated per worktree
# (bindings, headers) stays in the bridge's own `target/`. A relative CARGO_TARGET_DIR is cargo's, read from
# the bridge's folder.
built="${CARGO_TARGET_DIR:-target}"
case "$built" in /*) ;; *) built="$bridge/$built" ;; esac
dist="dist"

# Two invocations: the library alone has no generator in it (smaller, and cargo's feature
# unification would otherwise turn `uniffi/cli` on for it); the generator is a package of its own.
if [ "$shipped" = 1 ]; then
  if [ "${WORMHOLE_FFI_PERF_COUNTERS:-0}" = "1" ]; then
    echo "error: the shipped library does not count (WORMHOLE_FFI_PERF_COUNTERS)" >&2
    exit 1
  fi
  dist="dist/shipped"
  triples=(aarch64-apple-darwin x86_64-apple-darwin)
  # The paths of this machine stay out of what ships: the library's panic messages and log callsites would carry them (the
  # checkout, ~/.cargo, ~/.rustup) into every user's copy. Cargo's own `trim-paths` is not stable yet. bundle.sh trims the host
  # the same way.
  repo="$(cd "$macos/.." && pwd)"
  trim="--remap-path-prefix=$repo=/wormhole --remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo --remap-path-prefix=${RUSTUP_HOME:-$HOME/.rustup}=/rustup"
  # One cargo for both architectures: they build side by side and share the build scripts and proc macros. The generator is built
  # for this machine and untrimmed: it is a tool and is not shipped.
  (cd "$bridge" && RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }$trim" cargo build --release --locked -p wormhole-ffi --no-default-features \
    --target "${triples[0]}" --target "${triples[1]}" && cargo build --release --locked -p uniffi-bindgen)
  lib="$built/shipped/libwormhole_ffi.a"
  mkdir -p "$built/shipped"
  lipo -create "$built/${triples[0]}/release/libwormhole_ffi.a" "$built/${triples[1]}/release/libwormhole_ffi.a" -output "$lib"
else
  features=()
  if [ "${WORMHOLE_FFI_PERF_COUNTERS:-0}" = "1" ]; then features=(--features perf-counters); fi
  (cd "$bridge" && cargo build --release -p wormhole-ffi ${features[@]+"${features[@]}"} && cargo build --release -p uniffi-bindgen)
  lib="$built/release/libwormhole_ffi.a"
fi

bindings="$bridge/target/bindings"
headers="$bridge/target/xcframework-headers"
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
links=("$@")
if [ "$shipped" = 0 ]; then links=(ffi-bench "$@"); fi
for package in ${links[@]+"${links[@]}"}; do
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
