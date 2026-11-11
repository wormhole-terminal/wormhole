#!/usr/bin/env bash
# Frame CPU of the Swift renderer on the MOCK source: each synthetic scenario in its own isolated instance,
# `frames 60` three times through the control folder, and the idle check. These numbers are the renderer's
# alone (no PTY, no engine, no FFI): never compare them with the GPUI app's, which has all three.
#
#   macos/swift/scripts/mock-measure.sh [--bin <Wormhole executable>] [--grid 200x69] [--only a,b]
#
# Release build first: `swift build -c release` in macos/swift. Prints the load average and what else is busy.
set -euo pipefail

pkg="$(cd "$(dirname "$0")/.." && pwd)"
bin="$pkg/.build/release/Wormhole" grid="200x69" only="text flood noise matrix mixed unicode cursor"
while [ $# -gt 0 ]; do
  case "$1" in
    --bin) bin="${2:?--bin needs a path}"; shift ;;
    --grid) grid="${2:?--grid needs COLSxROWS}"; shift ;;
    --only) only="${2//,/ }"; shift ;;
    *) echo "unknown option $1" >&2; exit 1 ;;
  esac
  shift
done
[ -x "$bin" ] || { echo "no executable at $bin (swift build -c release)" >&2; exit 1; }

scratch="$(mktemp -d "${TMPDIR:-/tmp}/wormhole-swift-mock.XXXXXX")"
pid=""
cleanup() {
  [ -n "$pid" ] && kill "$pid" 2>/dev/null || true
  # Exactly the dir this run made.
  [[ "$scratch" == */wormhole-swift-mock.* ]] && rm -rf "$scratch"
}
trap cleanup EXIT

echo "load average: $(uptime | sed 's/.*load averages*: //')"
echo "busy processes (over 5 % of a core):"
ps -Ao pcpu,comm | awk 'NR>1 && $1 > 5 { printf "  %5.1f %s\n", $1, $2 }' | sort -rn | head -6
echo "mock source, grid $grid, release build: $("$bin" --help >/dev/null && echo ok)"

send() { printf '%s\n' "$2" > "$1/cmd.tmp"; mv "$1/cmd.tmp" "$1/cmd"; }
for scenario in $only; do
  dir="$scratch/$scenario"; mkdir -p "$dir/Wormhole-swift-mock" "$dir/ctl"
  WORMHOLE_DATA_DIR="$dir/Wormhole-swift-mock" WORMHOLE_DEVCTL="$dir/ctl" WORMHOLE_BACKGROUND=1 \
    WORMHOLE_SWIFT_GRID="$grid" "$bin" --mock "$scenario" --mock-hz 0 >"$dir/app.out" 2>&1 &
  pid=$!
  sleep 3
  for _ in 1 2 3; do
    n=0; [ -f "$dir/ctl/frames.txt" ] && n="$(wc -l < "$dir/ctl/frames.txt" | tr -d ' ')"
    send "$dir/ctl" "frames 60"
    for _ in $(seq 100); do
      [ -f "$dir/ctl/frames.txt" ] && [ "$(wc -l < "$dir/ctl/frames.txt" | tr -d ' ')" -gt "$n" ] && break
      sleep 0.2
    done
  done
  send "$dir/ctl" state; sleep 0.5
  echo "== $scenario"
  sed 's/^/  /' "$dir/ctl/frames.txt"
  tail -1 "$dir/ctl/out.txt" | sed 's/^/  state: /'
  send "$dir/ctl" quit; wait "$pid" 2>/dev/null || true; pid=""
done

echo "== idle (10 s, mock stopped, window open)"
dir="$scratch/idle"; mkdir -p "$dir/Wormhole-swift-mock"
WORMHOLE_DATA_DIR="$dir/Wormhole-swift-mock" WORMHOLE_BACKGROUND=1 WORMHOLE_SWIFT_GRID="$grid" "$bin" --mock text --idle 10 | sed 's/^/  /'
