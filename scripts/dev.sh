#!/usr/bin/env bash
# Always-running dev instance: rebuilds whenever crates/, Cargo.toml or Cargo.lock change and
# swaps to the new binary when the build succeeds. When it fails, the last good instance keeps
# running and the compiler errors are printed (plus a notification) until a build succeeds.
#
#   scripts/dev.sh              # debug build, restart on change
#   scripts/dev.sh --release    # optimized build
#   scripts/dev.sh --devctl     # with devctl: any of your processes can then read the instance's
#                               # screens and type into its terminals (your real shell), so only
#                               # when you need it; agents use scripts/devctl-instance.sh instead
#
# Keys (when run in a terminal): Enter = rebuild and restart now, q = quit.
#
# The dev instance keeps its own data dir (terminals, config, database, hook socket, logs), so it
# runs next to the real Wormhole without touching its state. It is only ever stopped by PID.
#   WORMHOLE_DATA_DIR       default ~/Library/Application Support/Wormhole-dev
#   WORMHOLE_SHELL          default: your login shell
#   WORMHOLE_DEV_FEATURES   cargo features, default none (--devctl adds devctl)
#   WORMHOLE_DEV_NOTIFY=0   no macOS notification when a build breaks or is fixed
# Other WORMHOLE_* variables (START_DIR, AGENT_HOME, LOG; DEVCTL with devctl) pass through.
set -uo pipefail
. "$(dirname "$0")/lib.sh"

cargo_args=(build)
profile=debug
with_devctl=0
while [ $# -gt 0 ]; do
  case "$1" in
    --release) cargo_args+=(--release); profile=release ;;
    --devctl) with_devctl=1 ;;
    -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
    *) die "unknown argument: $1" ;;
  esac
  shift
done
features="${WORMHOLE_DEV_FEATURES-}"
if [ "$with_devctl" = 1 ] && [[ " ${features//,/ } " != *" devctl "* ]]; then
  features="${features:+$features,}devctl"
fi
# The app and its session host: the host sits beside the app's copy, where the app looks for it.
set_app_build_args "$features"
cargo_args+=("${APP_BUILD_ARGS[@]}")
notify="${WORMHOLE_DEV_NOTIFY:-1}"

real_data="$HOME/Library/Application Support/$DATA_DIR_NAME"
if [ -z "${WORMHOLE_DATA_DIR:-}" ]; then
  # The dev instance's data from before the rename (the app's own moves when it starts).
  old_dev="$HOME/Library/Application Support/Andromeda-dev"
  new_dev="$HOME/Library/Application Support/$DATA_DIR_NAME-dev"
  if [ -d "$old_dev" ] && [ ! -e "$new_dev" ]; then
    mv "$old_dev" "$new_dev"
    if [ -f "$new_dev/andromeda.db" ] && [ ! -e "$new_dev/wormhole.db" ]; then
      for suffix in "" -wal -shm; do
        if [ -f "$new_dev/andromeda.db$suffix" ]; then
          mv "$new_dev/andromeda.db$suffix" "$new_dev/wormhole.db$suffix"
        fi
      done
    fi
    log "moved the dev data from $old_dev"
  fi
fi
export WORMHOLE_DATA_DIR="${WORMHOLE_DATA_DIR:-$HOME/Library/Application Support/$DATA_DIR_NAME-dev}"
mkdir -p "$WORMHOLE_DATA_DIR" || die "cannot create $WORMHOLE_DATA_DIR"
if [ -d "$real_data" ] && [ "$(cd "$WORMHOLE_DATA_DIR" && pwd -P)" = "$(cd "$real_data" && pwd -P)" ]; then
  die "WORMHOLE_DATA_DIR is the real app's data dir; the dev instance needs its own"
fi
run="$WORMHOLE_DATA_DIR/dev"
# A bare executable is drawn by AppKit as an app without a bundle (its traffic lights differ
# from the shipped app's), so the instance runs from a small bundle of its own, like the release.
dev_app="$run/$APP_NAME-Dev.app"
bin_dir="$dev_app/Contents/MacOS"
bin="$bin_dir/$APP_BINARY-Dev"   # the process name, so ⌘Tab and the menu tell it apart
legacy_bin="$run/bin/$APP_BINARY-Dev"   # where it ran before the bundle, for taking one over
mkdir -p "$bin_dir"
sed_escape() { printf '%s' "$1" | sed -e 's/[\\|&]/\\&/g'; }
# The release's Info.plist with its own name and id (so LaunchServices never mistakes it for the
# installed app) and without the document types, which would offer it in Finder's Open With.
sed \
  -e "s|@APP_NAME@|$(sed_escape "$APP_NAME Dev")|g" \
  -e "s|@APP_BINARY@|$(sed_escape "$APP_BINARY-Dev")|g" \
  -e "s|@BUNDLE_ID@|$(sed_escape "$BUNDLE_ID.dev")|g" \
  -e "s|@SHORT_VERSION@|$(sed_escape "${APP_VERSION%%[-+]*}")|g" \
  -e "s|@BUILD_VERSION@|$(sed_escape "${APP_VERSION%%[-+]*}")|g" \
  -e "s|@MIN_MACOS@|$(sed_escape "$MIN_MACOS")|g" \
  -e "s|@COPYRIGHT@|$(sed_escape "$COPYRIGHT")|g" \
  "$PACKAGING/Info.plist" > "$dev_app/Contents/Info.plist" || die "cannot write the dev bundle's Info.plist"
plutil -remove CFBundleDocumentTypes "$dev_app/Contents/Info.plist" >/dev/null 2>&1 || true
plutil -lint -s "$dev_app/Contents/Info.plist" || die "the dev bundle's Info.plist is not valid"
if [[ " ${features//,/ } " == *" devctl "* ]]; then
  export WORMHOLE_DEVCTL="${WORMHOLE_DEVCTL:-$run/ctl}"
  # Owner-only, which keeps other users out; your own processes can still use it, hence the warning.
  if ! { mkdir -p "$(dirname "$WORMHOLE_DEVCTL")" && { [ -d "$WORMHOLE_DEVCTL" ] || mkdir -m 700 "$WORMHOLE_DEVCTL"; } \
    && chmod 700 "$WORMHOLE_DEVCTL"; }; then
    die "cannot create $WORMHOLE_DEVCTL"
  fi
  warn "devctl is on: any of your processes can read this instance's screens and type into it ($WORMHOLE_DEVCTL)"
else
  # A build without devctl has no command file; one named in the environment isn't ours.
  unset WORMHOLE_DEVCTL
fi
# Set when this runs inside a Wormhole tab; they belong to that instance, not ours.
unset WORMHOLE_SOCKET WORMHOLE_TAB_ID WORMHOLE_BIN
# The dev instance is yours to work in: it comes to the front as usual, even when started from a
# shell that keeps test instances in the background.
unset WORMHOLE_BACKGROUND

cd "$ROOT" || die "cannot enter $ROOT"
watch=(crates Cargo.toml Cargo.lock)
stamp="$run/.build-started"
quiet="$run/.quiet"
build_log="$run/build.log"
app_log="$run/app.log"
app_pid=""
build_job=""
listing=""
last_ok=1

# Job control puts every background job in its own process group: a cancelled build can be
# stopped with its rustc children, and Ctrl-C reaches only this script, which then quits the
# app gracefully.
set -m

ok()   { printf '\033[1;32m==>\033[0m %s\n' "$*" >&2; }
fail() { printf '\n\033[1;97;41m %s \033[0m %s\n' "$1" "${2:-}" >&2; }
notify() {
  [ "$notify" = 1 ] || return 0
  osascript -e 'on run argv' -e 'display notification (item 2 of argv) with title (item 1 of argv)' \
    -e 'end run' "$@" >/dev/null 2>&1 || true
}

alive() { [ -n "$1" ] && kill -0 "$1" 2>/dev/null; }

files() {
  find "${watch[@]}" -type f ! -name '.*' ! -name '*~' ! -name '*.swp' ! -path '*/target/*' "$@" 2>/dev/null
}
# Edited or new files show up as newer than the stamp; deletions and renames change the listing.
changed() {
  [ -n "$(files -newer "$1" -print -quit)" ] || [ "$(files | cksum)" != "$listing" ]
}
wait_until_quiet() {
  while :; do
    touch "$quiet"
    sleep 0.3
    [ -n "$(files -newer "$quiet" -print -quit)" ] || break
  done
}

# 0 = built, 1 = failed, 2 = cancelled because the sources changed again.
build() {
  touch "$stamp"
  listing="$(files | cksum)"
  printf '\n\033[1;34m==>\033[0m %s  building (%s)\n' "$(date +%H:%M:%S)" "$profile" >&2
  # No job control inside, or the pipeline would get a process group of its own.
  (set +m; cargo "${cargo_args[@]}" --color=always 2>&1 | tee "$build_log") &
  build_job=$!
  while alive "$build_job"; do
    if changed "$stamp"; then
      wait_until_quiet
      log "sources changed again; restarting the build"
      stop_build
      return 2
    fi
    sleep 0.3
  done
  wait "$build_job"
  local status=$?
  build_job=""
  return "$status"
}

stop_build() {
  alive "$build_job" && kill -TERM -- "-$build_job" 2>/dev/null
  [ -n "$build_job" ] && wait "$build_job" 2>/dev/null
  build_job=""
}

report_failure() {
  local plain errors first
  plain="$(sed $'s/\033\\[[0-9;]*m//g' "$build_log")"
  errors="$(printf '%s\n' "$plain" | grep -E '^error(\[|:)' | grep -cv '^error: could not compile')"
  first="$(printf '%s\n' "$plain" | grep -m1 -E '^error(\[|:)')"
  if alive "$app_pid"; then
    fail "BUILD FAILED" "still running the last good build (pid $app_pid) until this is fixed"
  else
    fail "BUILD FAILED" "no dev instance yet; waiting for a build that works"
  fi
  printf '%s\n' "$plain" | grep -A1 -E '^error(\[|:)' | grep -E '^error|-->' \
    | grep -v '^error: could not compile' | head -n 20 >&2
  if [ "$last_ok" = 1 ]; then
    notify "$APP_NAME dev build failed" "${errors} error(s): ${first:-see the terminal}"
  fi
  last_ok=0
}

stop_app() {
  alive "$app_pid" || { app_pid=""; return 0; }
  log "stopping dev instance (pid $app_pid)"
  # devctl's quit first when it is on, then SIGTERM (which the app answers as ⌘Q, saving terminals
  # and scrollback), then SIGKILL; only ever our PID.
  if [ -n "${WORMHOLE_DEVCTL:-}" ]; then
    printf 'quit\n' > "$WORMHOLE_DEVCTL/cmd"
    for _ in $(seq 50); do alive "$app_pid" || break; sleep 0.1; done
  fi
  if alive "$app_pid"; then
    kill -TERM "$app_pid" 2>/dev/null
    # The app quits on SIGTERM as on ⌘Q, saving every window first.
    for _ in $(seq 50); do alive "$app_pid" || break; sleep 0.1; done
  fi
  alive "$app_pid" && kill -KILL "$app_pid" 2>/dev/null
  wait "$app_pid" 2>/dev/null
  app_pid=""
  rm -f "$run/app.pid"
}

start_app() {
  # A leftover command would quit the new instance on its first tick.
  [ -n "${WORMHOLE_DEVCTL:-}" ] && rm -f "$WORMHOLE_DEVCTL/cmd"
  "$bin" </dev/null >"$app_log" 2>&1 &
  app_pid=$!
  echo "$app_pid" > "$run/app.pid"
  sleep 1
  if alive "$app_pid"; then
    ok "dev instance running (pid $app_pid, data: $WORMHOLE_DATA_DIR)"
  else
    wait "$app_pid" 2>/dev/null
    fail "DEV INSTANCE EXITED" "right after starting (status $?)"
    [ -s "$app_log" ] && { echo "last output ($app_log):"; tail -n 20 "$app_log"; } >&2
    app_pid=""
  fi
}

# Swap only after a good build: the old instance keeps running until the new binary exists.
# The binary is copied (clone on APFS) so another `cargo build` can't replace it underneath us,
# and moved into place so the running instance keeps its own file.
swap() {
  local built="${CARGO_TARGET_DIR:-$ROOT/target}/$profile/$APP_BINARY"
  local built_host="${CARGO_TARGET_DIR:-$ROOT/target}/$profile/$HOST_BINARY" host="$bin_dir/$HOST_BINARY"
  cp -c "$built" "$bin.new" 2>/dev/null || cp "$built" "$bin.new" || { warn "cannot copy $built"; return 1; }
  if [ -x "$built_host" ]; then
    cp -c "$built_host" "$host.new" 2>/dev/null || cp "$built_host" "$host.new" \
      || { warn "cannot copy $built_host"; rm -f "$bin.new"; return 1; }
  fi
  stop_app
  mv -f "$bin.new" "$bin"
  # A rename, not an overwrite: hosts that run from the old file keep it open.
  [ ! -e "$host.new" ] || mv -f "$host.new" "$host"
  start_app
}

check_app() {
  [ -n "$app_pid" ] && ! alive "$app_pid" || return 0
  wait "$app_pid" 2>/dev/null
  local status=$?
  fail "DEV INSTANCE EXITED" "(status $status); Enter or the next change starts it again"
  [ -s "$app_log" ] && { echo "last output ($app_log):"; tail -n 20 "$app_log"; } >&2
  app_pid=""
  rm -f "$run/app.pid"
}

cleanup() {
  trap - INT TERM HUP EXIT
  stop_build
  stop_app
  rm -f "$run/loop.pid"
  exit 0
}
trap cleanup INT TERM HUP EXIT

# One dev loop per data dir; an instance left over from a killed loop is taken over (its
# binary lives in our folder, so it is ours), otherwise the new one would just activate it.
if [ -f "$run/loop.pid" ] && alive "$(cat "$run/loop.pid")"; then
  trap - INT TERM HUP EXIT
  die "a dev loop for this data dir is already running (pid $(cat "$run/loop.pid"))"
fi
echo $$ > "$run/loop.pid"
if [ -f "$run/app.pid" ]; then
  old="$(cat "$run/app.pid")"
  old_command="$(ps -p "$old" -o command= 2>/dev/null)"
  if alive "$old" && { [[ "$old_command" == "$bin"* ]] || [[ "$old_command" == "$legacy_bin"* ]]; }; then
    app_pid="$old"
    log "taking over dev instance left running (pid $app_pid)"
  fi
fi

log "watching ${watch[*]} · data: $WORMHOLE_DATA_DIR · Enter rebuilds + restarts, q quits"
tty=0
[ -t 0 ] && tty=1
force=1
while :; do
  if [ "$force" = 1 ] || changed "$stamp"; then
    [ "$force" = 1 ] || wait_until_quiet
    force=0
    build
    case $? in
      0)
        if swap; then
          [ "$last_ok" = 0 ] && notify "$APP_NAME dev build fixed" "running the new build"
          last_ok=1
        fi
        ;;
      2) force=1 ;;
      *) report_failure ;;
    esac
  fi
  check_app
  if [ "$tty" = 1 ]; then
    key=""
    if read -r -t 1 key; then
      case "$key" in q|Q) break ;; *) force=1 ;; esac
    fi
  else
    sleep 1
  fi
done
cleanup
