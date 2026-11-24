#!/usr/bin/env bash
# The Swift app as an always-running dev instance: builds `macos/swift` (debug) whenever its sources
# change and restarts the instance when the build succeeds. When it fails, the last good instance
# keeps running and the compiler's errors are printed until a build succeeds. `scripts/dev.sh` is the
# same for the GPUI app.
#
#   macos/scripts/dev.sh        # build, start, then rebuild and restart on every change; Ctrl-C quits
#
# The instance keeps a data folder of its own (the app refuses any that is not named Wormhole-swift*),
# so it runs beside the real Wormhole without touching its state. It is only ever stopped by PID, and
# its windows and terminals come back after a restart as after any quit.
#   WORMHOLE_DATA_DIR   default ~/Library/Application Support/Wormhole-swift-dev
#   WORMHOLE_SHELL      default: what the app's settings say
# The Rust bridge must be linked once (`macos/scripts/build-ffi.sh swift`); a newer one is picked up.
# A debug build has the inspector (⇧⌥⌘I). One build at a time: while another swift or cargo build runs
# on this Mac (another worktree's, say), this one waits.
set -uo pipefail
. "$(dirname "$0")/../../scripts/lib.sh"

package="$ROOT/macos/swift"
bin="$package/.build/debug/Wormhole"
[ -e "$package/WormholeFFI.xcframework" ] || die "the bridge is not linked: run macos/scripts/build-ffi.sh swift first"

export WORMHOLE_DATA_DIR="${WORMHOLE_DATA_DIR:-$HOME/Library/Application Support/Wormhole-swift-dev}"
mkdir -p "$WORMHOLE_DATA_DIR" || die "cannot create $WORMHOLE_DATA_DIR"
run="$WORMHOLE_DATA_DIR/dev"
mkdir -p "$run"
# Set when this runs inside a Wormhole terminal or a test's shell: they belong to that instance. The
# dev instance is the person's to work in, so it is no background instance and has no control folder.
unset WORMHOLE_SOCKET WORMHOLE_TAB_ID WORMHOLE_BIN WORMHOLE_BACKGROUND WORMHOLE_DEVCTL

cd "$package" || die "cannot enter $package"
# The bindings are a link into `macos/dist`: a rebuilt bridge changes that file, not the link.
watch=(Sources Package.swift ../dist/WormholeBindings.swift)
stamp="$run/.build-started"
quiet="$run/.quiet"
build_log="$run/build.log"
app_pid=""
listing=""

alive() { [ -n "$1" ] && kill -0 "$1" 2>/dev/null; }
files() { find "${watch[@]}" -type f ! -name '.*' ! -name '*~' ! -name '*.swp' "$@" 2>/dev/null; }
# Edited or new files are newer than the stamp; deletions and renames change the listing.
changed() { [ -n "$(files -newer "$stamp" -print -quit)" ] || [ "$(files | cksum)" != "$listing" ]; }
wait_until_quiet() {
  while :; do
    touch "$quiet"
    sleep 0.3
    [ -n "$(files -newer "$quiet" -print -quit)" ] || break
  done
}
other_build() { pgrep -x swift-build >/dev/null || pgrep -x cargo >/dev/null || pgrep -x rustc >/dev/null; }

build() {
  if other_build; then
    log "another build is running; waiting for it"
    while other_build; do sleep 2; done
  fi
  touch "$stamp"
  listing="$(files | cksum)"
  log "$(date +%H:%M:%S)  building"
  swift build -c debug --product Wormhole > "$build_log" 2>&1
}

stop_app() {
  alive "$app_pid" || { app_pid=""; return 0; }
  # SIGTERM is the app's ⌘Q: it saves its windows and terminals. Only ever our PID.
  kill -TERM "$app_pid" 2>/dev/null
  for _ in $(seq 1 50); do alive "$app_pid" || break; sleep 0.1; done
  alive "$app_pid" && kill -KILL "$app_pid" 2>/dev/null
  wait "$app_pid" 2>/dev/null
  app_pid=""
}

start_app() {
  "$bin" >> "$run/app.log" 2>&1 &
  app_pid=$!
  log "dev instance running (pid $app_pid, data in $WORMHOLE_DATA_DIR)"
}

quit() {
  stop_app
  exit 0
}
trap quit INT TERM

while :; do
  if build; then
    stop_app
    start_app
  else
    printf '\n\033[1;97;41m BUILD FAILED \033[0m %s\n' \
      "$(alive "$app_pid" && echo "still running the last good build (pid $app_pid)" || echo "no dev instance yet")" >&2
    grep -E 'error:' "$build_log" | sort -u | head -n 20 >&2
  fi
  until changed; do
    # Quit by the person (⌘Q): nothing to restart until the sources change again.
    alive "$app_pid" || app_pid=""
    sleep 0.5
  done
  wait_until_quiet
done
