#!/usr/bin/env bash
# Uploads the output of package.sh to Cloudflare R2 through its S3 API, using the
# AWS CLI (preinstalled on GitHub's macOS runners; locally: brew install awscli).
#
#   scripts/publish-r2.sh [dist-dir]      # default: target/release/dist
#
# Env:
#   R2_ACCOUNT_ID, R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY, R2_BUCKET
#   PUBLIC_BASE_URL  public URL that serves R2_PREFIX of the bucket (must match latest.json)
#   R2_PREFIX        optional key prefix inside the bucket, e.g. "wormhole" when
#                    PUBLIC_BASE_URL=https://downloads.example.com/wormhole on a custom
#                    domain bound to the bucket root. Default: none.
#   DRY_RUN=1        print what would be uploaded, touch nothing (no credentials needed)
#   FORCE=1          allow overwriting an already published version
#
# Layout (relative to PUBLIC_BASE_URL):
#   releases/<v>/Wormhole-<v>-macos.app.tar.gz(.minisig)  immutable, cached for a year
#   releases/<v>/Wormhole-<v>-macos.dmg, SHA256SUMS, latest.json
#   stable/Wormhole-latest.dmg                             short cache
#   stable/latest.json                                       uploaded LAST, short cache
set -euo pipefail
. "$(dirname "$0")/lib.sh"

dist="${1:-$DIST_DEFAULT}"
dry="${DRY_RUN:-0}"
feed="$dist/latest.json"
for f in "$ARCHIVE_NAME" "$ARCHIVE_NAME.minisig" "$DMG_NAME" SHA256SUMS latest.json; do
  [ -f "$dist/$f" ] || die "missing $dist/$f (run scripts/package.sh first)"
done

missing=()
for v in R2_ACCOUNT_ID R2_ACCESS_KEY_ID R2_SECRET_ACCESS_KEY R2_BUCKET PUBLIC_BASE_URL; do
  [ -n "${!v:-}" ] || missing+=("$v")
done
if [ ${#missing[@]} -gt 0 ]; then
  [ "$dry" = 1 ] || die "not configured, missing: ${missing[*]}"
  warn "DRY_RUN: missing ${missing[*]}, using placeholders"
  : "${R2_ACCOUNT_ID:=ACCOUNT}" "${R2_BUCKET:=BUCKET}" "${PUBLIC_BASE_URL:=https://downloads.example.com/wormhole}"
fi
base_url="${PUBLIC_BASE_URL%/}"
prefix="${R2_PREFIX:-}"; prefix="${prefix#/}"; prefix="${prefix%/}"; [ -n "$prefix" ] && prefix="$prefix/"

# Consistency: the feed must be for this version and point at where we upload.
feed_version="$(jq -r .version "$feed")"
[ "$feed_version" = "$APP_VERSION" ] || die "latest.json is for $feed_version, Cargo.toml says $APP_VERSION"
expected_url="$base_url/releases/$APP_VERSION/$ARCHIVE_NAME"
bad="$(jq -r --arg u "$expected_url" '[.platforms[] | select(.url != $u)] | length' "$feed")"
[ "$bad" = 0 ] || die "latest.json URLs don't match $expected_url (package.sh ran with a different PUBLIC_BASE_URL?)"

endpoint="https://$R2_ACCOUNT_ID.r2.cloudflarestorage.com"
export AWS_ACCESS_KEY_ID="${R2_ACCESS_KEY_ID:-}" AWS_SECRET_ACCESS_KEY="${R2_SECRET_ACCESS_KEY:-}"
export AWS_DEFAULT_REGION=auto AWS_EC2_METADATA_DISABLED=true
# AWS CLI >= 2.23 sends new default checksums R2 doesn't accept; only send them when required.
export AWS_REQUEST_CHECKSUM_CALCULATION=when_required AWS_RESPONSE_CHECKSUM_VALIDATION=when_required
[ "$dry" = 1 ] || need aws "Install the AWS CLI: brew install awscli"

s3() { if [ "$dry" = 1 ]; then echo "DRY_RUN: aws $*" >&2; else aws --endpoint-url "$endpoint" "$@"; fi; }

LONG="public, max-age=31536000, immutable"
SHORT="public, max-age=300"
FEED="public, max-age=60, must-revalidate"

put() { # file key content-type cache-control [content-disposition]
  local extra=()
  [ -n "${5:-}" ] && extra=(--content-disposition "$5")
  log "upload $(basename "$1") -> s3://$R2_BUCKET/$prefix$2"
  s3 s3 cp --only-show-errors "$1" "s3://$R2_BUCKET/$prefix$2" \
    --content-type "$3" --cache-control "$4" ${extra[@]+"${extra[@]}"}
}

rel="releases/$APP_VERSION"
if [ "$dry" != 1 ] && [ "${FORCE:-0}" != 1 ] \
   && aws --endpoint-url "$endpoint" s3api head-object --bucket "$R2_BUCKET" --key "$prefix$rel/$ARCHIVE_NAME" >/dev/null 2>&1; then
  die "version $APP_VERSION is already published ($prefix$rel/$ARCHIVE_NAME). Bump the version, or FORCE=1 to overwrite."
fi

# 1. Versioned, immutable files first...
put "$dist/$ARCHIVE_NAME"         "$rel/$ARCHIVE_NAME"         application/gzip "$LONG"
put "$dist/$ARCHIVE_NAME.minisig" "$rel/$ARCHIVE_NAME.minisig" text/plain       "$LONG"
put "$dist/$DMG_NAME"             "$rel/$DMG_NAME"             application/x-apple-diskimage "$LONG"
put "$dist/SHA256SUMS"            "$rel/SHA256SUMS"            text/plain       "$LONG"
put "$feed"                       "$rel/latest.json"           application/json "$LONG"
# 2. ...then the moving "latest" download...
put "$dist/$DMG_NAME" "$UPDATE_CHANNEL/$APP_NAME-latest.dmg" application/x-apple-diskimage "$SHORT" \
  "attachment; filename=\"$DMG_NAME\""
# 3. ...and the feed LAST, so no client ever sees a feed pointing at missing files.
put "$feed" "$UPDATE_CHANNEL/latest.json" application/json "$FEED"

if [ "$dry" != 1 ]; then
  for u in "$expected_url" "$base_url/$UPDATE_CHANNEL/latest.json" "$base_url/$UPDATE_CHANNEL/$APP_NAME-latest.dmg"; do
    if curl -fsSI -o /dev/null "$u"; then log "public OK: $u"
    else warn "not publicly reachable (yet?): $u  (check the bucket's public access / custom domain)"; fi
  done
fi
log "published $APP_VERSION to $base_url"
