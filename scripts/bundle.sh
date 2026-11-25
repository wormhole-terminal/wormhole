#!/usr/bin/env bash
# Builds Wormhole in release mode and assembles + signs target/release/Wormhole.app.
#
#   scripts/bundle.sh                 # host architecture only
#   scripts/bundle.sh --universal     # aarch64 + x86_64 merged with lipo
#   scripts/bundle.sh --arch x86_64   # one specific architecture
#   scripts/bundle.sh --no-build ...  # reuse existing target/<triple>/release binaries
#   scripts/bundle.sh --out DIR ...   # assemble DIR/Wormhole.app instead (a release makes three:
#                                     # universal, then each architecture from the same binaries)
#
# Signing: APPLE_SIGNING_IDENTITY (e.g. "Developer ID Application: Name (TEAMID)")
# selects a real identity from the keychain; unset = ad-hoc signature ("-"), which
# runs locally but cannot be notarized. Hardened runtime + entitlements either way.
# The bundle holds two Mach-Os: the app and the session host (Contents/MacOS/wormhole-host, built in
# the same profile and for the same architectures). The host is signed first, with the same identity,
# hardened runtime, timestamp and base entitlements, then the bundle.
# Optional: BUILD_NUMBER overrides CFBundleVersion (default: X.Y.Z of the version).
#
# Time Sensitive notifications: with a real APPLE_SIGNING_IDENTITY *and*
# APPLE_PROVISIONING_PROFILE_PATH (a Developer ID .provisionprofile for the app's App ID) the
# profile is embedded and the app is signed with the extra entitlements
# from scripts/entitlements.sh. Ad-hoc builds never carry them: a restricted entitlement
# without a profile that authorizes it makes macOS refuse to launch the app.
set -euo pipefail
. "$(dirname "$0")/lib.sh"

archs=()
build=1
app="$APP_PATH_DEFAULT"
while [ $# -gt 0 ]; do
  case "$1" in
    --universal) archs=(aarch64 x86_64) ;;
    --arch) shift; archs=("${1:?--arch needs aarch64 or x86_64}") ;;
    --no-build) build=0 ;;
    --out) shift; app="${1:?--out needs a directory}/$APP_NAME.app" ;;
    -h|--help) sed -n '2,23p' "$0"; exit 0 ;;
    *) die "unknown argument: $1" ;;
  esac
  shift
done
if [ ${#archs[@]} -eq 0 ]; then
  case "$(uname -m)" in arm64) archs=(aarch64) ;; *) archs=(x86_64) ;; esac
fi

# The binary's minimum OS must match LSMinimumSystemVersion.
export MACOSX_DEPLOYMENT_TARGET="$MIN_MACOS"

# Compiled into the app: where it looks for updates (dev builds have none, so never check)
# and, optionally, the Apple team the new version must be signed by.
export WORMHOLE_UPDATE_URL="${WORMHOLE_UPDATE_URL:-${PUBLIC_BASE_URL:-}}"
export WORMHOLE_TEAM_ID="${WORMHOLE_TEAM_ID:-${APPLE_TEAM_ID:-}}"

triples=()
for arch in "${archs[@]}"; do
  case "$arch" in aarch64|x86_64) ;; *) die "unsupported arch: $arch" ;; esac
  triples+=("$arch-apple-darwin")
done
if [ "$build" = 1 ]; then
  target_args=()
  for triple in "${triples[@]}"; do
    if command -v rustup >/dev/null 2>&1 && ! rustup target list --installed | grep -qx "$triple"; then
      die "Rust target $triple is not installed: rustup target add $triple"
    fi
    target_args+=(--target "$triple")
  done
  # One cargo for every architecture, so they build side by side and share the build scripts and
  # proc macros; two cargos would take turns on the target dir's lock.
  log "cargo build --release -p wormhole -p wormhole-host ${target_args[*]}"
  (cd "$ROOT" && cargo build --release --locked -p wormhole -p wormhole-host "${target_args[@]}")
fi

bins=()
host_bins=()
for triple in "${triples[@]}"; do
  bin="$TARGET_DIR/$triple/release/$APP_BINARY"
  [ -x "$bin" ] || die "missing binary $bin (drop --no-build?)"
  bins+=("$bin")
  host_bin="$TARGET_DIR/$triple/release/$HOST_BINARY"
  [ -x "$host_bin" ] || die "missing binary $host_bin (drop --no-build?)"
  host_bins+=("$host_bin")
done

mkdir -p "$(dirname "$app")"
log "assembling $app (version $APP_VERSION, ${archs[*]})"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"

if [ ${#bins[@]} -gt 1 ]; then
  lipo -create "${bins[@]}" -output "$app/Contents/MacOS/$APP_BINARY"
else
  cp "${bins[0]}" "$app/Contents/MacOS/$APP_BINARY"
fi
chmod 755 "$app/Contents/MacOS/$APP_BINARY"
# The host in the same shape as the app: universal when the app is.
if [ ${#host_bins[@]} -gt 1 ]; then
  lipo -create "${host_bins[@]}" -output "$app/Contents/MacOS/$HOST_BINARY"
else
  cp "${host_bins[0]}" "$app/Contents/MacOS/$HOST_BINARY"
fi
chmod 755 "$app/Contents/MacOS/$HOST_BINARY"
# Assets.car holds the icon (CFBundleIconName); the .icns is the CFBundleIconFile fallback.
cp "$PACKAGING/Assets.car" "$app/Contents/Resources/Assets.car"
cp "$PACKAGING/AppIcon.icns" "$app/Contents/Resources/AppIcon.icns"
# The bundled font is compiled into the binary; its license (OFL) travels with it.
mkdir -p "$app/Contents/Resources/Licenses"
cp "$ROOT/crates/wormhole-core/assets/fonts/JetBrainsMono/OFL.txt" \
  "$app/Contents/Resources/Licenses/JetBrainsMono-OFL.txt"
printf 'APPL????' > "$app/Contents/PkgInfo"

render_info_plist "$app/Contents/Info.plist"

profile="${APPLE_PROVISIONING_PROFILE_PATH:-}"
entitlements="$PACKAGING/entitlements.plist"
select_signing_identity
if [ "$SIGN_IDENTITY" != "-" ]; then
  if [ -n "$profile" ]; then
    # The profile is sealed into the signature, so it goes in before codesign runs.
    [ -f "$profile" ] || die "APPLE_PROVISIONING_PROFILE_PATH is set but '$profile' is not a file"
    entitlements="$TARGET_DIR/release/entitlements.signed.plist"
    log "embedding $profile: adding the Time Sensitive Notifications entitlement"
    "$ROOT/scripts/entitlements.sh" --profile "$profile" --out "$entitlements"
    cp "$profile" "$app/Contents/embedded.provisionprofile"
  else
    log "no APPLE_PROVISIONING_PROFILE_PATH: 'needs you' notifications are delivered at Active"
  fi
else
  [ -z "$profile" ] || warn "ignoring APPLE_PROVISIONING_PROFILE_PATH: only Developer ID builds carry its entitlement"
fi
# The restricted Time Sensitive entitlement stays with the app alone: the embedded profile authorizes the main
# executable, and a nested one claiming it would not launch.
sign_host "$app"
sign_app "$app" "$entitlements"
"$ROOT/scripts/check-entitlements.sh" "$app"

log "done: $app"
lipo -archs "$app/Contents/MacOS/$APP_BINARY" >&2
lipo -archs "$app/Contents/MacOS/$HOST_BINARY" >&2
