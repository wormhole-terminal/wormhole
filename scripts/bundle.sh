#!/usr/bin/env bash
# Builds Wormhole in release mode and assembles + signs target/release/Wormhole.app.
#
#   scripts/bundle.sh                 # host architecture only
#   scripts/bundle.sh --universal     # aarch64 + x86_64 merged with lipo
#   scripts/bundle.sh --arch x86_64   # one specific architecture
#   scripts/bundle.sh --no-build ...  # reuse existing target/<triple>/release binaries
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
while [ $# -gt 0 ]; do
  case "$1" in
    --universal) archs=(aarch64 x86_64) ;;
    --arch) shift; archs=("${1:?--arch needs aarch64 or x86_64}") ;;
    --no-build) build=0 ;;
    -h|--help) sed -n '2,21p' "$0"; exit 0 ;;
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

app="$APP_PATH_DEFAULT"
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

# CFBundleShortVersionString must be 1-3 integers: drop any "-beta.1" suffix.
short_version="${APP_VERSION%%[-+]*}"
build_version="${BUILD_NUMBER:-$short_version}"
sed_escape() { printf '%s' "$1" | sed -e 's/[\\|&]/\\&/g'; }
sed \
  -e "s|@APP_NAME@|$(sed_escape "$APP_NAME")|g" \
  -e "s|@APP_BINARY@|$(sed_escape "$APP_BINARY")|g" \
  -e "s|@BUNDLE_ID@|$(sed_escape "$BUNDLE_ID")|g" \
  -e "s|@SHORT_VERSION@|$(sed_escape "$short_version")|g" \
  -e "s|@BUILD_VERSION@|$(sed_escape "$build_version")|g" \
  -e "s|@MIN_MACOS@|$(sed_escape "$MIN_MACOS")|g" \
  -e "s|@COPYRIGHT@|$(sed_escape "$COPYRIGHT")|g" \
  "$PACKAGING/Info.plist" > "$app/Contents/Info.plist"
if grep -q '@[A-Z_]*@' "$app/Contents/Info.plist"; then die "unreplaced placeholder in Info.plist"; fi
plutil -lint -s "$app/Contents/Info.plist"

identity="${APPLE_SIGNING_IDENTITY:-}"
profile="${APPLE_PROVISIONING_PROFILE_PATH:-}"
entitlements="$PACKAGING/entitlements.plist"
if [ -n "$identity" ]; then
  log "signing with '$identity' (hardened runtime, secure timestamp)"
  timestamp=(--timestamp)
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
  warn "APPLE_SIGNING_IDENTITY not set: ad-hoc signing (fine locally; cannot be notarized)"
  [ -z "$profile" ] || warn "ignoring APPLE_PROVISIONING_PROFILE_PATH: only Developer ID builds carry its entitlement"
  identity="-"
  timestamp=(--timestamp=none)
fi
# Two Mach-Os, so the nested one is signed first and the bundle's seal then covers it (no --deep:
# it would re-sign everything with the app's entitlements). The host always gets the base
# entitlements: macOS attributes a terminal program's privacy (TCC) requests to the process that
# started it, and the host is that process, so it needs the same resource-access set as the app.
# The restricted Time Sensitive entitlement stays with the app alone: the embedded profile
# authorizes the main executable, and a nested one claiming it would not launch.
codesign --force --sign "$identity" --options runtime "${timestamp[@]}" \
  --identifier "$BUNDLE_ID.host" --entitlements "$PACKAGING/entitlements.plist" \
  "$app/Contents/MacOS/$HOST_BINARY"
codesign --force --sign "$identity" --options runtime "${timestamp[@]}" \
  --entitlements "$entitlements" "$app"
# --deep here only widens the check to the nested host; it signs nothing.
codesign --verify --deep --strict --verbose=2 "$app"
"$ROOT/scripts/check-entitlements.sh" "$app"

log "done: $app"
lipo -archs "$app/Contents/MacOS/$APP_BINARY" >&2
lipo -archs "$app/Contents/MacOS/$HOST_BINARY" >&2
