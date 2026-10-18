#!/bin/bash
# Prepares a release on main: CHANGELOG.md's [Unreleased] becomes the version, Cargo.toml and
# Cargo.lock get the version, one commit `chore(release): <version>`, and the annotated tag
# v<version> whose message is the notes from CHANGELOG.md. The tag is what starts the Release
# workflow (.github/workflows/release.yml); nothing is pushed unless you say so.
#
#   scripts/release.sh 0.3.1              commit and tag, then print the push command
#   scripts/release.sh 0.3.1 --push       and push main and the tag
#   scripts/release.sh 0.3.1 --dry-run    show what would change and what would stop it; change nothing
#   scripts/release.sh 0.3.1 --date 2026-10-12   the date in the changelog (default: today)
#
# Needs: a clean main that isn't behind origin/main, a version newer than Cargo.toml's, and
# something under [Unreleased]. A version with a suffix (0.4.0-beta.1) is a prerelease.
set -euo pipefail
. "$(dirname "$0")/lib.sh"
cd "$ROOT"

version="" push=0 dry=0 date=""
while [ $# -gt 0 ]; do
  case "$1" in
    --push) push=1 ;;
    --dry-run) dry=1 ;;
    --date) date="${2:?--date needs YYYY-MM-DD}"; shift ;;
    -h|--help) sed -n '2,13p' "$0"; exit 0 ;;
    -*) die "unknown argument: $1 (see --help)" ;;
    *) [ -z "$version" ] || die "one version only"; version="${1#v}" ;;
  esac
  shift
done
[ -n "$version" ] || die "usage: scripts/release.sh <version> [--push] [--dry-run] [--date YYYY-MM-DD]"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z]+(\.[0-9A-Za-z]+)*)?$ ]] \
  || die "'$version' is not a version (X.Y.Z, or X.Y.Z-beta.N for a prerelease)"
date="${date:-$(date +%F)}"
need python3

changelog=(python3 scripts/changelog.py)

# Everything that would stop a release; a dry run lists them and goes on, a real one stops.
problems=()
if [ "$(python3 -c '
import sys
sys.path.insert(0, "scripts")
import changelog
print(int(changelog.sem_key(sys.argv[1]) > changelog.sem_key(sys.argv[2])))' "$version" "$APP_VERSION")" != 1 ]; then
  problems+=("$version is not newer than $APP_VERSION, the version in Cargo.toml")
fi
branch="$(git rev-parse --abbrev-ref HEAD)"
[ "$branch" = main ] || problems+=("on '$branch', not main")
[ -z "$(git status --porcelain)" ] || problems+=("the working tree isn't clean (commit or stash first)")
if git fetch -q origin main 2>/dev/null; then
  behind="$(git rev-list --count HEAD..origin/main)"
  [ "$behind" = 0 ] || problems+=("main is $behind commit(s) behind origin/main (git pull --rebase)")
else
  problems+=("cannot reach origin to check main is up to date")
fi
if git rev-parse -q --verify "refs/tags/v$version" >/dev/null; then
  problems+=("the tag v$version exists already")
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
# The new changelog is made on a copy first: it fails here, with nothing touched, when [Unreleased]
# is empty or the file is malformed.
cp CHANGELOG.md "$work/CHANGELOG.md"
changelog_ok=1
if ! "${changelog[@]}" --file "$work/CHANGELOG.md" release "$version" --date "$date" 2>"$work/err"; then
  changelog_ok=0
  problems+=("CHANGELOG.md: $(sed 's/^error: //' "$work/err" | head -5)")
fi

if [ "$dry" = 1 ]; then
  log "dry run for $version ($date): nothing is changed"
  if [ ${#problems[@]} -gt 0 ]; then
    for p in "${problems[@]}"; do warn "would stop: $p"; done
  fi
  if [ "$changelog_ok" = 1 ]; then
    diff -u CHANGELOG.md "$work/CHANGELOG.md" | sed '1,2d' || true
    printf '\nCargo.toml: version = "%s" -> "%s"\nCargo.lock: the workspace crates follow (cargo update --workspace --offline)\n' "$APP_VERSION" "$version"
    printf '\ncommit: chore(release): %s\ntag:    v%s\n\n--- tag message ---\nWormhole %s\n\n' "$version" "$version" "$version"
    "${changelog[@]}" --file "$work/CHANGELOG.md" notes "$version"
    sev="$("${changelog[@]}" --file "$work/CHANGELOG.md" severity "$version")"
    [ -z "$sev" ] || printf '\nsecurity update, highest severity: %s\n' "$sev"
  fi
  exit 0
fi

if [ ${#problems[@]} -gt 0 ]; then
  for p in "${problems[@]}"; do warn "$p"; done
  die "not releasing $version"
fi

# A failure after the first file is written leaves a half-made release in the working tree; say how to undo it.
touched=0 committed=0
finish() {
  local rc=$?
  rm -rf "$work"
  if [ $rc -ne 0 ] && [ $touched = 1 ] && [ $committed = 0 ]; then
    warn "release failed; put the files back with: git restore CHANGELOG.md Cargo.toml Cargo.lock"
  elif [ $rc -ne 0 ] && [ $committed = 1 ]; then
    warn "committed, but not tagged; tag it by hand or undo with: git reset --hard HEAD~1"
  fi
}
trap finish EXIT

touched=1
cp "$work/CHANGELOG.md" CHANGELOG.md
set_app_version "$version"
git commit -q -m "chore(release): $version" -- CHANGELOG.md Cargo.toml Cargo.lock
committed=1

{ printf 'Wormhole %s\n\n' "$version"; "${changelog[@]}" notes "$version"; } > "$work/tag-message"
git tag -a "v$version" --cleanup=whitespace -F "$work/tag-message"

log "committed and tagged v$version"
git --no-pager log -1 --stat --format='%h %s' >&2
if [ "$push" = 1 ]; then
  git push origin main "v$version"
  log "pushed: the Release workflow builds v$version"
else
  log "next, to publish (the Release workflow starts from the tag):"
  echo "  git push origin main v$version"
  log "or undo: git tag -d v$version && git reset --hard HEAD~1"
fi
