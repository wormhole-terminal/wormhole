#!/bin/bash
# vttest (Thomas Dickey's VT100/VT220/xterm test program) as a gate: it runs headless against
# wormhole-vt (no window, no app) and every screen it draws is compared with the committed one in
# tools/vt-compare/suites/vttest/. A screen that differs fails the step and is printed as a diff.
# The committed screens are the engine's current behaviour, reviewed against what vttest says each
# should show (the review.md in each menu's folder says which pass, fail or are unsupported); the
# gate keeps that behaviour from changing by accident, in either direction.
#
#   scripts/vttest-gate.sh            # build vttest-host, run vttest (about 10 s), compare
#   scripts/vttest-gate.sh --update   # run it and rewrite the screens from this run; review the diff,
#                                     # update the review.md of every menu that changed, and commit
#                                     # (do this when the engine's behaviour changes on purpose, or
#                                     # after moving the pin in tools/vt-compare/suites.pin)
#   scripts/vttest-gate.sh --plan     # say what would run
#
# vttest is fetched at its pinned version into a cache outside the repository and built there
# (tools/vt-compare/fetch-suites.sh; it needs curl, make and a C compiler). Without it and without
# a network the step is skipped with a warning, never failed. scripts/suites-gate.sh runs this
# after the other outside suites, so scripts/check.sh runs it when the engine, the bench crate or
# the terminal layer changed.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
. scripts/lib.sh

update=0 plan=0
for arg in "$@"; do
  case "$arg" in
    --update) update=1 ;;
    --plan) plan=1 ;;
    -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
    *) die "unknown argument: $arg (see --help)" ;;
  esac
done
[ "$plan" = 1 ] && { echo "build vttest-host (release), walk vttest's menus headless, compare every screen with tools/vt-compare/suites/vttest/"; exit 0; }

skip() {
  [ "${SUITES_REQUIRED:-0}" = 1 ] && die "vttest cannot run: $*"   # CI: see scripts/suites-gate.sh
  warn "vttest skipped: $* The engine's conformance against vttest is unchecked (scripts/vttest-gate.sh)."; exit 0
}

command -v python3 >/dev/null || skip "python3 is not installed."
fetch_log="$(mktemp "${TMPDIR:-/tmp}/vttest-fetch.XXXXXX")"
env_out="$(tools/vt-compare/fetch-suites.sh --env 2>"$fetch_log")" || {
  reason="$(tail -n 2 "$fetch_log" | tr '\n' ' ')"; rm -f "$fetch_log"
  skip "the suites are not cached and could not be fetched (${reason:-no output})."
}
rm -f "$fetch_log"
VTTEST_BIN="$(sed -n 's/^VTTEST_BIN=//p' <<<"$env_out")"
[ -x "$VTTEST_BIN" ] || skip "vttest could not be built (see tools/vt-compare/fetch-suites.sh)."

start=$SECONDS
TARGET="${CARGO_TARGET_DIR:-$ROOT/target}"
GOLDEN=tools/vt-compare/suites/vttest
log "building vttest-host (release)"
cargo build --quiet --release -p wormhole-vt-bench --bin vttest-host || die "vttest-host did not build"

work="$(mktemp -d "${TMPDIR:-/tmp}/vttest-gate.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# One walk of every menu into the folder given.
walk() { "$TARGET/release/vttest-host" --vttest "$VTTEST_BIN" --out "$1" >"$1.out" 2>&1; }

log "vttest, headless"
walk "$work/run1" || { tail -n 15 "$work/run1.out" >&2; die "vttest-host failed"; }

if [ "$update" = 1 ]; then
  for dir in "$work"/run1/*/; do
    name="$(basename "$dir")"
    mkdir -p "$GOLDEN/$name"
    # Only the screens: a menu's review.md stays for a person to update.
    find "$GOLDEN/$name" -maxdepth 1 -name '*.txt' -delete
    cp "$dir"*.txt "$GOLDEN/$name/"
  done
  cp "$work/run1/notes.txt" "$GOLDEN/notes.txt"
  log "vttest: screens rewritten in $GOLDEN ($(find "$GOLDEN" -name '[0-9]*.txt' | wc -l | tr -d ' ') of them); review the diff and each menu's review.md"
  git status --short -- "$GOLDEN" | sed 's/^/    /' | head -20
  exit 0
fi

same() { diff -r -x review.md -x README.md "$GOLDEN" "$1" >"$1.diff" 2>&1; }

# A screen vttest drew while the machine was too busy to answer in time (it waits a fraction of a
# second for each reply) would differ, so a difference is walked once more before it counts: a real
# change shows up twice, a hiccup does not.
result="$work/run1"
if ! same "$result"; then
  log "vttest: a difference, walking again to see whether it holds"
  walk "$work/run2" || { tail -n 15 "$work/run2.out" >&2; die "vttest-host failed"; }
  result="$work/run2"
  if same "$result"; then
    warn "vttest: the first walk differed and the second did not (a busy machine); the engine matches the committed screens"
  fi
fi
if [ -s "$result.diff" ]; then
  head -n 120 "$result.diff" >&2
  log "vttest: $(grep -c '^diff \|^Only in' "$result.diff") file(s) differ (the first lines are above)"
  printf '\033[1;31mFAILED\033[0m vttest screens differ from tools/vt-compare/suites/vttest/ (scripts/vttest-gate.sh --update if on purpose)\n' >&2
  exit 1
fi
log "vttest: $(find "$GOLDEN" -name '[0-9]*.txt' | wc -l | tr -d ' ') screens as committed, $((SECONDS - start)) s"
