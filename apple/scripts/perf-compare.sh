#!/usr/bin/env bash
# Spike step 4: the Swift terminal's numbers, measured the way scripts/perf.sh measures the GPUI app
# (apple/MEASURING.md has the definitions). One workload at a time, each in its own isolated
# instance of the Swift app, driven through its control folder:
#
#   apple/scripts/perf-compare.sh --app <path to the Swift app's Wormhole executable> [options]
#     --only <a,b>    warptunnel, matrix, flood-ascii, flood-truecolor, throughput, idle
#     --ttystorm <p>  the ttystorm binary (default: apple/target/ttystorm/release/ttystorm)
#     --grid 200x69   the fixed grid (default; the GPUI baseline's terminal)
#     --keep          keep the scratch dir and print it
#   Results: files in $TMPDIR/wormhole-swift-perf/<time>/ and the path on stdout.
#
# Never touches the real Wormhole's data: each instance has a data folder of its own, named
# Wormhole-swift-perf, in the scratch dir. The instances it starts are stopped by PID.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
APPLE="$(dirname "$HERE")"
ROOT="$(dirname "$APPLE")"
# shellcheck source=../../scripts/lib.sh
. "$ROOT/scripts/lib.sh"

FRAME_RUNS=3 FRAMES=60 SAMPLE_SECONDS=5 SETTLE_SECONDS=4 IDLE_SECONDS=10
ALL_WORKLOADS="warptunnel matrix flood-ascii flood-truecolor throughput idle"
BENCH_TESTS=logflood,colorflood,cursorstorm

app="" only="" keep=0 grid="200x69" ttystorm="$APPLE/target/ttystorm/release/ttystorm"
while [ $# -gt 0 ]; do
  case "$1" in
    --app) app="${2:?--app needs a path}"; shift ;;
    --only) only="${2:?--only needs a list}"; shift ;;
    --ttystorm) ttystorm="${2:?--ttystorm needs a path}"; shift ;;
    --grid) grid="${2:?--grid needs COLSxROWS}"; shift ;;
    --keep) keep=1 ;;
    -h|--help) sed -n '2,/^set -euo/p' "$0" | sed '$d'; exit 0 ;;
    *) die "unknown option: $1" ;;
  esac
  shift
done
[ -n "$app" ] && [ -x "$app" ] || die "--app <the Swift app's executable> is needed"
[ -x "$ttystorm" ] || die "no ttystorm at $ttystorm (build it: see tools/bench/README.md)"
need sample; need python3
workloads="${only:-$ALL_WORKLOADS}"; workloads="${workloads//,/ }"

stamp="$(date +%Y%m%d-%H%M%S)"
base="${TMPDIR:-/tmp}"; base="${base%/}/wormhole-swift-perf"
mkdir -p "$base"
scratch="$(cd "$base" && pwd -P)/$stamp-$$"
mkdir -p "$scratch"
case "$scratch/" in "$(cd "$ROOT" && pwd -P)"/*) die "the scratch dir must be outside the repository" ;; esac
pids=()
# shellcheck disable=SC2329  # runs from the EXIT trap
cleanup() {
  local pid
  for pid in ${pids[@]+"${pids[@]}"}; do kill "$pid" 2>/dev/null || true; done
  if [ "$keep" = 1 ]; then
    log "scratch dir kept: $scratch"
  else
    # Exactly the dir this run made (never a glob).
    [[ "$scratch" == */wormhole-swift-perf/$stamp-$$ ]] && rm -rf "$scratch"
  fi
}
trap cleanup EXIT
trap 'exit 130' INT TERM

tools() { python3 -I "$ROOT/scripts/perf_tools.py" "$@"; }
lines_in() { if [ -f "$1" ]; then wc -l < "$1" | tr -d ' '; else echo 0; fi; }
# shellcheck disable=SC2329  # runs through wait_for
lines_over() { [ "$(lines_in "$1")" -gt "$2" ]; }
wait_for() {
  local seconds="$1" what="$2"; shift 2
  for _ in $(seq $((seconds * 5))); do "$@" && return 0; sleep 0.2; done
  die "timed out after ${seconds}s waiting for $what"
}
send() { local ctl="$1"; shift; printf '%s\n' "$@" > "$ctl/cmd.tmp"; mv "$ctl/cmd.tmp" "$ctl/cmd"; }

# The machine's state, printed once: the load average and what else is using the CPU.
out="$base/$stamp"; mkdir -p "$out"
{
  echo "load average: $(uptime | sed 's/.*load averages*: //')"
  echo "busy processes (over 5 % of a core, one sample):"
  ps -Ao pcpu,comm | awk 'NR>1 && $1 > 5 { printf "  %5.1f %s\n", $1, $2 }' | sort -rn | head -8
} | tee "$out/machine.txt"

# start_workload <name> <ttystorm arguments>: an instance whose only terminal runs ttystorm once the
# window has its grid, then cat. Sets $dir, $ctl, $pid.
start_workload() {
  local name="$1"; shift
  dir="$scratch/$name"; mkdir -p "$dir/Wormhole-swift-perf" "$dir/ctl"
  if [ -n "$*" ]; then
    cat > "$dir/shell.sh" <<SHELL
#!/bin/bash
while [ ! -e "$dir/go" ]; do sleep 0.1; done
"$ttystorm" $* 2>"$dir/ttystorm.err"
exec /bin/cat
SHELL
  else
    printf '#!/bin/bash\nexec /bin/cat\n' > "$dir/shell.sh"
  fi
  chmod +x "$dir/shell.sh"
  ctl="$dir/ctl"
  WORMHOLE_DATA_DIR="$dir/Wormhole-swift-perf" WORMHOLE_DEVCTL="$ctl" WORMHOLE_SHELL="$dir/shell.sh" \
    WORMHOLE_SWIFT_GRID="$grid" WORMHOLE_BACKGROUND=1 "$app" >"$dir/app.out" 2>&1 &
  pid=$!
  pids+=("$pid")
  sleep 3
  kill -0 "$pid" 2>/dev/null || die "$name: the app exited: $(cat "$dir/app.out")"
  : > "$dir/go"
}
stop_workload() {
  send "$ctl" quit 2>/dev/null || true
  for _ in $(seq 25); do kill -0 "$pid" 2>/dev/null || return 0; sleep 0.2; done
  kill "$pid" 2>/dev/null || true
}

run_busy_workload() {
  local name="$1" args="$2" run n
  log "$name: $args"
  start_workload "$name" "$args"
  sleep "$SETTLE_SECONDS"
  for run in $(seq "$FRAME_RUNS"); do
    n="$(lines_in "$ctl/frames.txt")"
    send "$ctl" "frames $FRAMES"
    wait_for 120 "frames run $run" lines_over "$ctl/frames.txt" "$n"
  done
  sample "$pid" "$SAMPLE_SECONDS" -file "$dir/sample.txt" >/dev/null 2>&1 || die "$name: sample failed"
  tools workload "$name" "$ctl/frames.txt" "$dir/sample.txt" > "$out/$name.json" || die "$name: cannot read the numbers"
  cp "$ctl/frames.txt" "$out/$name.frames.txt"
  cp "$dir/sample.txt" "$out/$name.sample.txt"
  stop_workload
}

run_throughput() {
  log "throughput: ttystorm bench --only $BENCH_TESTS"
  start_workload throughput "bench --only $BENCH_TESTS --size 16 --runs 3 --save $scratch/throughput/bench.json"
  wait_for 600 "the benchmark" test -f "$dir/bench.json"
  tools throughput "$dir/bench.json" > "$out/throughput.json"
  cp "$dir/bench.json" "$out/throughput.bench.json"
  stop_workload
}

# The idle check: the window open, a program that prints nothing, no input. A sample of the whole
# process (every thread), and what the kernel counts for it over IDLE_SECONDS (`top`: CPU share,
# IDLEW = interrupt wakeups from idle, WQ = workqueue threads, CSW = context switches).
run_idle() {
  log "idle: cat for ${IDLE_SECONDS}s"
  start_workload idle ""
  sleep "$SETTLE_SECONDS"
  top -l 3 -s "$((IDLE_SECONDS / 2))" -pid "$pid" -stats pid,command,cpu,idlew,wq,csw > "$out/idle.top.txt" 2>&1 || true
  sample "$pid" "$IDLE_SECONDS" -file "$out/idle.sample.txt" >/dev/null 2>&1 || die "idle: sample failed"
  stop_workload
}

for w in $workloads; do
  case "$w" in
    throughput) run_throughput ;;
    idle) run_idle ;;
    flood-ascii) run_busy_workload "$w" "flood --kind logflood" ;;
    flood-truecolor) run_busy_workload "$w" "flood --kind colorflood" ;;
    warptunnel|matrix) run_busy_workload "$w" "$w --fps 0 --no-hud" ;;
    *) die "no workload $w (they are: ${ALL_WORKLOADS// /, })" ;;
  esac
done
echo "results: $out"
