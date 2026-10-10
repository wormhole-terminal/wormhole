#!/usr/bin/env bash
# Cuts a release from this Mac, so it costs no Actions minutes (macOS runners count 10x):
# build, Developer ID signature, notarization, packaging, upload to R2, the tag and the GitHub
# Release from a Mac: sign, notarize, publish to R2 and push the tag.
#
#   op run --env-file=packaging/release.op.env -- scripts/release.sh --dry-run
#   op run --env-file=packaging/release.op.env -- scripts/release.sh
#
#   --dry-run   everything up to and including the package, then show what would be uploaded; nothing is
#               uploaded, tagged or released
#   --yes       skip the question before publishing
#   --universal build for Apple Silicon and Intel (an app with both, one update archive that both feed
#               entries point at). The default is Apple Silicon only: about half the build time and size.
#
# Before running it: bump the version in Cargo.toml, commit and push it to main, then make the annotated
# tag locally with the release notes as its message (do not push it, this script does):
#   git tag -a v0.2.0 -m "$(cat notes.md)"
#
# Env (see packaging/release.op.env.example): APPLE_TEAM_ID, PUBLIC_BASE_URL, APPLE_API_KEY_ID,
# APPLE_API_ISSUER, APPLE_API_KEY_P8, MINISIGN_SECRET_KEY, MINISIGN_PASSWORD and, to publish,
# R2_ACCOUNT_ID, R2_BUCKET, R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY. APPLE_SIGNING_IDENTITY is taken from
# the login keychain when unset; APPLE_PROVISIONING_PROFILE_PATH and R2_PREFIX are optional.
#
# Order matters: the upload comes first and the tag after it. Pushing a tag starts the Release
# workflow, and its first (Linux) job stops the run when the version is already on R2.
set -euo pipefail
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"

dry=0 yes=0 universal=0
for arg in "$@"; do
  case "$arg" in
    --dry-run) dry=1 ;;
    --yes) yes=1 ;;
    --universal) universal=1 ;;
    *) die "unknown option: $arg (see the top of this script)" ;;
  esac
done

tag="v$APP_VERSION"
cd "$ROOT"

# --- what is being released ---------------------------------------------------
need git; need gh "brew install gh"; need jq; need minisign "brew install minisign"
[ "$dry" = 1 ] || need aws "brew install awscli"

git fetch --quiet origin
[ "$(git rev-parse --abbrev-ref HEAD)" = main ] || die "not on main"
[ -z "$(git status --porcelain)" ] || die "the working tree has changes: commit or stash them first"
[ "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" ] \
  || die "HEAD is not origin/main: push your commits (or pull) first, the tag must point at what CI would build"
[ "$(git cat-file -t "refs/tags/$tag" 2>/dev/null)" = tag ] \
  || die "no annotated tag $tag: git tag -a $tag -m \"<release notes>\" (a lightweight tag has no notes)"
[ "$(git rev-parse "$tag^{commit}")" = "$(git rev-parse HEAD)" ] || die "$tag does not point at HEAD"
if [ -n "$(git ls-remote --tags origin "refs/tags/$tag")" ]; then die "$tag is already on origin"; fi
python3 scripts/gen-licenses.py --check || die "licenses.json is out of date: run scripts/gen-licenses.py and commit it"

# --- configuration ------------------------------------------------------------
missing=()
required=(APPLE_TEAM_ID PUBLIC_BASE_URL APPLE_API_KEY_ID APPLE_API_ISSUER APPLE_API_KEY_P8 MINISIGN_SECRET_KEY MINISIGN_PASSWORD)
[ "$dry" = 1 ] || required+=(R2_ACCOUNT_ID R2_BUCKET R2_ACCESS_KEY_ID R2_SECRET_ACCESS_KEY)
for v in "${required[@]}"; do [ -n "${!v:-}" ] || missing+=("$v"); done
[ ${#missing[@]} -eq 0 ] || die "not set: ${missing[*]} (packaging/release.op.env.example)"

if [ -z "${APPLE_SIGNING_IDENTITY:-}" ]; then
  APPLE_SIGNING_IDENTITY="$(security find-identity -v -p codesigning \
    | grep -o '"Developer ID Application: [^"]*"' | head -1 | tr -d '"')"
fi
[ -n "$APPLE_SIGNING_IDENTITY" ] || die "no 'Developer ID Application' certificate in the keychain"
log "signing as $APPLE_SIGNING_IDENTITY"
export APPLE_SIGNING_IDENTITY APPLE_TEAM_ID PUBLIC_BASE_URL

notes="$(mktemp)"
trap 'rm -f "$notes"' EXIT
git tag -l --format='%(contents)' "$tag" | sed '/-----BEGIN PGP SIGNATURE-----/,$d' > "$notes"
[ -n "$(tr -d '[:space:]' < "$notes")" ] || die "the tag $tag has an empty message"
export RELEASE_NOTES_FILE="$notes"

# --- build, sign, notarize, package ---------------------------------------------
[ -z "${APPLE_PROVISIONING_PROFILE_PATH:-}" ] || export APPLE_PROVISIONING_PROFILE_PATH
bundle_args=()
[ "$universal" = 1 ] && bundle_args+=(--universal)
"$ROOT/scripts/bundle.sh" ${bundle_args[@]+"${bundle_args[@]}"}
REQUIRE_NOTARIZATION=1 "$ROOT/scripts/notarize.sh"
# A release from a Mac always has the styled disk image: fail instead of shipping a plain one.
REQUIRE_DMG_LAYOUT=1 REQUIRE_RELEASE_KEY=1 REQUIRE_NOTARIZATION=1 "$ROOT/scripts/package.sh"
dist="$DIST_DEFAULT"

if [ "$dry" = 1 ]; then
  DRY_RUN=1 "$ROOT/scripts/publish-r2.sh" "$dist"
  log "dry run done: $dist is what would be published for $tag. Nothing was uploaded, tagged or released."
  exit 0
fi

# --- publish ----------------------------------------------------------------------
if [ "$yes" != 1 ]; then
  printf 'Publish %s to %s? This cannot be undone (a published version is never overwritten). [y/N] ' \
    "$tag" "$PUBLIC_BASE_URL" >&2
  read -r answer
  [ "$answer" = y ] || [ "$answer" = Y ] || die "not published"
fi

"$ROOT/scripts/publish-r2.sh" "$dist"
git push origin "$tag"
flags=(--title "$APP_NAME $APP_VERSION" --notes-file "$notes" --verify-tag)
case "$APP_VERSION" in *-*) flags+=(--prerelease) ;; esac
gh release create "$tag" "$dist"/* "${flags[@]}"
log "released $tag"
