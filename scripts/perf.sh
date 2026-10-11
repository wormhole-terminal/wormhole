#!/usr/bin/env bash
# Performance check: measure how fast a release build of Wormhole parses and draws, and fail when
# that regressed against the baseline committed for this machine (tools/bench/baselines/<id>.json),
# so a later change can't silently undo the speedups of 11ed409 and 215284b.
#
#   scripts/perf.sh [options]
#     --only <a,b>      run only these: warptunnel, matrix, unicode, dashboard (ttystorm animations),
#                       flood-ascii, flood-truecolor (ttystorm flood), throughput (ttystorm bench)
#     --save-baseline   write this run as the baseline of this machine (tools/bench/baselines/)
#     --bin <path>      measure this prebuilt devctl binary instead of building HEAD
#     --fps <n>         the animations' frame rate (default 0, see below)
#     --keep            keep the scratch dir (instances' data, samples, frames.txt) and print it
#     -h, --help
#   Results: a table, and $TMPDIR/wormhole-perf/<time>.json with the commit, machine and every number.
#   Exit 1 when a number is worse than the baseline by more than the tolerances below; 0 otherwise,
#   also without a baseline for this machine (it says how to save one).
#
# What is measured, per workload, in its own isolated instance (scripts/devctl-instance.sh, the
# window resized to 2400x1300, a wrapper script as the "shell" that runs ttystorm):
#   A  draw time per frame: devctl `frames 60` (60 forced draws of the workspace, avg and max in ms),
#      run 3 times; the medians are reported. It forces the draws, so it does not depend on the window
#      being visible. The animations run with --fps 0: new output is then always pending, so every
#      forced draw re-renders the terminal grid, the expensive case. With --fps 60 about a third of
#      the draws find nothing new, so the average depends on the phase of the animation against the
#      60 forced draws. Measured on an M5 Pro, 4 runs of every animation at --fps 0 and 3 at --fps 60:
#      the draw average varied 9% (--fps 0) against 7% (--fps 60) between runs, so no real difference
#      there, but --fps 0 has the tighter max (2.2-3.0 ms against 2.4-4.3 ms), a steadier busy share
#      (sd 3.3 against 4.0 points) and numbers about 30% higher, which leaves more signal above the
#      0.5 ms margin below. A flood is one more way to keep new output pending: they run the same.
#   B  main-thread busy share: `sample <pid> 5`, the thread named com.apple.main-thread; busy are the
#      samples whose innermost frame is not an idle wait (mach_msg2_trap, semaphore_timedwait_trap,
#      __psynch_cvwait, __workq_kernreturn, kevent, read, ...; perf_tools.py). This depends on how
#      much the app is asked to draw: with the display asleep or the window covered, macOS throttles
#      the window's frame callbacks, so the number can be lower than with it on screen (not measured
#      here). The result file records whether the display slept, and the table warns when it differs
#      from the baseline's.
#   C  throughput: `ttystorm bench --only logflood,colorflood,cursorstorm --size 16 --runs 3` inside an
#      instance, the medians in MB/s (terminal size 2400x1300 points, see the cols x rows printed).
# The frame rate on screen (devctl `paints`) is NOT a metric: it is exactly what macOS throttles for a
# window that isn't visible, so it would measure the display, not Wormhole.
#
# Noise: these are wall-clock numbers on a laptop, so the tolerances are loose on purpose. A number
# fails when it is worse than the baseline by more than the factor AND the margin: the factor covers
# slow drift (thermals, another machine of the same model), the margin keeps tiny baselines (1.4 ms
# draws) from failing on a 0.3 ms wobble. The old code was 10 to 20 times slower, far outside both.
# Run it with nothing else busy; on a battery or hot machine, run it again before believing a fail.
#
# Everything lives in a scratch dir under $TMPDIR (never the repository, never your data dir); the
# script stops only the instances it started, by PID, also when it fails or is interrupted. The release
# build goes to $CARGO_TARGET_DIR/perf-build (target/perf-build), not into target/release.
set -euo pipefail
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"

DRAW_FACTOR=1.3        # draw avg may be this many times the baseline ...
DRAW_MARGIN_MS=0.5     # ... plus this
BUSY_FACTOR=1.3        # main-thread busy % likewise ...
BUSY_MARGIN_PTS=5      # ... plus this many percentage points
THROUGHPUT_FLOOR=0.75  # MB/s may fall to this share of the baseline
WINDOW_W=2400 WINDOW_H=1300
FRAME_RUNS=3 FRAMES=60 SAMPLE_SECONDS=5
SETTLE_SECONDS=4       # the workload runs this long before it is measured (caches, first frames)
ANIMATIONS="warptunnel matrix unicode dashboard"
ALL_WORKLOADS="$ANIMATIONS flood-ascii flood-truecolor throughput"
BENCH_TESTS=logflood,colorflood,cursorstorm

only="" save_baseline=0 bin="" keep=0 fps=0
while [ $# -gt 0 ]; do
  case "$1" in
    --only) only="${2:?--only needs a list}"; shift ;;
    --save-baseline) save_baseline=1 ;;
    --bin) bin="${2:?--bin needs a path}"; shift ;;
    --fps) fps="${2:?--fps needs a number}"; shift ;;
    --keep) keep=1 ;;
    -h|--help) sed -n '2,/^set -euo/p' "$0" | sed '$d'; exit 0 ;;
    -*) die "unknown option: $1" ;;
    *) die "unexpected argument: $1" ;;
  esac
  shift
done
[ "$(uname -s)" = Darwin ] || die "the performance check needs macOS and a display"
need cargo; need git; need sample; need python3

[ "$save_baseline" = 0 ] || [ -z "$only" ] || die "--save-baseline needs a full run (no --only)"
workloads="$ALL_WORKLOADS"
if [ -n "$only" ]; then
  workloads="${only//,/ }"
  for w in $workloads; do
    case " $ALL_WORKLOADS " in *" $w "*) ;; *) die "no workload $w (they are: ${ALL_WORKLOADS// /, })" ;; esac
  done
fi

tools() { python3 -I "$ROOT/scripts/perf_tools.py" "$@"; }

# ---------------------------------------------------------------- machine, scratch dir, cleanup

machine_id="$(sysctl -n hw.model)-$(sysctl -n machdep.cpu.brand_string)"
machine_id="$(printf '%s' "$machine_id" | tr -c 'A-Za-z0-9._-' '-' | sed 's/--*/-/g; s/-$//')"
baseline="$ROOT/tools/bench/baselines/$machine_id.json"

stamp="$(date +%Y%m%d-%H%M%S)"
base_dir="${TMPDIR:-/tmp}"; base_dir="${base_dir%/}/wormhole-perf"
mkdir -p "$base_dir"
scratch="$(cd "$base_dir" && pwd -P)/$stamp-$$"
mkdir -p "$scratch"
case "$scratch/" in "$(cd "$ROOT" && pwd -P)"/*) die "the scratch dir must be outside the repository" ;; esac
result="$base_dir/$stamp.json"

started=()   # the instances' scratch dirs, to stop them again
# shellcheck disable=SC2329  # runs from the EXIT trap
cleanup() {
  local dir
  for dir in ${started[@]+"${started[@]}"}; do
    "$ROOT/scripts/devctl-instance.sh" stop "$dir" >/dev/null 2>&1 || warn "could not stop the instance in $dir"
  done
  if [ "$keep" = 1 ]; then
    log "scratch dir kept: $scratch"
  else
    # Exactly the dir this run made (never a glob).
    [[ "$scratch" == */wormhole-perf/$stamp-$$ ]] && rm -rf "$scratch"
  fi
}
trap cleanup EXIT
trap 'exit 130' INT TERM

# ---------------------------------------------------------------- builds

if [ -z "$bin" ]; then
  log "building the release app (devctl) into $TARGET_DIR/perf-build"
  (cd "$ROOT" && CARGO_TARGET_DIR="$TARGET_DIR/perf-build" cargo build --release --features devctl --quiet)
  bin="$TARGET_DIR/perf-build/release/$APP_BINARY"
  binary_from="HEAD"
else
  binary_from="$bin"
fi
[ -x "$bin" ] || die "no binary $bin"
# Our own copy: a later build can't replace it while it runs.
mkdir -p "$scratch/app"
cp -c "$bin" "$scratch/app/$APP_BINARY" 2>/dev/null || cp "$bin" "$scratch/app/$APP_BINARY"
bin="$scratch/app/$APP_BINARY"

log "building ttystorm"
(cd "$ROOT/tools/bench" && cargo build --release --quiet)
ttystorm="$ROOT/tools/bench/target/release/ttystorm"
[ -x "$ttystorm" ] || die "no $ttystorm"

# ---------------------------------------------------------------- driving an instance

# send <ctl dir> <command>...: one batch for the instance's next tick (written whole, then moved).
send() {
  local ctl="$1"; shift
  local waited=0
  while [ -e "$ctl/cmd" ] && [ "$waited" -lt 100 ]; do sleep 0.1; waited=$((waited + 1)); done
  printf '%s\n' "$@" > "$ctl/cmd.tmp"
  mv "$ctl/cmd.tmp" "$ctl/cmd"
}

# wait_for <seconds> <description> <command>...: poll until the command succeeds.
wait_for() {
  local seconds="$1" what="$2"; shift 2
  for _ in $(seq $((seconds * 5))); do "$@" && return 0; sleep 0.2; done
  die "timed out after ${seconds}s waiting for $what"
}

lines_in() { if [ -f "$1" ]; then wc -l < "$1" | tr -d ' '; else echo 0; fi; }
# lines_over <file> <n>: the file has more than n lines now.
# shellcheck disable=SC2329  # runs through wait_for
lines_over() { [ "$(lines_in "$1")" -gt "$2" ]; }

# start_workload <name> <ttystorm arguments>: an instance whose only terminal runs ttystorm once
# the window has its size (the animations and the benchmark read the terminal's size when they
# start), then cat. Sets $dir, $ctl, $pid.
start_workload() {
  local name="$1"; shift
  dir="$scratch/$name"
  mkdir -p "$dir"
  cat > "$dir/shell.sh" <<EOF
#!/bin/bash
while [ ! -e "$dir/go" ]; do sleep 0.1; done
"$ttystorm" $* 2>"$dir/ttystorm.err"
exec /bin/cat
EOF
  chmod +x "$dir/shell.sh"
  started+=("$dir")
  "$ROOT/scripts/devctl-instance.sh" --bin "$bin" --shell "$dir/shell.sh" "$dir" > "$dir/start.txt"
  ctl="$dir/ctl"
  pid="$(sed -n 's/^pid=//p' "$dir/start.txt")"
  send "$ctl" "onboarding close" "appearance dark" "resize $WINDOW_W $WINDOW_H"
  sleep 3
  : > "$dir/go"
}

stop_workload() {
  "$ROOT/scripts/devctl-instance.sh" stop "$1" >/dev/null 2>&1
}

# measure_draws <ctl dir>: FRAME_RUNS times `frames`, each waited for.
measure_draws() {
  local ctl="$1" run n
  for run in $(seq "$FRAME_RUNS"); do
    n="$(lines_in "$ctl/frames.txt")"
    send "$ctl" "frames $FRAMES"
    wait_for 120 "frames run $run" lines_over "$ctl/frames.txt" "$n"
  done
}

run_busy_workload() {
  local name="$1" args="$2"
  log "$name: $args"
  start_workload "$name" "$args"
  sleep "$SETTLE_SECONDS"
  measure_draws "$ctl"
  # A sample that lacks the main thread has been seen once; take it again rather than fail the run.
  for attempt in 1 2; do
    sample "$pid" "$SAMPLE_SECONDS" -file "$dir/sample.txt" >/dev/null 2>&1 || die "$name: sample failed"
    if tools workload "$name" "$ctl/frames.txt" "$dir/sample.txt" > "$dir/workload.json" 2>"$dir/parse.err"; then
      break
    fi
    cp "$dir/sample.txt" "$base_dir/failed-sample-$stamp-$name-$attempt.txt"
    [ "$attempt" = 1 ] || die "$name: $(cat "$dir/parse.err") (sample kept in $base_dir)"
  done
  cp "$ctl/frames.txt" "$dir/frames.keep"
  stop_workload "$dir"
  assemble_args+=(--workload "$name" "$dir/workload.json")
}

run_throughput() {
  log "throughput: ttystorm bench --only $BENCH_TESTS"
  start_workload throughput "bench --only $BENCH_TESTS --size 16 --runs 3 --save $scratch/throughput/bench.json"
  wait_for 600 "the benchmark" test -f "$dir/bench.json"
  tools throughput "$dir/bench.json" > "$dir/throughput.json"
  stop_workload "$dir"
  assemble_args+=(--throughput "$dir/throughput.json")
}

# ---------------------------------------------------------------- run

assemble_args=()
for w in $workloads; do
  case "$w" in
    throughput) run_throughput ;;
    flood-ascii) run_busy_workload "$w" "flood --kind logflood" ;;
    flood-truecolor) run_busy_workload "$w" "flood --kind colorflood" ;;
    *) run_busy_workload "$w" "$w --fps $fps --no-hud" ;;
  esac
done

commit="$(git -C "$ROOT" rev-parse --short HEAD)"
[ -z "$(git -C "$ROOT" status --porcelain --untracked-files=no)" ] || commit="$commit-dirty"
tools assemble "$result" \
  "commit=$commit" "binary=$binary_from" "machine_id=$machine_id" \
  "model=$(sysctl -n hw.model)" "cpu=$(sysctl -n machdep.cpu.brand_string)" \
  "date=$(date -u +%Y-%m-%dT%H:%M:%SZ)" "os=$(sw_vers -productVersion)" \
  "window=${WINDOW_W}x${WINDOW_H}" "animation_fps=$fps" "display_asleep=$(tools display)" \
  "frames_per_run=$FRAMES" "frame_runs=$FRAME_RUNS" "sample_seconds=$SAMPLE_SECONDS" \
  ${assemble_args[@]+"${assemble_args[@]}"}

# ---------------------------------------------------------------- report

limits=(--limits "$DRAW_FACTOR" "$DRAW_MARGIN_MS" "$BUSY_FACTOR" "$BUSY_MARGIN_PTS" "$THROUGHPUT_FLOOR")
status=0
if [ -f "$baseline" ]; then
  tools report "$result" "$baseline" "${limits[@]}" || status=$?
else
  tools report "$result" "${limits[@]}"
  echo
  echo "No baseline for this machine ($machine_id): nothing to compare. To keep this run as one:"
  echo "  scripts/perf.sh --save-baseline   (and commit tools/bench/baselines/$machine_id.json)"
fi
echo
echo "result: $result"

if [ "$save_baseline" = 1 ]; then
  mkdir -p "$(dirname "$baseline")"
  cp "$result" "$baseline"
  log "baseline written: $baseline"
  status=0
fi
exit "$status"
