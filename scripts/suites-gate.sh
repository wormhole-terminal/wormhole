#!/bin/bash
# The outside conformance suites as a gate: esctest2 and ucs-detect run headless against wormhole-vt
# (no window, no app) and are compared, test by test, with the committed expectation in
# tools/vt-compare/expected/. Every test that passes there must still pass; a regression is
# reported by name and fails the step. A known failure that starts passing is reported and the
# expectation is left to be updated by hand.
#
#   scripts/suites-gate.sh            # build the two hosts, run both suites, compare (about a minute)
#   scripts/suites-gate.sh --update   # run them and rewrite the expectation from this run; review the
#                                     # diff and commit it (do this when a failure is fixed, or after
#                                     # moving the pins in tools/vt-compare/suites.pin)
#   scripts/suites-gate.sh --plan     # say what would run
#
# The suites are fetched at their pinned commits into a cache outside the repository
# (tools/vt-compare/fetch-suites.sh). With no cache and no network (cloud sessions, a plane) the
# step is skipped with a warning, never failed. scripts/check.sh runs it when the engine
# (crates/wormhole-vt), the bench crate or core's terminal layer changed, and with --all.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
. scripts/lib.sh

update=0 plan=0
for arg in "$@"; do
  case "$arg" in
    --update) update=1 ;;
    --plan) plan=1 ;;
    -h|--help) sed -n '2,16p' "$0"; exit 0 ;;
    *) die "unknown argument: $arg (see --help)" ;;
  esac
done
[ "$plan" = 1 ] && { echo "build esctest-host and ucs-host (release), run esctest2 and ucs-detect headless, compare with tools/vt-compare/expected/"; exit 0; }

skip() { warn "outside suites skipped: $* The engine's conformance against esctest2 and ucs-detect is unchecked (scripts/suites-gate.sh)."; exit 0; }

command -v python3 >/dev/null || skip "python3 is not installed."
command -v git >/dev/null || skip "git is not installed."
# The cache is there, or fetched now (a shallow clone of two repositories and a venv; needs the network).
fetch_log="$(mktemp "${TMPDIR:-/tmp}/suites-fetch.XXXXXX")"
env_out="$(tools/vt-compare/fetch-suites.sh --env 2>"$fetch_log")" || {
  reason="$(tail -n 2 "$fetch_log" | tr '\n' ' ')"; rm -f "$fetch_log"
  skip "the suites are not cached and could not be fetched (${reason:-no output})."
}
rm -f "$fetch_log"
SUITES_PYTHON="$(sed -n 's/^SUITES_PYTHON=//p' <<<"$env_out")"
SUITES_DIR="$(tools/vt-compare/fetch-suites.sh --path)"
[ -x "$SUITES_PYTHON" ] || skip "the suites' Python is missing ($SUITES_PYTHON)."

start=$SECONDS
TARGET="${CARGO_TARGET_DIR:-$ROOT/target}"
log "building esctest-host and ucs-host (release)"
cargo build --quiet --release -p wormhole-vt-bench --bin esctest-host --bin ucs-host || die "the hosts did not build"

work="$(mktemp -d "${TMPDIR:-/tmp}/suites-gate.XXXXXX")"
trap 'rm -rf "$work"' EXIT

log "esctest2 and ucs-detect, headless"
"$TARGET/release/esctest-host" --suites "$SUITES_DIR" --out "$work/esctest.jsonl" >"$work/esctest.out" 2>&1 &
esctest_pid=$!
CARGO_TARGET_DIR="$TARGET" python3 tools/vt-compare/ucs-headless.py --out "$work/ucs.yaml" >"$work/ucs.out" 2>&1
ucs_status=$?
wait "$esctest_pid"; esctest_status=$?

flag=""; [ "$update" = 1 ] && flag="--update"
failed=0
if [ "$esctest_status" != 0 ] || [ ! -s "$work/esctest.jsonl" ]; then
  tail -n 15 "$work/esctest.out" >&2; echo "esctest-host failed (status $esctest_status)" >&2; failed=1
else
  python3 tools/vt-compare/gate.py esctest "$work/esctest.jsonl" $flag || failed=1
fi
if [ "$ucs_status" != 0 ] || [ ! -s "$work/ucs.yaml" ]; then
  tail -n 15 "$work/ucs.out" >&2; echo "ucs-headless failed (status $ucs_status)" >&2; failed=1
else
  "$SUITES_PYTHON" tools/vt-compare/gate.py ucs "$work/ucs.yaml" $flag || failed=1
fi
log "outside suites: $((SECONDS - start)) s"
[ "$failed" = 0 ] || printf '\033[1;31mFAILED\033[0m the outside suites (scripts/suites-gate.sh)\n' >&2
exit "$failed"
