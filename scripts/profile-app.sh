#!/usr/bin/env bash
# Profile: what a frame and a PTY read cost in a release build of Wormhole under fixed loads, part by
# part, compared with the profile saved for this machine (tools/bench/baselines/profile/<id>.json).
# Where scripts/perf.sh says *whether* drawing or reading got slower, this says *where*: the
# snapshot, the rows, shaping, painting, allocations, the parser, the lock.
#
#   scripts/profile-app.sh [options]
#     --only <a,b>      run only these loads: warptunnel, matrix (ttystorm animations, at --fps 0),
#                       flood-truecolor (ttystorm flood --kind colorflood)
#     --seconds <n>     how long each load is profiled (default 5)
#     --save-baseline   write this run as the profile baseline of this machine
#     --bin <path>      profile this prebuilt devctl binary instead of building HEAD
#     --keep            keep the scratch dir (instances' data and every report) and print it
#     -h, --help
#   Results: each load's summary, then a table of every number against the baseline with the change,
#   and $TMPDIR/wormhole-profile/<time>.json (the commit, the machine and each load's full report).
#   Exit 1 when a number is worse than perf_tools.py's PROFILE_TOLERANCES allow; 0 otherwise, also
#   without a baseline for this machine (it says how to save one).
#
# Each load runs in its own isolated instance (scripts/devctl-instance.sh, the window 2400x1300, a
# wrapper as the "shell" that starts ttystorm once the window has its size), settles for 4 s, and is
# then recorded by devctl `perf-profile <seconds>`: the window drawn again and again (as `frames`
# does, so it doesn't matter whether the window can be seen), each draw's time and parts and the UI
# thread's allocations, and every terminal's reader and lock meanwhile. What each number means:
# repo-docs/docs/performance.md, "Profiling".
#
# Noise: times are wall clock on a laptop (loose tolerances, as perf.sh's); allocations, rows and
# words are counted, but follow the load's phase. Run it with nothing else busy, and run it again
# before believing a single worse line. Compare a change against a baseline saved from its parent on
# the same machine (a build of the parent in a worktree, saved with --bin), not against an old one.
#
# Everything lives in a scratch dir under $TMPDIR (never the repository, never your data dir); the
# script stops only the instances it started, by PID, also when it fails or is interrupted. The release
# build goes to $CARGO_TARGET_DIR/perf-build (target/perf-build), shared with perf.sh.
set -euo pipefail
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"
# shellcheck source=perf_lib.sh
. "$(dirname "$0")/perf_lib.sh"

WINDOW_W=2400 WINDOW_H=1300
SETTLE_SECONDS=4
ALL_LOADS="warptunnel matrix flood-truecolor"

only="" save_baseline=0 bin="" keep=0 seconds=5
while [ $# -gt 0 ]; do
  case "$1" in
    --only) only="${2:?--only needs a list}"; shift ;;
    --seconds) seconds="${2:?--seconds needs a number}"; shift ;;
    --save-baseline) save_baseline=1 ;;
    --bin) bin="${2:?--bin needs a path}"; shift ;;
    --keep) keep=1 ;;
    -h|--help) sed -n '2,/^set -euo/p' "$0" | sed '$d'; exit 0 ;;
    -*) die "unknown option: $1" ;;
    *) die "unexpected argument: $1" ;;
  esac
  shift
done
[ "$(uname -s)" = Darwin ] || die "profiling needs macOS and a display"
need cargo; need git; need python3
[[ "$seconds" =~ ^[0-9]+(\.[0-9]+)?$ ]] || die "--seconds takes a number, not $seconds"
# perf-profile's own range, checked before anything is built.
awk -v s="$seconds" 'BEGIN { exit !(s >= 0.1 && s <= 600) }' || die "--seconds is 0.1 to 600, not $seconds"
[ "$save_baseline" = 0 ] || [ -z "$only" ] || die "--save-baseline needs a full run (no --only)"
loads="$ALL_LOADS"
if [ -n "$only" ]; then
  loads="${only//,/ }"
  for l in $loads; do
    case " $ALL_LOADS " in *" $l "*) ;; *) die "no load $l (they are: ${ALL_LOADS// /, })" ;; esac
  done
fi

machine_id="$(machine_id)"
baseline="$ROOT/tools/bench/baselines/profile/$machine_id.json"
perf_scratch wormhole-profile
result="$base_dir/$stamp.json"
perf_builds "$bin"

# wait_profile <name> <seconds>: until the report is written; stops with the instance's own words
# when the command failed (its results file) or the instance is gone.
wait_profile() {
  local name="$1" seconds="$2" failed
  for _ in $(seq $((seconds * 5))); do
    [ -f "$ctl/$name.json" ] && return 0
    failed="$(grep -h -e '^perf-profile failed:' -e '^unknown command: perf-profile' "$ctl/out.txt" 2>/dev/null || true)"
    [ -z "$failed" ] || die "$name: $failed"
    kill -0 "$pid" 2>/dev/null || die "$name: the instance (pid $pid) exited (see $dir/app.log)"
    sleep 0.2
  done
  die "$name: no report after ${seconds}s (the instance's results: $(cat "$ctl/out.txt" 2>/dev/null))"
}

# profile_load <name> <ttystorm arguments>: the load in an instance, `perf-profile` once it settled.
profile_load() {
  local name="$1" args="$2"
  log "$name: ttystorm $args, profiled for ${seconds}s"
  start_workload "$name" "$args"
  sleep "$SETTLE_SECONDS"
  send "$ctl" "perf-profile $seconds $name"
  wait_profile "$name" $((${seconds%.*} + 120))
  cat "$ctl/$name.txt"
  echo
  stop_workload "$dir"
  assemble_args+=(--workload "$name" "$ctl/$name.json")
}

assemble_args=()
for l in $loads; do
  case "$l" in
    flood-truecolor) profile_load "$l" "flood --kind colorflood" ;;
    *) profile_load "$l" "$l --fps 0 --no-hud" ;;
  esac
done

tools assemble "$result" \
  "format=wormhole-profile-app" "commit=$(perf_commit)" "binary=$binary_from" "machine_id=$machine_id" \
  "model=$(sysctl -n hw.model)" "cpu=$(sysctl -n machdep.cpu.brand_string)" \
  "date=$(date -u +%Y-%m-%dT%H:%M:%SZ)" "os=$(sw_vers -productVersion)" \
  "window=${WINDOW_W}x${WINDOW_H}" "seconds=$seconds" "display_asleep=$(tools display)" \
  ${assemble_args[@]+"${assemble_args[@]}"}

status=0
if [ -f "$baseline" ]; then
  log "against the baseline $baseline"
  tools profile-compare "$result" "$baseline" || status=$?
else
  tools profile-compare "$result"
  echo
  echo "No profile baseline for this machine ($machine_id): nothing to compare. To keep this run as one:"
  echo "  scripts/profile-app.sh --save-baseline   (and commit tools/bench/baselines/profile/$machine_id.json)"
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
