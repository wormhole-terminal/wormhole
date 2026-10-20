#!/bin/bash
# Profile the terminal engine (wormhole-vt) on a generated stream: builds the `profile_feed` example
# with the `profiling` profile (release code with line tables) and runs it under a sampling profiler.
#
#   scripts/profile-vt.sh [options] [-- profile_feed options]
#     --tool samply|xctrace   samply (default; opens the Firefox Profiler in the browser) or
#                             Instruments' Time Profiler (macOS; opens the .trace in Instruments)
#     --corpus NAME           the stream: log (default), ascii, utf8, sgr, osc, scroll, wide, links
#     --seconds N             how long the engine runs under the profiler (default 10)
#     --parser                profile the parser alone
#     --no-open               save the profile and print where, without opening it
#     --alloc                 no profiler: print the allocator calls per phase (parser alone, feed
#                             cold and warm, the reader's copy) and what is still held after each
#     --list                  the streams
#     -h, --help
#   Anything after `--` goes to profile_feed itself (--size KiB, --chunk KiB, --no-copy, ...).
#
# Needs samply (cargo install --locked samply; on macOS run `samply setup` once so it may attach)
# or, for --tool xctrace, Xcode's command line tools. Profiles go to <target>/profiling/profiles/.
# Read a profile bottom-up (the inverted call tree): the engine's time is in `feed`, the parser and
# the grid's write paths; allocator calls under them are a finding, the hot paths allocate nothing.
set -euo pipefail
cd "$(dirname "$0")/.." || exit 1
. scripts/lib.sh

tool=samply corpus=log seconds=10 alloc=0 open=1 parser=()
extra=()
while [ $# -gt 0 ]; do
  case "$1" in
    --tool) tool="${2:?--tool needs samply or xctrace}"; shift ;;
    --corpus) corpus="${2:?--corpus needs a name}"; shift ;;
    --seconds) seconds="${2:?--seconds needs a number}"; shift ;;
    --parser) parser=(--parser) ;;
    --no-open) open=0 ;;
    --alloc) alloc=1 ;;
    --list) exec cargo run -q --profile profiling -p wormhole-vt --example profile_feed -- --list ;;
    -h|--help) sed -n '2,/^set -euo/p' "$0" | sed '$d'; exit 0 ;;
    --) shift; extra=("$@"); break ;;
    *) die "unknown argument: $1 (see --help)" ;;
  esac
  shift
done
case "$tool" in samply|xctrace) ;; *) die "unknown tool: $tool (samply or xctrace)" ;; esac
case "$seconds" in ''|*[!0-9]*) die "--seconds needs a number" ;; esac

log "building profile_feed (profiling profile)"
cargo build -q --profile profiling -p wormhole-vt --example profile_feed
bin="$TARGET_DIR/profiling/examples/profile_feed"
[ -x "$bin" ] || die "$bin was not built"
args=(--corpus "$corpus" --seconds "$seconds" "${parser[@]+"${parser[@]}"}" "${extra[@]+"${extra[@]}"}")

if [ "$alloc" = 1 ]; then
  exec "$bin" --alloc "${args[@]}"
fi

out="$TARGET_DIR/profiling/profiles"
mkdir -p "$out"
stamp="$(date +%Y%m%d-%H%M%S)"
case "$tool" in
  samply)
    need samply "Install it with: cargo install --locked samply (then, on macOS, once: samply setup)"
    file="$out/vt-$corpus-$stamp.json.gz"
    flags=(record --output "$file")
    [ "$open" = 1 ] || flags+=(--save-only)
    log "samply: $corpus for ${seconds}s"
    samply "${flags[@]}" -- "$bin" "${args[@]}"
    [ "$open" = 1 ] || log "saved $file (open it with: samply load $file)"
    ;;
  xctrace)
    [ "$(uname)" = Darwin ] || die "xctrace is macOS only; use --tool samply"
    need xcrun "Install Xcode's command line tools: xcode-select --install"
    file="$out/vt-$corpus-$stamp.trace"
    log "Instruments (Time Profiler): $corpus for ${seconds}s"
    xcrun xctrace record --template 'Time Profiler' --output "$file" --launch -- "$bin" "${args[@]}"
    if [ "$open" = 1 ]; then open "$file"; else log "saved $file"; fi
    ;;
esac
