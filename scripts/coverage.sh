#!/bin/bash
# How much of wormhole-vt its tests run: line and branch coverage per module (cargo-llvm-cov).
#
#   scripts/coverage.sh              # the engine's tests, its .vtt conformance runner and the bench crate's tests
#   scripts/coverage.sh --suites     # also esctest2 and ucs-detect run headless; shows what they add
#   scripts/coverage.sh --html       # also write an HTML report and print where it is
#   scripts/coverage.sh --files 25   # the 25 files with the most lines not run (default 12)
#   scripts/coverage.sh --missing    # also list the lines not run, file by file
#
# Needs: cargo install cargo-llvm-cov, and a nightly toolchain with llvm-tools (branch coverage is
# nightly-only): rustup toolchain install nightly --profile minimal -c llvm-tools-preview
# It builds into its own target folder (<target>/llvm-cov-target), so the normal builds stay as they are.
# Test code that sits in the engine's own files counts as run, so the numbers are a little high;
# files under tests/ folders and the invariant checkers (check.rs: only their error
# branches are unrun) are left out. The Unicode table builders (unicode/grapheme.rs) run when the crate
# is compiled, so they show as unrun. The numbers per module are in ../repo-docs/docs/engine.md ("Coverage").
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
. scripts/lib.sh

suites=0 html=0 files=12 missing=0
while [ $# -gt 0 ]; do
  case "$1" in
    --suites) suites=1 ;;
    --html) html=1 ;;
    --missing) missing=1 ;;
    --files) files="${2:?--files needs a number}"; shift ;;
    -h|--help) sed -n '2,17p' "$0"; exit 0 ;;
    *) die "unknown argument: $1 (see --help)" ;;
  esac
  shift
done
need cargo-llvm-cov "Install it with: cargo install cargo-llvm-cov"
rustup toolchain list | grep -q '^nightly' || die "no nightly toolchain: rustup toolchain install nightly --profile minimal -c llvm-tools-preview"

# What counts: the engine's sources; the tests' own folders and everything else are left out.
IGNORE='(/tests/|/tests\.rs|/check\.rs|wormhole-vt-bench|/\.cargo/|/rustc/|crates/wormhole-core|crates/wormhole-term|crates/wormhole/)'
PACKAGES=(-p wormhole-vt -p wormhole-vt-bench)
work="$(mktemp -d "${TMPDIR:-/tmp}/coverage.XXXXXX")"
trap 'rm -rf "$work"' EXIT

log "building and running the tests with coverage"
cargo +nightly llvm-cov clean --workspace || exit 1
env_out="$(cargo +nightly llvm-cov show-env --export-prefix --branch 2>/dev/null)" || die "cargo llvm-cov show-env failed"
# shellcheck disable=SC1090
eval "$env_out"
cargo +nightly test --quiet "${PACKAGES[@]}" >"$work/tests.log" 2>&1 || { tail -n 30 "$work/tests.log" >&2; die "the tests failed"; }
cargo +nightly llvm-cov report --json --output-path "$work/tests.json" --ignore-filename-regex "$IGNORE" >/dev/null 2>&1 || die "no report"

args=("$work/tests.json")
if [ "$suites" = 1 ]; then
  log "esctest2 and ucs-detect on the instrumented hosts"
  SUITES_DIR="$(tools/vt-compare/fetch-suites.sh --path)"
  SUITES_PYTHON="$(tools/vt-compare/fetch-suites.sh --env 2>/dev/null | sed -n 's/^SUITES_PYTHON=//p')"
  [ -x "$SUITES_PYTHON" ] || die "the suites are not cached (tools/vt-compare/fetch-suites.sh)"
  cargo +nightly build --quiet -p wormhole-vt-bench --bin esctest-host --bin ucs-host || die "the hosts did not build"
  bin="$CARGO_LLVM_COV_TARGET_DIR/debug"
  "$bin/esctest-host" --suites "$SUITES_DIR" --out "$work/esctest.jsonl" >"$work/esctest.out" 2>&1 &
  esctest_pid=$!
  "$SUITES_PYTHON" tools/vt-compare/ucs-headless.py --host "$bin/ucs-host" --out "$work/ucs.yaml" >"$work/ucs.out" 2>&1
  wait "$esctest_pid"
  cargo +nightly llvm-cov report --json --output-path "$work/all.json" --ignore-filename-regex "$IGNORE" >/dev/null 2>&1 || die "no report"
  args+=("$work/all.json")
fi

python3 scripts/coverage_table.py --files "$files" "${args[@]}"
if [ "$missing" = 1 ]; then
  echo; echo "lines not run:"
  cargo +nightly llvm-cov report --show-missing-lines --ignore-filename-regex "$IGNORE" 2>/dev/null |
    sed -n 's#^/.*/crates/wormhole-vt/src/\([^:]*\): \(.*\)$#  \1: \2#p'
fi
if [ "$html" = 1 ]; then
  out="${CARGO_TARGET_DIR:-$ROOT/target}/llvm-cov-html"
  cargo +nightly llvm-cov report --html --output-dir "$out" --ignore-filename-regex "$IGNORE" >/dev/null 2>&1 && log "HTML report: $out/html/index.html"
fi
