#!/usr/bin/env bash
# Drive the session hosts of earlier releases with this build's client (crates/wormhole-host/tests/previous_hosts.rs).
#
#   scripts/previous-hosts.sh [--dir <folder>] [<tag> ...]
#
# A session host outlives the app, so after an update today's app talks to a host built by an earlier release. For each
# tag (default: the last two stable releases) this downloads the release's app archive with `gh release download`,
# takes the `wormhole-host` out of it, re-signs it ad hoc (`codesign -s - -f`: the release's own signature names a team
# whose peer check would refuse the test's client, and a copy outside its bundle needs one to run) into
# <folder>/<tag>/, and runs the test with WORMHOLE_PREVIOUS_HOSTS=<folder>. The folder defaults to a new temporary one,
# kept so the hosts can be run again by hand; nothing outside it is touched.
set -euo pipefail
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"

need gh "Install the GitHub CLI and run 'gh auth login'."
need codesign
need tar
need cargo

REPO="${WORMHOLE_RELEASES_REPO:-wormhole-terminal/wormhole}"
dir=""
tags=()
while [ $# -gt 0 ]; do
  case "$1" in
    --dir) dir="${2:?--dir needs a folder}"; shift 2 ;;
    -h|--help) sed -n '2,13p' "$0"; exit 0 ;;
    -*) die "unknown option $1" ;;
    *) tags+=("$1"); shift ;;
  esac
done

if [ -z "$dir" ]; then dir="$(mktemp -d "${TMPDIR:-/tmp}/wormhole-previous-hosts.XXXXXX")"; fi
mkdir -p "$dir"

if [ ${#tags[@]} -eq 0 ]; then
  # The two newest stable releases (a preview is not a release a person keeps running a host from).
  while IFS= read -r tag; do tags+=("$tag"); done < <(
    gh release list --repo "$REPO" --exclude-pre-releases --limit 2 --json tagName --jq '.[].tagName'
  )
fi
[ ${#tags[@]} -gt 0 ] || die "no releases found in $REPO"

case "$(uname -m)" in
  arm64) arch=aarch64 ;;
  *) arch="$(uname -m)" ;;
esac

for tag in "${tags[@]}"; do
  log "$tag"
  work="$dir/.download-$tag"
  rm -rf "$work"
  mkdir -p "$work" "$dir/$tag"
  # Releases from 1.5 name the architecture; the first ones shipped one archive for both.
  gh release download "$tag" --repo "$REPO" --pattern "Wormhole-*-macos-$arch.app.tar.gz" --dir "$work" 2>/dev/null \
    || gh release download "$tag" --repo "$REPO" --pattern "Wormhole-*-macos.app.tar.gz" --dir "$work" \
    || die "$tag has no app archive"
  archive="$(find "$work" -name '*.app.tar.gz' | head -n 1)"
  [ -n "$archive" ] || die "$tag: no archive downloaded"
  # Only the host, wherever the archive keeps it.
  tar -xzf "$archive" -C "$work" --include "*/$HOST_BINARY" || die "$tag: no $HOST_BINARY in the archive"
  host="$(find "$work" -name "$HOST_BINARY" -type f | head -n 1)"
  [ -n "$host" ] || die "$tag: no $HOST_BINARY in the archive (a release before hosts?)"
  cp "$host" "$dir/$tag/$HOST_BINARY"
  codesign -s - -f "$dir/$tag/$HOST_BINARY" 2>/dev/null
  rm -rf "$work"
done

log "driving the hosts in $dir with this build's client"
cd "$ROOT"
WORMHOLE_PREVIOUS_HOSTS="$dir" cargo test -p wormhole-host --test previous_hosts -- --nocapture
