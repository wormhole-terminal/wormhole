#!/usr/bin/env bash
# Packages a bundled (and ideally notarized + stapled) Wormhole.app for release:
#
#   dist/Wormhole-<v>-macos.app.tar.gz          update archive (what the updater downloads)
#   dist/Wormhole-<v>-macos.app.tar.gz.minisig  minisign signature of the archive
#   dist/Wormhole-<v>-macos.dmg                 human download (Developer ID signed + notarized when possible)
#   dist/SHA256SUMS
#   dist/latest.json                              update feed, published as <BASE_URL>/<channel>/latest.json
#
#   scripts/package.sh [path/to/Wormhole.app] [dist-dir]
#
# Env:
#   PUBLIC_BASE_URL      public download root, e.g. https://downloads.example.com/wormhole (required
#                        for a real release; defaults to a placeholder with a warning)
#   RELEASE_NOTES        Markdown notes for latest.json, or RELEASE_NOTES_FILE=path
#   MINISIGN_SECRET_KEY  contents of the minisign secret key file (or a path to it)
#   MINISIGN_PASSWORD    its password
#     Without MINISIGN_SECRET_KEY the DEV key packaging/dev-updater.key (password
#     "wormhole-dev") is used for local testing, unless REQUIRE_RELEASE_KEY=1.
#   APPLE_SIGNING_IDENTITY + APPLE_API_* (see notarize.sh): sign + notarize the dmg.
# Needs: minisign (brew install minisign), jq, hdiutil, shasum.
set -euo pipefail
. "$(dirname "$0")/lib.sh"

app="${1:-$APP_PATH_DEFAULT}"
dist="${2:-$DIST_DEFAULT}"
[ -d "$app" ] || die "no app bundle at $app (run scripts/bundle.sh first)"
need minisign "Install it: brew install minisign"
need jq

base_url="${PUBLIC_BASE_URL:-}"
if [ -z "$base_url" ]; then
  [ "${REQUIRE_RELEASE_KEY:-0}" = 1 ] && die "PUBLIC_BASE_URL is not set"
  base_url="https://downloads.example.com/wormhole"
  warn "PUBLIC_BASE_URL not set: using placeholder $base_url in latest.json"
fi
base_url="${base_url%/}"

bundled_version="$(plutil -extract CFBundleShortVersionString raw "$app/Contents/Info.plist")"
[ "$bundled_version" = "${APP_VERSION%%[-+]*}" ] \
  || die "app bundle is version $bundled_version but Cargo.toml says $APP_VERSION (re-run bundle.sh)"

codesign --verify --strict "$app" || die "code signature of $app is invalid"
if is_developer_id_signed "$app"; then
  xcrun stapler validate -q "$app" >/dev/null 2>&1 \
    || warn "$app is Developer ID signed but has no stapled notarization ticket (run scripts/notarize.sh first)"
else
  warn "$app is ad-hoc signed: fine for testing, Gatekeeper will block it on other Macs"
fi

# darwin-<arch> keys the updater looks up; a universal binary serves both.
platforms=()
for a in $(lipo -archs "$app/Contents/MacOS/$APP_BINARY"); do
  case "$a" in arm64) platforms+=(darwin-aarch64) ;; x86_64) platforms+=(darwin-x86_64) ;; esac
done
[ ${#platforms[@]} -gt 0 ] || die "could not determine architectures of $app"

rm -rf "$dist"
mkdir -p "$dist"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# --- update archive -----------------------------------------------------------
# bsdtar keeps symlinks, modes and the code signature (which lives in regular files,
# not xattrs). COPYFILE_DISABLE/--no-mac-metadata keep AppleDouble "._*" files out.
archive="$dist/$ARCHIVE_NAME"
log "creating $ARCHIVE_NAME"
COPYFILE_DISABLE=1 tar --no-mac-metadata --no-xattrs -czf "$archive" \
  -C "$(dirname "$app")" "$(basename "$app")"
# Round-trip check: the extracted copy must still carry a valid signature.
mkdir "$work/extract"
tar -xzf "$archive" -C "$work/extract"
codesign --verify --strict "$work/extract/$(basename "$app")" \
  || die "extracted app from the archive fails codesign --verify"

# --- minisign -----------------------------------------------------------------
if [ -n "${MINISIGN_SECRET_KEY:-}" ]; then
  seckey="$work/updater.key"
  if [ -f "$MINISIGN_SECRET_KEY" ]; then cp "$MINISIGN_SECRET_KEY" "$seckey"
  else printf '%s\n' "$MINISIGN_SECRET_KEY" > "$seckey"; fi
  chmod 600 "$seckey"
  password="${MINISIGN_PASSWORD:-}"
  pubkey="$PACKAGING/updater.pub"
else
  [ "${REQUIRE_RELEASE_KEY:-0}" = 1 ] && die "MINISIGN_SECRET_KEY is not set"
  seckey="$PACKAGING/dev-updater.key"
  [ -f "$seckey" ] || die "no MINISIGN_SECRET_KEY and no dev key at $seckey (minisign -G -p packaging/dev-updater.pub -s packaging/dev-updater.key, password wormhole-dev)"
  password="wormhole-dev"
  pubkey="$PACKAGING/dev-updater.pub"
  warn "signing with the DEV updater key ($seckey): testing only"
fi
log "signing archive with minisign"
# The trusted comment is covered by the signature, so the updater may check it
# names the version it expects (protects against replaying an older signed archive).
printf '%s\n' "$password" | minisign -S -s "$seckey" -m "$archive" -x "$archive.minisig" \
  -t "$APP_NAME $APP_VERSION $ARCHIVE_NAME" >/dev/null 2>"$work/minisign.err" \
  || { cat "$work/minisign.err" >&2; die "minisign failed (wrong MINISIGN_PASSWORD?)"; }
# Catch a secret key that doesn't match the public key embedded in the app.
minisign -V -q -p "$pubkey" -m "$archive" -x "$archive.minisig" \
  || die "signature does not verify with $(basename "$pubkey"): the secret key does not match the committed public key"

# --- dmg ----------------------------------------------------------------------
dmg="$dist/$DMG_NAME"
log "creating $DMG_NAME"
# The window, background and volume icon: scripts/make-dmg.sh (Finder lays it out, so it needs a desktop
# session; without one it warns and makes a plain image, unless REQUIRE_DMG_LAYOUT=1).
"$(dirname "$0")/make-dmg.sh" "$app" "$dmg"
if [ -n "${APPLE_SIGNING_IDENTITY:-}" ]; then
  codesign --force --sign "$APPLE_SIGNING_IDENTITY" --timestamp "$dmg"
  "$(dirname "$0")/notarize.sh" "$dmg"
fi
hdiutil verify -quiet "$dmg"

# --- checksums + feed ---------------------------------------------------------
(cd "$dist" && shasum -a 256 "$ARCHIVE_NAME" "$ARCHIVE_NAME.minisig" "$DMG_NAME" > SHA256SUMS)
sha256="$(shasum -a 256 "$archive" | cut -d' ' -f1)"
size="$(stat -f%z "$archive")"

notes="${RELEASE_NOTES:-}"
if [ -n "${RELEASE_NOTES_FILE:-}" ]; then notes="$(cat "$RELEASE_NOTES_FILE")"; fi
[ -n "$notes" ] || notes="$APP_NAME $APP_VERSION"

url="$base_url/releases/$APP_VERSION/$ARCHIVE_NAME"
jq -n \
  --arg version "$APP_VERSION" \
  --arg pub_date "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg notes "$notes" \
  --arg min "$MIN_MACOS" \
  --arg url "$url" \
  --arg sha256 "$sha256" \
  --rawfile signature "$archive.minisig" \
  --argjson size "$size" \
  --args '{
    version: $version,
    pub_date: $pub_date,
    notes: $notes,
    minimum_system_version: $min,
    platforms: ($ARGS.positional | sort | map({key: ., value: {url: $url, sha256: $sha256, signature: $signature, size: $size}}) | from_entries)
  }' "${platforms[@]}" > "$dist/latest.json"

log "wrote $dist:"
ls -l "$dist" >&2
