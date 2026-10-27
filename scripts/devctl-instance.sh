#!/usr/bin/env bash
# Start an isolated Wormhole to drive with devctl, or stop it again.
#
#   scripts/devctl-instance.sh [options] <scratch dir> [start dir]
#     --bin <path>      the devctl build to run (default: debug/Wormhole in $CARGO_TARGET_DIR or
#                       target/; build it first with `cargo build --features devctl`). It is copied
#                       into <scratch dir>/bin, so a later build can't replace it while it runs
#     --shell <path>    what a tab runs (default /bin/cat: typed text is echoed, never executed)
#     --env KEY=VALUE   more environment for the instance (repeatable; it wins over the defaults)
#   scripts/devctl-instance.sh stop <scratch dir>
#
# Prints `ctl=<dir>` (write commands to <dir>/cmd, read <dir>/out.txt; screenshots land there) and
# `pid=<pid>`. The instance keeps everything in <scratch dir>: data dir, HOME, agent home and the
# folder the command line tool installs `wh` into (cli-bin); it gets
# the Finder's PATH and no agent counts as installed (`--env WORMHOLE_AGENT_PATH=<dir>` with a copy
# of cat named like one puts it back); Touch ID is answered "cancel", a program that asks to control
# it is answered "Don't Allow" (--env WORMHOLE_AGENT_ASK=hold|session|quit|always changes that), notifications are logged
# instead of shown, and it runs in background mode: it opens behind your windows and never takes
# the focus. The first terminal opens in [start dir] (default: the scratch HOME). A fresh data dir
# is a first launch, whose first terminal runs the welcome (`Wormhole welcome`, while its feature flag
# is on, as in any development build) and no animation plays in a test instance: look at it, or send
# `onboarding close` first, which skips it (and closes the intro window when that is what opened).
# `stop` quits only the process this started, by the PID in <scratch dir>/pid, and then ends the session
# hosts in <scratch dir>/data/hosts (they outlive their app): TERM, KILL after 3 s, each found by its
# .meta file and checked to be a wormhole-host. A wormhole-host built next to --bin is copied to bin/
# too: the app looks for it beside its own executable.
set -euo pipefail
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"

usage() { sed -n '2,27p' "$0"; exit "${1:-0}"; }

# The process <scratch>/pid names, if it is still the binary this script started there.
own_pid() {
  local scratch="$1" pid command
  [ -f "$scratch/pid" ] || return 1
  pid="$(cat "$scratch/pid")"
  [[ "$pid" =~ ^[0-9]+$ ]] || return 1
  command="$(ps -p "$pid" -o command= 2>/dev/null)" || return 1
  [[ "$command" == "$scratch/bin/$APP_BINARY"* ]] || return 1
  echo "$pid"
}

if [ "${1:-}" = stop ]; then
  [ $# -eq 2 ] || usage 1
  scratch="$(cd "$2" && pwd -P)" || die "no scratch dir $2"
  if pid="$(own_pid "$scratch")"; then
    stop_isolated "$pid" "$scratch/ctl" || die "instance $pid is still running"
    log "instance $pid stopped"
  else
    log "no instance of this script runs from $scratch"
  fi
  stop_hosts "$scratch/data"
  exit 0
fi

bin="$TARGET_DIR/debug/$APP_BINARY" shell=/bin/cat
extra=()
while [ $# -gt 0 ]; do
  case "$1" in
    --bin) bin="${2:?--bin needs a path}"; shift ;;
    --shell) shell="${2:?--shell needs a path}"; shift ;;
    --env) extra+=("${2:?--env needs KEY=VALUE}"); shift ;;
    -h|--help) usage ;;
    -*) die "unknown option: $1" ;;
    *) break ;;
  esac
  shift
done
[ $# -ge 1 ] && [ $# -le 2 ] || usage 1
[ "$(uname -s)" = Darwin ] || die "an instance needs macOS and a display"
[ -x "$bin" ] || die "no $bin: build it with \`cargo build -p wormhole -p wormhole-host --features wormhole/devctl\` (or pass --bin)"

mkdir -p "$1"
scratch="$(cd "$1" && pwd -P)"
case "$scratch/" in "$(cd "$ROOT" && pwd -P)"/*) die "the scratch dir must be outside the repository" ;; esac
if pid="$(own_pid "$scratch")"; then
  die "an instance already runs from $scratch (pid $pid): \`$0 stop $scratch\` first"
fi
start="${2:-}"
if [ -n "$start" ]; then
  start="$(cd "$start" && pwd -P)" || die "no start dir $2"
fi

mkdir -p "$scratch/bin"
rm -f "$scratch/bin/$APP_BINARY"
cp -c "$bin" "$scratch/bin/$APP_BINARY" 2>/dev/null || cp "$bin" "$scratch/bin/$APP_BINARY"
# The session host, when it was built beside the app: the app finds it next to its own executable.
rm -f "$scratch/bin/$HOST_BINARY"
if [ -x "$(dirname "$bin")/$HOST_BINARY" ]; then
  cp -c "$(dirname "$bin")/$HOST_BINARY" "$scratch/bin/$HOST_BINARY" 2>/dev/null \
    || cp "$(dirname "$bin")/$HOST_BINARY" "$scratch/bin/$HOST_BINARY"
fi
mkdir -p "$scratch/home"
start_isolated "$scratch/bin/$APP_BINARY" "$scratch" "${start:-$(cd "$scratch/home" && pwd -P)}" "$shell" \
  ${extra[@]+"${extra[@]}"}
sleep 1
kill -0 "$pid" 2>/dev/null || die "the instance exited at once (see $scratch/app.log)"
echo "ctl=$scratch/ctl"
echo "pid=$pid"
