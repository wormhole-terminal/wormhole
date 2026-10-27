# shellcheck shell=bash
# shellcheck disable=SC2034,SC2154  # globals set here for the caller, and `keep` set by it
# What scripts/perf.sh and scripts/profile-app.sh share: the machine's id, a scratch dir under $TMPDIR
# that is removed again, the builds (a release app with devctl, ttystorm), and isolated instances
# (scripts/devctl-instance.sh) whose only terminal runs a ttystorm load, driven through devctl.
# Sourced after lib.sh. The caller sets WINDOW_W and WINDOW_H (the window's size) and `keep` (1
# keeps the scratch dir); the functions set the globals named in their comments.

tools() { python3 -I "$ROOT/scripts/perf_tools.py" "$@"; }

# The machine's id, as baselines are named: model and CPU, letters, digits, `.`, `_` and `-`.
machine_id() {
  local id
  id="$(sysctl -n hw.model)-$(sysctl -n machdep.cpu.brand_string)"
  printf '%s' "$id" | tr -c 'A-Za-z0-9._-' '-' | sed 's/--*/-/g; s/-$//'
}

# perf_scratch <name>: sets $base_dir ($TMPDIR/<name>), $stamp and $scratch (this run's dir in it),
# and the EXIT trap that stops the instances started (`started`) and removes the scratch dir.
perf_scratch() {
  scratch_name="$1"
  stamp="$(date +%Y%m%d-%H%M%S)"
  base_dir="${TMPDIR:-/tmp}"; base_dir="${base_dir%/}/$scratch_name"
  mkdir -p "$base_dir"
  scratch="$(cd "$base_dir" && pwd -P)/$stamp-$$"
  mkdir -p "$scratch"
  case "$scratch/" in "$(cd "$ROOT" && pwd -P)"/*) die "the scratch dir must be outside the repository" ;; esac
  started=()   # the instances' scratch dirs, to stop them again
  trap perf_cleanup EXIT
  trap 'exit 130' INT TERM
}

# shellcheck disable=SC2329  # runs from the EXIT trap
perf_cleanup() {
  local dir
  for dir in ${started[@]+"${started[@]}"}; do
    "$ROOT/scripts/devctl-instance.sh" stop "$dir" >/dev/null 2>&1 || warn "could not stop the instance in $dir"
  done
  if [ "$keep" = 1 ]; then
    log "scratch dir kept: $scratch"
  else
    # Exactly the dir this run made (never a glob).
    [[ "$scratch" == */$scratch_name/$stamp-$$ ]] && rm -rf "$scratch"
  fi
}

# perf_builds [<prebuilt devctl binary>]: sets $bin (our own copy of the app, the session host beside
# it), $binary_from (HEAD or the binary given) and $ttystorm. Without a binary, builds the release app
# with devctl into $TARGET_DIR/perf-build; ttystorm (its own workspace) into $TARGET_DIR/ttystorm-build.
perf_builds() {
  bin="${1:-}"
  if [ -z "$bin" ]; then
    log "building the release app (devctl) into $TARGET_DIR/perf-build"
    set_app_build_args devctl
    (cd "$ROOT" && CARGO_TARGET_DIR="$TARGET_DIR/perf-build" cargo build --release "${APP_BUILD_ARGS[@]}" --quiet)
    bin="$TARGET_DIR/perf-build/release/$APP_BINARY"
    binary_from="HEAD"
  else
    binary_from="$bin"
  fi
  [ -x "$bin" ] || die "no binary $bin"
  # Our own copy: a later build can't replace it while it runs.
  mkdir -p "$scratch/app"
  cp -c "$bin" "$scratch/app/$APP_BINARY" 2>/dev/null || cp "$bin" "$scratch/app/$APP_BINARY"
  # The session host beside it, where the app looks for it (a --bin without one runs in-process).
  copy_host_beside "$bin" "$scratch/app"
  bin="$scratch/app/$APP_BINARY"

  log "building ttystorm"
  # Its own workspace, so its own target dir: an inherited CARGO_TARGET_DIR would send it into the app's.
  (cd "$ROOT/tools/bench" && CARGO_TARGET_DIR="$TARGET_DIR/ttystorm-build" cargo build --release --quiet)
  ttystorm="$TARGET_DIR/ttystorm-build/release/ttystorm"
  [ -x "$ttystorm" ] || die "no $ttystorm"
}

# The commit measured, `-dirty` with uncommitted changes to tracked files.
perf_commit() {
  local commit
  commit="$(git -C "$ROOT" rev-parse --short HEAD)"
  [ -z "$(git -C "$ROOT" status --porcelain --untracked-files=no)" ] || commit="$commit-dirty"
  echo "$commit"
}

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
# start), then cat. Sets $dir, $ctl, $pid. A caller may set, before the call:
#   workload_setup     (array) more devctl commands, sent with the first batch (settings, say);
#   workload_watchers  shell lines the terminal runs in the background just before ttystorm, with
#                      $WORMHOLE_BIN and $WORMHOLE_TAB_ID of the terminal and $dir of the workload
#                      in the environment (as `wh events` and `wh wait` clients of the app); they
#                      are ended when ttystorm is.
start_workload() {
  local name="$1"; shift
  dir="$scratch/$name"
  mkdir -p "$dir"
  cat > "$dir/shell.sh" <<SHELL
#!/bin/bash
export WORKLOAD_DIR="$dir"
while [ ! -e "$dir/go" ]; do sleep 0.1; done
${workload_watchers:-}
"$ttystorm" $* 2>"$dir/ttystorm.err"
kill \$(jobs -p) 2>/dev/null
exec /bin/cat
SHELL
  chmod +x "$dir/shell.sh"
  started+=("$dir")
  "$ROOT/scripts/devctl-instance.sh" --bin "$bin" --shell "$dir/shell.sh" "$dir" > "$dir/start.txt"
  ctl="$dir/ctl"
  pid="$(sed -n 's/^pid=//p' "$dir/start.txt")"
  send "$ctl" "onboarding close" "appearance dark" "resize $WINDOW_W $WINDOW_H" ${workload_setup[@]+"${workload_setup[@]}"}
  sleep 3
  : > "$dir/go"
}

stop_workload() {
  "$ROOT/scripts/devctl-instance.sh" stop "$1" >/dev/null 2>&1
}
