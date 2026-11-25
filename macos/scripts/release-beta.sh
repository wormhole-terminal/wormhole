#!/usr/bin/env bash
# Prepares a release of Wormhole Beta (the Swift app in macos/) on main: macos/CHANGELOG.md's [Unreleased] becomes the
# version, the bridge workspace's version (VERSION_MANIFEST in packaging/beta.env) and its Cargo.lock get it, one commit
# `chore(release): beta <version>`, and the annotated tag beta-<version> whose message is the notes. Nothing is pushed, and
# a pushed tag starts nothing: the Beta workflow (.github/workflows/beta.yml) is started by hand with the tag's name.
#
# The 1.x app is left as it is: its version (the root Cargo.toml), its CHANGELOG.md and its v* tags are what release.yml,
# preview.yml and `preview_version` read, and this touches none of them (the tag is never v*).
#
#   macos/scripts/release-beta.sh 2.0.0-beta.1             commit and tag, then print what to run next
#   macos/scripts/release-beta.sh 2.0.0-beta.1 --dry-run   show what would change and what would stop it; change nothing
#   macos/scripts/release-beta.sh 2.0.0-beta.1 --date 2026-11-25   the date in the changelog (default: today)
#
# Needs: a clean main that isn't behind origin/main, a version X.Y.Z-beta.N newer than the bridge's, and something under
# [Unreleased] in macos/CHANGELOG.md.
set -euo pipefail
# Before lib.sh reads it: APP_VERSION is then the bridge's version, not the 1.x app's.
export WORMHOLE_APP_ENV=packaging/beta.env
. "$(dirname "$0")/../../scripts/lib.sh"
cd "$ROOT"

version="" dry=0 date=""
while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run) dry=1 ;;
    --date) date="${2:?--date needs YYYY-MM-DD}"; shift ;;
    -h|--help) sed -n '2,15p' "$0"; exit 0 ;;
    -*) die "unknown argument: $1 (see --help)" ;;
    *) [ -z "$version" ] || die "one version only"; version="${1#beta-}" ;;
  esac
  shift
done
[ -n "$version" ] || die "usage: macos/scripts/release-beta.sh <version> [--dry-run] [--date YYYY-MM-DD]"
# The form the Beta workflow's gate accepts: a beta is always a prerelease.
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+-beta\.[0-9]+$ ]] || die "'$version' is not a beta version (X.Y.Z-beta.N)"
date="${date:-$(date +%F)}"
need python3
tag="beta-$version"
manifest="$VERSION_MANIFEST"
lock="$(dirname "$manifest")/Cargo.lock"
notes_file=macos/CHANGELOG.md
# changelog <file> <command> ...: scripts/changelog.py on a changelog whose tags are beta-<version>.
changelog() { python3 scripts/changelog.py --tag-prefix beta- --file "$@"; }

# What the 1.x app's release machinery reads, printed before and after so a run shows they did not move.
# An empty VERSION_MANIFEST is lib.sh's default, the root Cargo.toml.
guarded() {
  local root_version
  root_version="$(VERSION_MANIFEST='' app_version)"
  printf '  root Cargo.toml version: %s\n' "$root_version"
  printf '  preview_version:         %s\n' "$(APP_VERSION="$root_version" preview_version)"
  printf '  newest v* tags:          %s\n' "$(git tag --list 'v*' --sort=-v:refname | head -n 3 | tr '\n' ' ')"
}

# Everything that would stop a release; a dry run lists them and goes on, a real one stops.
problems=()
if [ "$(python3 -c '
import sys
sys.path.insert(0, "scripts")
import changelog
print(int(changelog.sem_key(sys.argv[1]) > changelog.sem_key(sys.argv[2])))' "$version" "$APP_VERSION")" != 1 ]; then
  problems+=("$version is not newer than $APP_VERSION, the version in $manifest")
fi
branch="$(git rev-parse --abbrev-ref HEAD)"
[ "$branch" = main ] || problems+=("on '$branch', not main")
[ -z "$(git status --porcelain)" ] || problems+=("the working tree isn't clean (commit first)")
if git fetch -q origin main 2>/dev/null; then
  behind="$(git rev-list --count HEAD..origin/main)"
  [ "$behind" = 0 ] || problems+=("main is $behind commit(s) behind origin/main (git pull --rebase)")
else
  problems+=("cannot reach origin to check main is up to date")
fi
if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then problems+=("the tag $tag exists already"); fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
# The new changelog is made on a copy first: it fails here, with nothing touched, when [Unreleased] is empty or the file
# is malformed.
cp "$notes_file" "$work/CHANGELOG.md"
changelog_ok=1
if ! changelog "$work/CHANGELOG.md" release "$version" --date "$date" 2>"$work/err"; then
  changelog_ok=0
  problems+=("$notes_file: $(sed 's/^error: //' "$work/err" | head -5)")
fi

if [ "$dry" = 1 ]; then
  log "dry run for Wormhole Beta $version ($date): nothing is changed"
  if [ ${#problems[@]} -gt 0 ]; then
    for p in "${problems[@]}"; do warn "would stop: $p"; done
  fi
  if [ "$changelog_ok" = 1 ]; then
    diff -u "$notes_file" "$work/CHANGELOG.md" | sed '1,2d' || true
    printf '\n%s: version = "%s" -> "%s"\n%s: the workspace crates follow (cargo update --workspace --offline)\n' \
      "$manifest" "$APP_VERSION" "$version" "$lock"
    printf '\ncommit: chore(release): beta %s\ntag:    %s\n\n--- tag message ---\nWormhole Beta %s\n\n' "$version" "$tag" "$version"
    changelog "$work/CHANGELOG.md" notes "$version"
  fi
  printf '\nThe 1.x app, which this does not touch:\n'
  guarded
  exit 0
fi

if [ ${#problems[@]} -gt 0 ]; then
  for p in "${problems[@]}"; do warn "$p"; done
  die "not releasing $version"
fi

# set_beta_version <version>: `[workspace.package] version` in the bridge's manifest, and its workspace crates' entries in
# its Cargo.lock (nothing else changes; the index is fetched only when there is no copy).
set_beta_version() {
  local tmp
  tmp="$(mktemp)"
  awk -v v="$1" '
    /^\[workspace\.package\]/ { f = 1; print; next }
    /^\[/ { f = 0 }
    f && !done && /^version[[:space:]]*=/ { print "version = \"" v "\""; done = 1; next }
    { print }' "$manifest" > "$tmp"
  cat "$tmp" > "$manifest"
  rm -f "$tmp"
  [ "$(app_version)" = "$1" ] || die "could not set the version in $manifest"
  (cd "$(dirname "$manifest")" && { cargo update --workspace --offline --quiet 2>/dev/null || cargo update --workspace --quiet; })
}

# A failure after the first file is written leaves a half-made release in the working tree; say how to undo it.
touched=0 committed=0
finish() {
  local rc=$?
  rm -rf "$work"
  if [ $rc -ne 0 ] && [ $touched = 1 ] && [ $committed = 0 ]; then
    warn "release failed; put the files back with: git restore $notes_file $manifest $lock"
  elif [ $rc -ne 0 ] && [ $committed = 1 ]; then
    warn "committed, but not tagged; tag it by hand or undo with: git reset --hard HEAD~1"
  fi
}
trap finish EXIT

touched=1
cp "$work/CHANGELOG.md" "$notes_file"
set_beta_version "$version"
git commit -q -m "chore(release): beta $version" -- "$notes_file" "$manifest" "$lock"
committed=1

{ printf 'Wormhole Beta %s\n\n' "$version"; changelog "$notes_file" notes "$version"; } > "$work/tag-message"
git tag -a "$tag" --cleanup=whitespace -F "$work/tag-message"

log "committed and tagged $tag"
git --no-pager log -1 --stat --format='%h %s' >&2
printf '\nThe 1.x app, untouched:\n' >&2
guarded >&2
log "next: push, then start the Beta workflow from main (a dry run first: it builds, signs and notarizes, and publishes nothing)"
echo "  git push origin main $tag"
echo "  gh workflow run beta.yml --ref main -f tag=$tag"
echo "  gh workflow run beta.yml --ref main -f tag=$tag -f publish=true"
log "or undo (before pushing): git tag -d $tag && git reset --hard HEAD~1"
