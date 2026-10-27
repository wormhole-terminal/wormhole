#!/usr/bin/env bash
# Packages bundled (and ideally notarized + stapled) Wormhole.apps for release. A release has three, one
# per build: universal, Apple silicon (aarch64) and Intel (x86_64); each is named by its build (the
# universal one without a suffix, the names every release before had):
#
#   dist/Wormhole-<v>-macos[-<arch>].dmg               human download (Developer ID signed + notarized
#                                                      when possible)
#   dist/Wormhole-macos[-<arch>].dmg                   the same, under a name that stays put: what the
#                                                      website's links reach through releases/latest/download/
#   dist/Wormhole-<v>-macos[-<arch>].app.tar.gz        update archive (what the updater downloads), and its
#   dist/Wormhole-<v>-macos[-<arch>].app.tar.gz.minisig  minisign signature
#   dist/SHA256SUMS
#   dist/latest.json                                   update feed
#
# The feed also lists each DMG under its stable name and size ("dmgs": {"<build>": {name, size}}), so the
# website can show the download's size and whether an Intel one exists without asking for the files.
#
# The feed gives each Mac the archive of its own architecture: a thin app where there is one, else the
# universal one; so with all three the universal app has a DMG and no archive.
#
# All of them go onto the GitHub Release vX.Y.Z (a build of main: the rolling release `unstable`). The
# app reads <PUBLIC_BASE_URL>/<channel>/latest.json, which redirects to that release's latest.json,
# and that names the archives on its release.
#
#   scripts/package.sh [--dist DIR] [path/to/Wormhole.app ...]   (default: target/release/Wormhole.app)
#
# Env:
#   GITHUB_REPOSITORY    owner/name whose Releases host the files (set in Actions; default
#                        wormhole-terminal/wormhole)
#   RELEASE_TAG          the release the files go onto (default v<version>; `unstable` for main builds)
#   RELEASE_NOTES        Markdown notes for latest.json, or RELEASE_NOTES_FILE=path
#   RELEASE_SEVERITY     critical|high|medium|low: this version is a security update, so latest.json gets
#                        "security": {"severity": "<level>", "advisories": ["GHSA-…"]}. Without it
#                        there is no "security" key. Both variables come from CHANGELOG.md
#                        (scripts/changelog.py severity [--advisories]).
#   RELEASE_ADVISORIES   the advisories' GHSA ids, one per line (none: an empty list)
#   MINISIGN_SECRET_KEY  contents of the minisign secret key file (or a path to it)
#   MINISIGN_PASSWORD    its password
#     Without MINISIGN_SECRET_KEY the DEV key packaging/dev-updater.key (password
#     "wormhole-dev") is used for local testing, unless REQUIRE_RELEASE_KEY=1.
#   APPLE_SIGNING_IDENTITY + APPLE_API_* (see notarize.sh): sign + notarize the dmg.
# Needs: minisign (brew install minisign), jq, hdiutil, shasum.
set -euo pipefail
. "$(dirname "$0")/lib.sh"

dist="$DIST_DEFAULT"
apps=()
while [ $# -gt 0 ]; do
  case "$1" in
    --dist) shift; dist="${1:?--dist needs a directory}" ;;
    -*) die "unknown argument: $1" ;;
    *) apps+=("$1") ;;
  esac
  shift
done
[ ${#apps[@]} -gt 0 ] || apps=("$APP_PATH_DEFAULT")
need minisign "Install it: brew install minisign"
need jq

base_url="https://github.com/${GITHUB_REPOSITORY:-wormhole-terminal/wormhole}/releases/download/${RELEASE_TAG:-v$APP_VERSION}"

# An app's build: universal, aarch64 or x86_64, from the architectures its binary holds.
build_of() {
  case "$(lipo -archs "$1/Contents/MacOS/$APP_BINARY" | tr ' ' '\n' | sort | tr '\n' ' ')" in
    "arm64 x86_64 ") echo universal ;;
    "arm64 ") echo aarch64 ;;
    "x86_64 ") echo x86_64 ;;
    *) die "could not determine the architectures of $1" ;;
  esac
}
# The suffix of a build's file names: none for the universal one, as before there were three.
suffix_of() { [ "$1" = universal ] || printf -- '-%s' "$1"; }

# Bash 3.2 (macOS's) has no associative arrays: a build's app is the variable app_<build>.
# shellcheck disable=SC2034  # read through app_of's eval
app_universal="" app_aarch64="" app_x86_64=""
app_of() { eval "printf '%s' \"\${app_$1}\""; }
for app in "${apps[@]}"; do
  [ -d "$app" ] || die "no app bundle at $app (run scripts/bundle.sh first)"
  bundled_version="$(plutil -extract CFBundleShortVersionString raw "$app/Contents/Info.plist")"
  [ "$bundled_version" = "${APP_VERSION%%[-+]*}" ] \
    || die "$app is version $bundled_version but Cargo.toml says $APP_VERSION (re-run bundle.sh)"
  # --deep so the nested session host is checked too, as the updater does with the downloaded app.
  [ -x "$app/Contents/MacOS/$HOST_BINARY" ] || die "$app has no Contents/MacOS/$HOST_BINARY (re-run bundle.sh)"
  codesign --verify --deep --strict "$app" || die "code signature of $app is invalid"
  if is_developer_id_signed "$app"; then
    xcrun stapler validate -q "$app" >/dev/null 2>&1 \
      || warn "$app is Developer ID signed but has no stapled notarization ticket (run scripts/notarize.sh first)"
  else
    warn "$app is ad-hoc signed: fine for testing, Gatekeeper will block it on other Macs"
  fi
  # The host serves the same platforms as the app: a lone arm64 host in a universal app breaks Intel Macs.
  [ "$(lipo -archs "$app/Contents/MacOS/$HOST_BINARY" | tr ' ' '\n' | sort | tr '\n' ' ')" = \
    "$(lipo -archs "$app/Contents/MacOS/$APP_BINARY" | tr ' ' '\n' | sort | tr '\n' ' ')" ] \
    || die "$HOST_BINARY and $APP_BINARY in $app are built for different architectures"
  build="$(build_of "$app")"
  [ -z "$(app_of "$build")" ] || die "two $build apps: $(app_of "$build") and $app"
  eval "app_$build=\$app"
done

# Which build each of the updater's platform keys (darwin-<arch>) gets: its own thin one, else
# universal, else none.
feed_build() {
  if [ -n "$(app_of "$1")" ]; then echo "$1"
  elif [ -n "$app_universal" ]; then echo universal
  fi
}

rm -rf "$dist"
mkdir -p "$dist"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# --- minisign key -------------------------------------------------------------
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

# --- update archive -----------------------------------------------------------
# bsdtar keeps symlinks, modes and the code signature (which lives in regular files,
# not xattrs). COPYFILE_DISABLE/--no-mac-metadata keep AppleDouble "._*" files out.
archive_of() { printf '%s-%s-macos%s.app.tar.gz' "$APP_NAME" "$APP_VERSION" "$(suffix_of "$1")"; }
make_archive() {
  local build="$1" app name archive
  app="$(app_of "$1")"
  name="$(archive_of "$build")"
  archive="$dist/$name"
  log "creating $name"
  COPYFILE_DISABLE=1 tar --no-mac-metadata --no-xattrs -czf "$archive" \
    -C "$(dirname "$app")" "$(basename "$app")"
  # Round-trip check: the extracted copy must still carry a valid signature.
  rm -rf "$work/extract" && mkdir "$work/extract"
  tar -xzf "$archive" -C "$work/extract"
  codesign --verify --deep --strict "$work/extract/$(basename "$app")" \
    || die "extracted app from $name fails codesign --verify"
  log "signing $name with minisign"
  # The trusted comment is covered by the signature, so the updater may check it
  # names the version and the file it expects (protects against replaying an older signed archive).
  printf '%s\n' "$password" | minisign -S -s "$seckey" -m "$archive" -x "$archive.minisig" \
    -t "$APP_NAME $APP_VERSION $name" >/dev/null 2>"$work/minisign.err" \
    || { cat "$work/minisign.err" >&2; die "minisign failed (wrong MINISIGN_PASSWORD?)"; }
  # Catch a secret key that doesn't match the public key embedded in the app.
  minisign -V -q -p "$pubkey" -m "$archive" -x "$archive.minisig" \
    || die "signature does not verify with $(basename "$pubkey"): the secret key does not match the committed public key"
}
archived=()
for build in $( (feed_build aarch64; feed_build x86_64) | sort -u); do
  make_archive "$build"
  archived+=("$build")
done

# --- dmg ----------------------------------------------------------------------
sums=()
dmgs='{}'
for build in universal aarch64 x86_64; do
  [ -n "$(app_of "$build")" ] || continue
  name="$APP_NAME-$APP_VERSION-macos$(suffix_of "$build").dmg"
  dmg="$dist/$name"
  log "creating $name"
  # The window, background and volume icon: scripts/make-dmg.sh (Finder lays it out, so it needs a desktop
  # session; without one it warns and makes a plain image, unless REQUIRE_DMG_LAYOUT=1).
  "$(dirname "$0")/make-dmg.sh" "$(app_of "$build")" "$dmg"
  if [ -n "${APPLE_SIGNING_IDENTITY:-}" ]; then
    codesign --force --sign "$APPLE_SIGNING_IDENTITY" --timestamp "$dmg"
    "$(dirname "$0")/notarize.sh" "$dmg"
  fi
  hdiutil verify -quiet "$dmg"
  stable="$APP_NAME-macos$(suffix_of "$build").dmg"
  cp "$dmg" "$dist/$stable"
  dmgs="$(jq -n --argjson dmgs "$dmgs" --arg build "$build" --arg name "$stable" \
    --argjson size "$(stat -f%z "$dmg")" '$dmgs + {($build): {name: $name, size: $size}}')"
  sums+=("$name")
done
for build in "${archived[@]}"; do
  sums+=("$(archive_of "$build")" "$(archive_of "$build").minisig")
done

# --- checksums + feed ---------------------------------------------------------
(cd "$dist" && shasum -a 256 "${sums[@]}" > SHA256SUMS)

notes="${RELEASE_NOTES:-}"
if [ -n "${RELEASE_NOTES_FILE:-}" ]; then notes="$(cat "$RELEASE_NOTES_FILE")"; fi
[ -n "$notes" ] || notes="$APP_NAME $APP_VERSION"

security=null
if [ -n "${RELEASE_SEVERITY:-}" ]; then
  case "$RELEASE_SEVERITY" in critical|high|medium|low) ;; *) die "RELEASE_SEVERITY must be critical, high, medium or low, not '$RELEASE_SEVERITY'" ;; esac
  security="$(jq -n --arg severity "$RELEASE_SEVERITY" --arg advisories "${RELEASE_ADVISORIES:-}" \
    '{severity: $severity, advisories: ($advisories | split("\n") | map(select(length > 0)))}')"
fi

# Each platform's entry: its build's archive, checksum, signature and size.
platforms='{}'
for arch in aarch64 x86_64; do
  build="$(feed_build "$arch")"
  [ -n "$build" ] || continue
  platform="darwin-$arch"
  name="$(archive_of "$build")"
  platforms="$(jq -n \
    --argjson platforms "$platforms" \
    --arg platform "$platform" \
    --arg url "$base_url/$name" \
    --arg sha256 "$(shasum -a 256 "$dist/$name" | cut -d' ' -f1)" \
    --rawfile signature "$dist/$name.minisig" \
    --argjson size "$(stat -f%z "$dist/$name")" \
    '$platforms + {($platform): {url: $url, sha256: $sha256, signature: $signature, size: $size}}')"
done

jq -n \
  --arg version "$APP_VERSION" \
  --arg pub_date "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg notes "$notes" \
  --arg min "$MIN_MACOS" \
  --argjson platforms "$platforms" \
  --argjson dmgs "$dmgs" \
  --argjson security "$security" \
  '{
    version: $version,
    pub_date: $pub_date,
    notes: $notes,
    minimum_system_version: $min,
    platforms: $platforms,
    dmgs: $dmgs
  } + (if $security == null then {} else {security: $security} end)' > "$dist/latest.json"

log "wrote $dist:"
ls -l "$dist" >&2
