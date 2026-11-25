#!/usr/bin/env bash
# Builds "Wormhole Beta.app", the Swift app's public beta, from this checkout: universal (arm64 + x86_64), signed, and
# complete on its own (nothing in it points back at the repository or the build folder). `scripts/bundle.sh` is the
# shipped 1.x app's; this is the beta's, which installs beside it (packaging/beta.env is its identity).
#
#   macos/scripts/bundle.sh                 # build everything, assemble macos/dist/"Wormhole Beta.app", sign it
#   macos/scripts/bundle.sh --no-build      # assemble and sign again from what the last run built
#   macos/scripts/bundle.sh --out DIR       # DIR/"Wormhole Beta.app" instead of macos/dist/
#
# What goes in, from the environment:
#   APPLE_SIGNING_IDENTITY   "Developer ID Application: Name (TEAMID)": a real identity from the keychain, with the hardened
#                            runtime and a secure timestamp. Unset: an ad-hoc signature, which runs here but cannot be notarized.
#   WORMHOLE_TEAM_ID         (else APPLE_TEAM_ID) the Apple team a new version must be signed by; baked into the library.
#   WORMHOLE_UPDATE_URL, WORMHOLE_UPDATE_CHANNEL   where the app looks for updates; default packaging/beta.env's
#                            UPDATE_BASE_URL and UPDATE_CHANNEL; baked into the library.
#   BUILD_NUMBER             CFBundleVersion; default the version's X.Y.Z.
# The version is the bridge's (VERSION_MANIFEST in beta.env): CFBundleShortVersionString is X.Y.Z, and WormholeVersion, which the
# updater reads, is the whole of it ("2.0.0-beta.1"). A provisioning profile is not used: the beta's App ID has none.
#
# What a bundle is made of, and why:
#   - the library is the shipped one (`build-ffi.sh --shipped`: both architectures, no dev-key, update settings baked in);
#   - the app is a release build of the `Wormhole` product by `swift build --build-system swiftbuild`, whose generated resource
#     accessor looks in the app's `Contents/Resources` first. SwiftPM's older build system looks beside the executable and then at
#     an absolute path into the build folder, which is on no one's Mac but the builder's, so it is not used (checked below);
#   - the session host is built from the root workspace for both architectures and sits beside the app's executable, where
#     the core looks for it; it is signed first, under the identifier "<bundle id>.host" that it derives its app from;
#   - the executable embeds the same Info.plist the bundle has (Package.swift: WORMHOLE_INFO_PLIST);
#   - the fonts and their licence are in Resources/Fonts/JetBrainsMono, where the app looks first.
# `macos/scripts/check-bundle.sh` checks the result, and runs a copy of it.
# One build at a time on a Mac that other builds share; this runs three (the library, the host, the app).
set -euo pipefail
# Before lib.sh reads it: the beta's scripts run with the beta's identity, whatever the caller has set.
export WORMHOLE_APP_ENV=packaging/beta.env
. "$(dirname "$0")/../../scripts/lib.sh"

macos="$ROOT/macos"
package="$macos/swift"
dist="$macos/dist"
work="$dist/bundle-work"
build=1
out="$dist"
while [ $# -gt 0 ]; do
  case "$1" in
    --no-build) build=0 ;;
    --out) shift; out="${1:?--out needs a directory}" ;;
    -h|--help) sed -n '2,30p' "$0"; exit 0 ;;
    *) die "unknown argument: $1" ;;
  esac
  shift
done
app="$out/$APP_NAME.app"
triples=(aarch64-apple-darwin x86_64-apple-darwin)
fonts_source="$ROOT/crates/wormhole-core/assets/fonts/JetBrainsMono"
for tool in cargo swift lipo codesign xcodebuild plutil strip strings; do need "$tool"; done

# --- what the library and the app bake in ---------------------------------------------------------------------------
export WORMHOLE_UPDATE_URL="${WORMHOLE_UPDATE_URL:-$UPDATE_BASE_URL}"
export WORMHOLE_UPDATE_CHANNEL="${WORMHOLE_UPDATE_CHANNEL:-$UPDATE_CHANNEL}"
team="${WORMHOLE_TEAM_ID:-${APPLE_TEAM_ID:-}}"
# Set and empty is a team of no name to a build that reads it with option_env!.
if [ -n "$team" ]; then export WORMHOLE_TEAM_ID="$team"; else unset WORMHOLE_TEAM_ID; fi
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
# The binary's minimum OS must match LSMinimumSystemVersion.
export MACOSX_DEPLOYMENT_TARGET="$MIN_MACOS"

# --- the Info.plist, rendered first because the executable embeds it ------------------------------------------------------
mkdir -p "$work"
plist="$work/Info.plist"
render_info_plist "$plist"
# The updater compares whole versions, which CFBundleShortVersionString cannot hold (it has no prerelease part).
plutil -insert WormholeVersion -string "$APP_VERSION" "$plist"
plutil -lint -s "$plist"
# Read by Package.swift whenever it is evaluated, so every `swift` call below carries it.
export WORMHOLE_INFO_PLIST="$plist"

# --- the three builds -------------------------------------------------------------------------------------------------
# The package is linked to the shipped library for the build and to what it was linked to before (the dev library, for the
# dev loop that shares this checkout) when this ends, however it ends.
link_library="$package/WormholeFFI.xcframework"
link_bindings="$package/Sources/WormholeBindings/WormholeBindings.swift"
restore_link() { if [ -n "$2" ]; then ln -sfn "$2" "$1"; else rm -f "$1"; fi; }
previous_library="$(readlink "$link_library" || true)"
previous_bindings="$(readlink "$link_bindings" || true)"
restore_links() { restore_link "$link_library" "$previous_library"; restore_link "$link_bindings" "$previous_bindings"; }

# The app's flags, for the build and for asking where its products went: swiftbuild is named, whatever this toolchain's
# default is, because the accessor it generates is the one the bundle needs.
swift_args=(--package-path "$package" --build-system swiftbuild -c release --arch arm64 --arch x86_64)

if [ "$build" = 1 ]; then
  for triple in "${triples[@]}"; do
    if command -v rustup >/dev/null 2>&1 && ! (cd "$ROOT" && rustup target list --installed) | grep -qx "$triple"; then
      die "Rust target $triple is not installed: rustup target add $triple"
    fi
  done
  trap restore_links EXIT
  log "the library: build-ffi.sh --shipped"
  "$macos/scripts/build-ffi.sh" --shipped "$package"

  # One cargo for both architectures, as in scripts/bundle.sh. The paths of this Mac are remapped, as build-ffi.sh does for the
  # library: the host's panic messages would carry them (cargo's own `trim-paths` is not stable yet).
  log "the session host: cargo build --release -p wormhole-host (${triples[*]})"
  trim="--remap-path-prefix=$ROOT=/wormhole --remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo --remap-path-prefix=${RUSTUP_HOME:-$HOME/.rustup}=/rustup"
  (cd "$ROOT" && RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }$trim" cargo build --release --locked -p wormhole-host \
    --target "${triples[0]}" --target "${triples[1]}")

  log "the app: swift build ${swift_args[*]:2}"
  swift build "${swift_args[@]}" --product "$APP_BINARY"
fi

# --- the pieces -------------------------------------------------------------------------------------------------------
products="$(swift build "${swift_args[@]}" --show-bin-path)"
executable="$products/$APP_BINARY"
[ -x "$executable" ] || die "no $executable (drop --no-build?)"
host_parts=()
for triple in "${triples[@]}"; do
  [ -x "$TARGET_DIR/$triple/release/$HOST_BINARY" ] || die "no $TARGET_DIR/$triple/release/$HOST_BINARY (drop --no-build?)"
  host_parts+=("$TARGET_DIR/$triple/release/$HOST_BINARY")
done
# The resource bundles are named after the package (`Wormhole_DesignSystem.bundle`). A products folder that has seen an older
# build also holds the ones of its time (`WormholeMetalSpike_*`, under the package's earlier name): only this package's go in.
package_name="$(swift package --package-path "$package" describe | sed -n 's/^Name: //p' | head -n 1)"
[ -n "$package_name" ] || die "cannot read the package's name from $package"
resource_bundles=()
for resources in "$products/${package_name}_"*.bundle; do
  [ -d "$resources" ] && resource_bundles+=("$resources")
done
[ ${#resource_bundles[@]} -gt 0 ] || die "$products holds no ${package_name}_*.bundle: the themes, keymap, icons and shaders are missing"

# --- the bundle -------------------------------------------------------------------------------------------------------
log "assembling $app (version $APP_VERSION)"
mkdir -p "$out"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources/Fonts/JetBrainsMono"
cp "$executable" "$app/Contents/MacOS/$APP_BINARY"
lipo -create "${host_parts[@]}" -output "$app/Contents/MacOS/$HOST_BINARY"
# The debugging symbols are the builder's (their object files' paths, in the executable's symbol table, are folders of this Mac).
strip -S "$app/Contents/MacOS/$APP_BINARY" "$app/Contents/MacOS/$HOST_BINARY"
chmod 755 "$app/Contents/MacOS/$APP_BINARY" "$app/Contents/MacOS/$HOST_BINARY"
# Resource bundles belong in Contents/Resources: codesign refuses loose content at the bundle's root.
for resources in "${resource_bundles[@]}"; do cp -R "$resources" "$app/Contents/Resources/"; done
cp "$PACKAGING/Assets.car" "$PACKAGING/AppIcon.icns" "$app/Contents/Resources/"
for face in Regular Bold Italic BoldItalic; do
  cp "$fonts_source/JetBrainsMono-$face.ttf" "$app/Contents/Resources/Fonts/JetBrainsMono/"
done
cp "$fonts_source/OFL.txt" "$app/Contents/Resources/Fonts/JetBrainsMono/"
printf 'APPL????' > "$app/Contents/PkgInfo"
cp "$plist" "$app/Contents/Info.plist"

# --- what must not be in it --------------------------------------------------------------------------------------------
# A path into a folder of this Mac in an executable is what a resource accessor of the older build system, a panic message or
# a debug map leaves behind: it works for its builder and for no one else. The checkout holds the build folders; the home folder
# holds cargo's and rustup's. (A bare "/Users/" is no test: the app's own sample data and tests of home-folder shortening spell it.)
# The one path left is the host's: its hidden `--built-from` (crates/wormhole-host/src/lib.rs) prints `env!("CARGO_MANIFEST_DIR")`,
# which no path remapping reaches. It is dropped here by name, so anything else of this Mac in the host still fails the build.
paths_of_this_mac() {
  strings -a "$1" | awk -v own="$ROOT/crates/wormhole-host" '{ while ((i = index($0, own)) > 0) $0 = substr($0, 1, i - 1) substr($0, i + length(own)); print }' \
    | grep -F -e "$ROOT" -e "$HOME/" -e "${CARGO_HOME:-$HOME/.cargo}" -e "${RUSTUP_HOME:-$HOME/.rustup}" | sort -u | head -n 5 | cut -c1-200 || true
}
for binary in "$app/Contents/MacOS/$APP_BINARY" "$app/Contents/MacOS/$HOST_BINARY"; do
  hits="$(paths_of_this_mac "$binary")"
  [ -z "$hits" ] || die "$binary names folders of this Mac:"$'\n'"$hits"
done
for resources in "${resource_bundles[@]}"; do
  [ -d "$app/Contents/Resources/$(basename "$resources")" ] || die "$(basename "$resources") is not in Contents/Resources"
done

# --- signing -----------------------------------------------------------------------------------------------------------
[ -z "${APPLE_PROVISIONING_PROFILE_PATH:-}" ] || warn "ignoring APPLE_PROVISIONING_PROFILE_PATH: the beta's App ID has no profile"
select_signing_identity
sign_host "$app"
sign_app "$app" "$PACKAGING/entitlements.plist"
"$ROOT/scripts/check-entitlements.sh" "$app"

log "done: $app ($(du -sh "$app" | cut -f1), version $APP_VERSION)"
lipo -archs "$app/Contents/MacOS/$APP_BINARY" >&2
lipo -archs "$app/Contents/MacOS/$HOST_BINARY" >&2
