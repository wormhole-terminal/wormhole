#!/bin/bash
# Coverage-guided fuzzing of wormhole-vt (cargo fuzz, libFuzzer; nightly Rust). By hand: it is not in
# scripts/check.sh and not in CI. The targets and what they check are in crates/wormhole-vt/fuzz/README.md.
#
#   scripts/fuzz.sh seeds                     # (re)make the starting corpus from the .vtt cases and corpora
#   scripts/fuzz.sh run [target...] [-m MIN] [-j JOBS]
#                                             # fuzz (default: every target, 10 minutes, 4 processes in all)
#   scripts/fuzz.sh repro <target> <file>     # run one saved input (fuzz/artifacts/<target>/crash-*)
#   scripts/fuzz.sh min                       # shrink the corpus (cargo fuzz cmin) before keeping it
#   scripts/fuzz.sh list                      # the targets
#
# The processes are split evenly over the targets (at least one each), never more than 4 in all, and
# run at a low priority. A crash is saved to crates/wormhole-vt/fuzz/artifacts/<target>/; turn it into a
# regression test in the engine's tests, then fix the engine.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
. scripts/lib.sh

FUZZ=crates/wormhole-vt/fuzz
TARGETS=(parser split term graphics)
# Bound the work an input may cause: a hang or a huge allocation is a finding.
LIBFUZZER=(-max_len=3000 -timeout=10 -rss_limit_mb=2048 -dict="$ROOT/$FUZZ/escape.dict")
# Nightly builds with sanitizers get their own target dir, so they never invalidate the normal one.
BASE_TARGET="${CARGO_TARGET_DIR:-$ROOT/target}"
export CARGO_TARGET_DIR="$BASE_TARGET/fuzz-nightly"

need cargo-fuzz "Install it with: cargo install cargo-fuzz"
rustup toolchain list | grep -q '^nightly' || die "no nightly toolchain. Install it with: rustup toolchain install nightly --profile minimal"

cmd="${1:-}"; [ $# -gt 0 ] && shift
case "$cmd" in
  seeds)
    CARGO_TARGET_DIR="$BASE_TARGET" cargo run -q -p wormhole-vt-bench --bin fuzz-seeds -- "$ROOT/$FUZZ/corpus"
    ;;
  list) printf '%s\n' "${TARGETS[@]}" ;;
  run)
    minutes=10 jobs=4 targets=()
    while [ $# -gt 0 ]; do
      case "$1" in
        -m) minutes="${2:?-m needs minutes}"; shift ;;
        -j) jobs="${2:?-j needs a number}"; shift ;;
        *) printf '%s\n' "${TARGETS[@]}" | grep -qx "$1" || die "unknown target: $1 (scripts/fuzz.sh list)"; targets+=("$1") ;;
      esac
      shift
    done
    [ ${#targets[@]} -gt 0 ] || targets=("${TARGETS[@]}")
    [ "$jobs" -le 4 ] || die "at most 4 processes (-j 4)"
    each=$(( jobs / ${#targets[@]} )); [ "$each" -ge 1 ] || each=1
    [ -d "$ROOT/$FUZZ/corpus" ] || scripts/fuzz.sh seeds || exit 1
    (cd "$ROOT/crates/wormhole-vt" && cargo +nightly fuzz build) || exit 1
    # One process per target runs plain; more run as libFuzzer jobs (which write fuzz-N.log in the
    # folder they start in, so that is only used when a single target gets them all).
    pids=()
    for t in "${targets[@]}"; do
      workers=()
      [ "$each" -gt 1 ] && workers=(-jobs="$each" -workers="$each")
      (cd "$ROOT/crates/wormhole-vt" && exec nice -n 10 cargo +nightly fuzz run "$t" -- "${LIBFUZZER[@]}" \
        -max_total_time=$(( minutes * 60 )) "${workers[@]+"${workers[@]}"}" >"fuzz/fuzz-$t.log" 2>&1) &
      pids+=($!)
    done
    status=0
    for p in "${pids[@]}"; do wait "$p" || status=1; done
    crashes=$(find "$ROOT/$FUZZ/artifacts" -type f 2>/dev/null | sort)
    if [ -n "$crashes" ]; then
      warn "findings:"; echo "$crashes" >&2; status=1
    else
      log "no findings in $minutes min on ${targets[*]} ($each process(es) each)"
    fi
    exit $status
    ;;
  repro)
    t="${1:?repro <target> <file>}"; f="${2:?repro <target> <file>}"
    (cd "$ROOT/crates/wormhole-vt" && cargo +nightly fuzz run "$t" "$(cd "$(dirname "$f")" && pwd)/$(basename "$f")")
    ;;
  min)
    for t in "${TARGETS[@]}"; do
      (cd "$ROOT/crates/wormhole-vt" && cargo +nightly fuzz cmin "$t" -- "${LIBFUZZER[@]}") || exit 1
    done
    du -sh "$ROOT/$FUZZ/corpus"/* >&2
    ;;
  *) sed -n '2,15p' "$0"; exit 1 ;;
esac
