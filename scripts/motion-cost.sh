#!/usr/bin/env bash
# What the sidebar's looping animations cost while agents work, in a window in front: how often the
# list is built (the animations' redraws) and the CPU of an instance whose terminals all show a
# working agent, against the same instance once they are done (the floor). No real agent runs: each
# terminal's "shell" is a copy of /bin/cat named `claude`, which is detected as Claude Code, and
# devctl `hook <id> UserPromptSubmit` makes it work.
#
#   scripts/motion-cost.sh [options]
#     --bin <label>=<path>   measure this devctl build (repeatable: each is measured the same way, one
#                            after the other, to compare two builds); without one, the release devctl
#                            build of HEAD is built into $CARGO_TARGET_DIR/perf-build and measured
#     --agents <n>           terminals with a working agent (default 3)
#     --seconds <n>          how long each sample runs (default 10)
#     --keep                 keep the scratch dir and print it
#     -h, --help
#   Results: a row per build and state: the list's builds per second, the CPU % over --seconds, and
#   whether the window stayed in front throughout (a row where it didn't measured something else).
#
# Someone has to click: each instance runs as a normal app (not in background mode), and macOS
# doesn't let a script bring it forward, so the script waits until you click its window or its Dock
# icon, and then measures. A window in background mode measures nothing useful: covered, it is
# drawn by background mode's own 20 fps pump (`crate::background`), and behind another app GPUI
# draws it at most 30 times a second.
#
# Numbers from a laptop are noisy: run it with nothing else busy, and compare builds in one run.
# Everything lives in a scratch dir under $TMPDIR; the script stops only the instances it started.
set -euo pipefail
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"

usage() { sed -n '2,25p' "$0"; exit "${1:-0}"; }

bins=() agents=3 seconds=10 keep=0
while [ $# -gt 0 ]; do
  case "$1" in
    --bin) bins+=("${2:?--bin needs <label>=<path>}"); shift ;;
    --agents) agents="${2:?--agents needs a number}"; shift ;;
    --seconds) seconds="${2:?--seconds needs a number}"; shift ;;
    --keep) keep=1 ;;
    -h|--help) usage ;;
    *) die "unknown option: $1" ;;
  esac
  shift
done
[ "$(uname -s)" = Darwin ] || die "this needs macOS and a display"
[[ "$agents" =~ ^[1-9][0-9]*$ ]] || die "--agents takes a positive number"
[[ "$seconds" =~ ^[1-9][0-9]*$ ]] || die "--seconds takes a positive number"

scratch="$(mktemp -d "${TMPDIR:-/tmp}/wormhole-motion.XXXXXX")"
running=()
cleanup() {
  for dir in ${running[@]+"${running[@]}"}; do
    "$ROOT/scripts/devctl-instance.sh" stop "$dir" >/dev/null 2>&1 || true
  done
  if [ "$keep" = 1 ]; then log "kept $scratch"; else rm -rf "$scratch"; fi
}
trap cleanup EXIT

if [ ${#bins[@]} -eq 0 ]; then
  log "building the release app (devctl) into $TARGET_DIR/perf-build"
  set_app_build_args devctl
  (cd "$ROOT" && CARGO_TARGET_DIR="$TARGET_DIR/perf-build" cargo build --release "${APP_BUILD_ARGS[@]}" --quiet)
  bins=("HEAD=$TARGET_DIR/perf-build/release/$APP_BINARY")
fi

# The agent: cat under Claude Code's name. macOS kills a copy of a system binary that keeps Apple's
# signature, so the copy is signed again (ad hoc).
mkdir -p "$scratch/agent"
cp /bin/cat "$scratch/agent/claude"
codesign --sign - --force "$scratch/agent/claude" 2>/dev/null
agent="$scratch/agent/claude"

# send <ctl> <command>...: one batch, then its results once the tick has run it.
send() {
  local ctl="$1"; shift
  local waited=0
  while [ -e "$ctl/cmd" ] && [ "$waited" -lt 100 ]; do sleep 0.1; waited=$((waited + 1)); done
  rm -f "$ctl/out.txt"
  printf '%s\n' "$@" > "$ctl/cmd.tmp"
  mv "$ctl/cmd.tmp" "$ctl/cmd"
  waited=0
  while [ ! -s "$ctl/out.txt" ] && [ "$waited" -lt 100 ]; do sleep 0.1; waited=$((waited + 1)); done
  sleep 0.2
  cat "$ctl/out.txt" 2>/dev/null || true
}

list_builds() { send "$1" state | sed -n 's/.*sidebar list built \([0-9]*\).*/\1/p'; }

frontmost_pid() {
  osascript -e 'tell application "System Events" to get unix id of first application process whose frontmost is true' \
    2>/dev/null || true
}

# sample <label> <state> <ctl> <pid>: one row of the table.
sample() {
  local label="$1" state="$2" ctl="$3" pid="$4" before after cpu front=yes
  [ "$(frontmost_pid)" = "$pid" ] || front=no
  before="$(list_builds "$ctl")"
  # Two samples: the second covers the interval.
  cpu="$(top -l 2 -s "$seconds" -pid "$pid" -stats pid,cpu | awk -v pid="$pid" '$1 == pid { cpu = $2 } END { print cpu }')"
  after="$(list_builds "$ctl")"
  [ "$(frontmost_pid)" = "$pid" ] || front=no
  awk -v l="$label" -v s="$state" -v b=$((after - before)) -v t="$seconds" -v c="$cpu" -v f="$front" \
    'BEGIN { printf "%-12s %-8s %8.1f %7s  %s\n", l, s, b / t, c, f }'
}

rows=()
for entry in "${bins[@]}"; do
  label="${entry%%=*}" bin="${entry#*=}"
  [ "$label" != "$entry" ] || die "--bin takes <label>=<path>"
  [ -x "$bin" ] || die "no binary $bin"
  dir="$scratch/$label"
  log "$label: $agents working agents"
  started="$("$ROOT/scripts/devctl-instance.sh" --bin "$bin" --shell "$agent" --env WORMHOLE_BACKGROUND=0 "$dir")"
  running+=("$dir")
  ctl="$(sed -n 's/^ctl=//p' <<< "$started")" pid="$(sed -n 's/^pid=//p' <<< "$started")"

  send "$ctl" "onboarding close" >/dev/null
  for _ in $(seq 2 "$agents"); do send "$ctl" new >/dev/null; done
  # The agent check finds `claude` in each terminal (every ~2 s).
  for _ in $(seq 20); do
    [ "$(send "$ctl" state | grep -c '^[*+ ][0-9][0-9]* id=.* agent=Some(Claude)')" -lt "$agents" ] || break
    sleep 0.5
  done
  ids="$(send "$ctl" state | sed -n 's/^[*+ ][0-9][0-9]* id=\([0-9]*\) .*/\1/p')"
  [ "$(wc -w <<< "$ids")" -eq "$agents" ] || die "$label: expected $agents terminals, found: $ids"

  log "$label: click its window (terminals named claude) or its Dock icon to bring it to the front"
  for _ in $(seq 240); do [ "$(frontmost_pid)" = "$pid" ] && break; sleep 0.5; done
  [ "$(frontmost_pid)" = "$pid" ] || die "$label: its window wasn't brought to the front within 2 minutes"
  log "$label: in front; leave it there for about $((2 * seconds + 10)) s"

  hooks=()
  for id in $ids; do hooks+=("hook $id UserPromptSubmit"); done
  send "$ctl" "${hooks[@]}" >/dev/null
  sleep 2
  working="$(send "$ctl" state | grep -c '^[*+ ][0-9][0-9]* id=[0-9]* Busy' || true)"
  [ "$working" -eq "$agents" ] || warn "$label: only $working of $agents terminals show Busy (an agent working)"
  rows+=("$(sample "$label" working "$ctl" "$pid")")

  hooks=()
  for id in $ids; do hooks+=("hook $id Stop"); done
  send "$ctl" "${hooks[@]}" >/dev/null
  sleep 2
  rows+=("$(sample "$label" "done" "$ctl" "$pid")")

  "$ROOT/scripts/devctl-instance.sh" stop "$dir" >/dev/null
  running=()
done

printf '\n%-12s %-8s %8s %7s  %s\n' build state "list/s" "cpu %" "in front"
printf '%s\n' "${rows[@]}"
