#!/bin/bash
# Mutation testing of the terminal engine (cargo-mutants, decision D-22): change one operator, return
# value or branch of crates/wormhole-vt at a time and run the tests; a mutant no test fails ("missed")
# is code whose behaviour nothing pins down.
#
#   scripts/mutants.sh --list                    # how many mutants each area has
#   scripts/mutants.sh --area parser             # one area (names below)
#   scripts/mutants.sh --area grid --jobs 6      # more or fewer parallel builds (default: half the cores)
#   scripts/mutants.sh --area repaint            # also runs the bisimulation tests (see below)
#   scripts/mutants.sh --file '**/grid/scroll.rs' # one file, a glob as cargo-mutants reads it
#   scripts/mutants.sh --area osc --out DIR      # where mutants.out/ goes (default <target>/mutants/<area>)
#   scripts/mutants.sh --area osc --iterate      # any other argument goes to cargo-mutants (see its --help)
#
# Areas: parser unicode grid dispatch text screen osc reports graphics repaint input passthrough rest
# (rest = everything else: types/, terminal/*.rs, terminal/modes, terminal/event ...). No --area runs
# them all, in that order.
#
# The tests that judge a mutant are the engine's own plus the bench crate's .vtt conformance runner
# and differential tests (crates/wormhole-vt/mutants.toml, which also lists what is left out and why).
# The three bisimulation tests (32 s in a debug build, every mutant would pay for them) run only for
# --area repaint, whose writer they exist to check, or with --bisim.
#
# The results are <out>/mutants.out/{missed,caught,timeout,unviable}.txt. "timeout" counts as caught.
# Everything, including the copies of the tree cargo-mutants builds in, lives under one folder in the
# target directory (CARGO_TARGET_DIR if set, else <repo>/target), which is git-ignored; delete
# <target>/mutants when done. A full run takes hours.
#
# Needs: cargo install --locked cargo-mutants
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/lib.sh

area="" out="" jobs="" bisim=0 list=0 files=()
extra=()
while [ $# -gt 0 ]; do
  case "$1" in
    --area) area="${2:?--area needs a name}"; shift ;;
    --out) out="${2:?--out needs a folder}"; shift ;;
    --jobs) jobs="${2:?--jobs needs a number}"; shift ;;
    --file) files+=("${2:?--file needs a glob}"); shift ;;
    --bisim) bisim=1 ;;
    --list) list=1 ;;
    -h|--help) sed -n '2,26p' "$0"; exit 0 ;;
    *) extra+=("$1") ;;
  esac
  shift
done
need cargo-mutants "Install it with: cargo install --locked cargo-mutants"

# An area is a set of file globs; "rest" is what none of the others covers.
glob_for() {
  case "$1" in
    parser) echo '**/src/parser/**' ;;
    unicode) echo '**/src/unicode/**' ;;
    grid) echo '**/src/grid/**' ;;
    dispatch) echo '**/src/terminal/dispatch/**' ;;
    text) echo '**/src/terminal/text/**' ;;
    screen) echo '**/src/terminal/screen/**' ;;
    osc) echo '**/src/terminal/osc/**' ;;
    reports) echo '**/src/terminal/reports/**' ;;
    graphics) echo '**/src/terminal/graphics/**' ;;
    repaint) echo '**/src/terminal/repaint/**' ;;
    input) echo '**/src/input/**' ;;
    passthrough) echo '**/src/passthrough.rs' ;;
  esac
}
ALL_AREAS=(parser unicode grid dispatch text screen osc reports graphics repaint input passthrough rest)

base="${CARGO_TARGET_DIR:-$ROOT/target}/mutants"
[ -n "$jobs" ] || jobs=$(( $(sysctl -n hw.ncpu 2>/dev/null || nproc) / 2 ))
[ "$jobs" -ge 1 ] || jobs=1

run_one() {
  local name="$1" dir args=() tests=() g k
  dir="${out:-$base/$name}"
  if [ "$name" = rest ]; then
    for g in "${ALL_AREAS[@]}"; do [ "$g" = rest ] || args+=(-e "$(glob_for "$g")"); done
  elif [ "$name" != all ]; then
    [ -n "$(glob_for "$name")" ] || die "unknown area: $name (${ALL_AREAS[*]})"
    args+=(-f "$(glob_for "$name")")
  fi
  for g in ${files[@]+"${files[@]}"}; do args+=(-f "$g"); done
  if [ "$list" = 1 ]; then
    printf '%-12s' "$name"
    cargo mutants -p wormhole-vt --config "$ROOT/crates/wormhole-vt/mutants.toml" --no-times --list \
      ${args[@]+"${args[@]}"} ${extra[@]+"${extra[@]}"} | wc -l | tr -d ' '
    return
  fi
  if [ "$bisim" = 0 ] && [ "$name" != repaint ]; then
    tests=(-- -- --skip every_vtt_case_at_every_byte --skip every_corpus_at_many_cuts
      --skip every_corpus_to_the_end_of_the_stream)
  fi
  mkdir -p "$dir/tmp"
  log "mutants: $name ($jobs jobs) -> $dir/mutants.out"
  # Each job builds in its own copy of the tree (so the builds don't fight over one target directory):
  # the copies go under TMPDIR, i.e. inside <out>, and the shared CARGO_TARGET_DIR must not override them.
  env -u CARGO_TARGET_DIR TMPDIR="$dir/tmp" CARGO_PROFILE_DEV_DEBUG=0 \
    cargo mutants -p wormhole-vt --config "$ROOT/crates/wormhole-vt/mutants.toml" -j "$jobs" -o "$dir" \
    --no-times ${args[@]+"${args[@]}"} ${extra[@]+"${extra[@]}"} ${tests[@]+"${tests[@]}"} || true
  [ -d "$dir/mutants.out" ] || return 0
  printf '%s: ' "$name"
  for k in caught missed timeout unviable; do
    printf '%s %s  ' "$k" "$(wc -l < "$dir/mutants.out/$k.txt" 2>/dev/null | tr -d ' ' || echo 0)"
  done
  echo
}

if [ -n "$area" ]; then
  run_one "$area"
elif [ ${#files[@]} -gt 0 ]; then
  run_one all
else
  for a in "${ALL_AREAS[@]}"; do run_one "$a"; done
fi
