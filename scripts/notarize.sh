#!/usr/bin/env bash
# Notarizes a Developer ID-signed Wormhole.app or .dmg with Apple and staples the
# ticket, using an App Store Connect API key (no Apple ID password needed):
#
#   APPLE_API_KEY_ID    key ID, e.g. 2X9R4HXF34
#   APPLE_API_ISSUER    issuer ID (UUID) from App Store Connect > Users and Access > Integrations
#   APPLE_API_KEY_P8    the AuthKey_<id>.p8: a file path, the PEM text, or base64 of the file
#
#   scripts/notarize.sh [path]   # default: target/release/Wormhole.app
#
# Not configured / ad-hoc signed: prints why and exits 0 (so local builds work),
# unless REQUIRE_NOTARIZATION=1, then it fails.
set -euo pipefail
. "$(dirname "$0")/lib.sh"

target="${1:-$APP_PATH_DEFAULT}"
[ -e "$target" ] || die "not found: $target"

skip() {
  if [ "${REQUIRE_NOTARIZATION:-0}" = 1 ]; then die "cannot notarize: $*"; fi
  warn "skipping notarization: $*"
  exit 0
}

missing=()
for v in APPLE_API_KEY_ID APPLE_API_ISSUER APPLE_API_KEY_P8; do
  [ -n "${!v:-}" ] || missing+=("$v")
done
[ ${#missing[@]} -eq 0 ] || skip "${missing[*]} not set"
is_developer_id_signed "$target" || skip "$target is not signed with a Developer ID Application certificate"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

key="$work/AuthKey_$APPLE_API_KEY_ID.p8"
if [ -f "$APPLE_API_KEY_P8" ]; then
  cp "$APPLE_API_KEY_P8" "$key"
elif printf '%s' "$APPLE_API_KEY_P8" | grep -q 'BEGIN PRIVATE KEY'; then
  printf '%s\n' "$APPLE_API_KEY_P8" > "$key"
else
  printf '%s' "$APPLE_API_KEY_P8" | base64 --decode > "$key" 2>/dev/null \
    || die "APPLE_API_KEY_P8 is neither a file, PEM text nor valid base64"
fi
chmod 600 "$key"
grep -q 'BEGIN PRIVATE KEY' "$key" || die "APPLE_API_KEY_P8 does not contain a .p8 private key"

# notarytool accepts .dmg/.pkg/.zip; an .app has to be zipped (ditto keeps symlinks/xattrs).
case "$target" in
  *.app) upload="$work/$(basename "$target" .app).zip"
         ditto -c -k --sequesterRsrc --keepParent "$target" "$upload" ;;
  *.dmg|*.pkg|*.zip) upload="$target" ;;
  *) die "don't know how to notarize $target" ;;
esac

log "submitting $(basename "$upload") to Apple notary service (can take a few minutes)"
auth=(--key "$key" --key-id "$APPLE_API_KEY_ID" --issuer "$APPLE_API_ISSUER")
set +e
xcrun notarytool submit "$upload" "${auth[@]}" --wait --timeout 60m --output-format json > "$work/result.json"
rc=$?
set -e
cat "$work/result.json" >&2; echo >&2
id="$(jq -r '.id // empty' "$work/result.json" 2>/dev/null || true)"
status="$(jq -r '.status // empty' "$work/result.json" 2>/dev/null || true)"
if [ "$rc" != 0 ] || [ "$status" != "Accepted" ]; then
  if [ -n "$id" ]; then
    log "notary log for $id:"
    xcrun notarytool log "$id" "${auth[@]}" >&2 || true
  fi
  die "notarization failed (status: ${status:-unknown})"
fi

case "$target" in
  *.app|*.dmg|*.pkg)
    log "stapling ticket to $target"
    xcrun stapler staple "$target"
    xcrun stapler validate "$target" ;;
esac
log "notarized: $target"
