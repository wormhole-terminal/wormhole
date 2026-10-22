#!/bin/bash
# Coverage-guided fuzzing of wormhole-wire (cargo fuzz, libFuzzer; nightly Rust). By hand: it is not in
# scripts/check.sh and not in CI. The targets and what they check are in crates/wormhole-wire/fuzz/README.md;
# the terminal engine's fuzzing is in its own repository (wormhole-terminal/wormhole-vt).
#
#   scripts/fuzz.sh seeds                     # (re)make the starting corpus (the wire samples)
#   scripts/fuzz.sh run [target...] [-m MIN] [-j JOBS]
#                                             # fuzz (default: every target, 10 minutes, 4 processes at a time)
#   scripts/fuzz.sh repro <target> <file>     # run one saved input (fuzz/artifacts/<target>/crash-*)
#   scripts/fuzz.sh min                       # shrink the corpus (cargo fuzz cmin) before keeping it
#   scripts/fuzz.sh list                      # the targets
#
# The processes are split evenly over the targets (at least one each), never more than 4 at a time, and
# run at a low priority; more targets than processes run in batches, MIN minutes each. A crash is saved to
# the crate's fuzz/artifacts/<target>/; turn it into a regression test in that crate's tests, then fix it.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
. scripts/lib.sh

TARGETS=(wire_frame wire_message)
# Every target is the wire protocol's; the helpers keep the crate a parameter all the same.
crate_of() { echo wormhole-wire; }
# Bound the work an input may cause: a hang or a huge allocation is a finding. A frame is up to 64 KiB.
libfuzzer_args() { echo "-max_len=70000 -timeout=10 -rss_limit_mb=2048 -dict=$ROOT/crates/wormhole-wire/fuzz/wire.dict"; }
# Nightly builds with sanitizers get their own target dir, so they never invalidate the normal one.
BASE_TARGET="${CARGO_TARGET_DIR:-$ROOT/target}"
export CARGO_TARGET_DIR="$BASE_TARGET/fuzz-nightly"

need cargo-fuzz "Install it with: cargo install cargo-fuzz"
rustup toolchain list | grep -q '^nightly' || die "no nightly toolchain. Install it with: rustup toolchain install nightly --profile minimal"

cmd="${1:-}"; [ $# -gt 0 ] && shift
case "$cmd" in
  seeds)
    CARGO_TARGET_DIR="$BASE_TARGET" cargo run -q -p wormhole-wire --example seeds -- "$ROOT/crates/wormhole-wire/fuzz/corpus"
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
    # Build and seed each crate the chosen targets touch, once.
    crates=$(for t in "${targets[@]}"; do crate_of "$t"; done | sort -u)
    for c in $crates; do
      [ -d "$ROOT/crates/$c/fuzz/corpus" ] || scripts/fuzz.sh seeds || exit 1
      (cd "$ROOT/crates/$c" && cargo +nightly fuzz build) || exit 1
    done
    status=0
    # Batches of at most $jobs targets; in each, one process per target runs plain, and a target
    # that gets several runs them as libFuzzer jobs (which write fuzz-N.log in the folder they start
    # in, so that is only used when a single target gets them all).
    for ((i = 0; i < ${#targets[@]}; i += jobs)); do
      batch=("${targets[@]:i:jobs}")
      each=$(( jobs / ${#batch[@]} )); [ "$each" -ge 1 ] || each=1
      pids=()
      for t in "${batch[@]}"; do
        workers=()
        [ "$each" -gt 1 ] && workers=(-jobs="$each" -workers="$each")
        # shellcheck disable=SC2046  # libfuzzer_args is a list of words on purpose
        (cd "$ROOT/crates/$(crate_of "$t")" && exec nice -n 10 cargo +nightly fuzz run "$t" -- $(libfuzzer_args "$t") \
          -max_total_time=$(( minutes * 60 )) "${workers[@]+"${workers[@]}"}" >"fuzz/fuzz-$t.log" 2>&1) &
        pids+=($!)
      done
      for p in "${pids[@]}"; do wait "$p" || status=1; done
    done
    crashes=$(for c in $crates; do find "$ROOT/crates/$c/fuzz/artifacts" -type f 2>/dev/null; done | sort)
    if [ -n "$crashes" ]; then
      warn "findings:"; echo "$crashes" >&2; status=1
    else
      log "no findings in $minutes min on ${targets[*]} (up to $jobs process(es) at a time)"
    fi
    exit $status
    ;;
  repro)
    t="${1:?repro <target> <file>}"; f="${2:?repro <target> <file>}"
    (cd "$ROOT/crates/$(crate_of "$t")" && cargo +nightly fuzz run "$t" "$(cd "$(dirname "$f")" && pwd)/$(basename "$f")")
    ;;
  min)
    for t in "${TARGETS[@]}"; do
      # shellcheck disable=SC2046
      (cd "$ROOT/crates/$(crate_of "$t")" && cargo +nightly fuzz cmin "$t" -- $(libfuzzer_args "$t")) || exit 1
    done
    du -sh "$ROOT"/crates/wormhole-*/fuzz/corpus/* >&2
    ;;
  *) sed -n '2,15p' "$0"; exit 1 ;;
esac
