# Driving terminal apps, shared by scripts/terminal-compare.sh and scripts/suite-compare.sh. Source it
# (after lib.sh), don't run it.
#
# What it does: finds which of Wormhole, Ghostty, Alacritty, kitty, WezTerm, iTerm2 and Terminal are
# installed, starts one at a time with a throwaway configuration (see the callers' headers for what each
# app is given), has it run a program of the caller's in a window of a given size, waits for that program
# to end and stops what it started. Everything an app starts is stopped again by its PID; an app that has
# one process and is already running (iTerm2, Terminal) is skipped, never taken over.
#
# The caller sets before using it:  cols rows font font_size  (the terminal's size and font),
# wormhole_bin ("" to build HEAD), max_load, and optionally ghostty_args and iterm2_args (more options
# for those two apps).  It calls apps_scratch once, then app_run per terminal.
# shellcheck shell=bash disable=SC2034,SC2154  # the callers read what is set here and set the sizes, font and max_load

ALL_TERMINALS="wormhole ghostty alacritty kitty wezterm iterm2 terminal"

# ---------------------------------------------------------------- what is installed

ALACRITTY_BIN=""
for candidate in /Applications/Alacritty.app/Contents/MacOS/alacritty "$HOME/.cargo/bin/alacritty"; do
  if [ -x "$candidate" ]; then ALACRITTY_BIN="$candidate"; break; fi
done
GHOSTTY_BIN=/Applications/Ghostty.app/Contents/MacOS/ghostty
KITTY_BIN=/Applications/kitty.app/Contents/MacOS/kitty
WEZTERM_BIN=/Applications/WezTerm.app/Contents/MacOS/wezterm-gui
ITERM_APP=/Applications/iTerm.app
TERMINAL_APP=/System/Applications/Utilities/Terminal.app

# bundle_version <Info.plist>: the bundle's version text.
bundle_version() { plist_get "$1" :CFBundleShortVersionString; }

# running <process name>: it is running now (an exact name, as `ps` has it).
running() { pgrep -x "$1" >/dev/null 2>&1; }

# installed <terminal>: prints its version, and fails when it is not installed.
installed() {
  case "$1" in
    ghostty) [ -x "$GHOSTTY_BIN" ] && bundle_version /Applications/Ghostty.app/Contents/Info.plist ;;
    kitty) [ -x "$KITTY_BIN" ] && "$KITTY_BIN" --version | awk '{print $2}' ;;
    wezterm) [ -x "$WEZTERM_BIN" ] && /Applications/WezTerm.app/Contents/MacOS/wezterm --version | awk '{print $2}' ;;
    iterm2) [ -d "$ITERM_APP" ] && bundle_version "$ITERM_APP/Contents/Info.plist" ;;
    terminal) [ -d "$TERMINAL_APP" ] && bundle_version "$TERMINAL_APP/Contents/Info.plist" ;;
    alacritty) [ -n "$ALACRITTY_BIN" ] && "$ALACRITTY_BIN" --version | awk '{print $2}' ;;
    wormhole) echo "$(git -C "$ROOT" rev-parse --short HEAD) (this checkout)" ;;
  esac
}

# already_running <terminal>: the app has a process that was not started by this run, so a second
# one would have to share it (only the two AppleScript-driven apps need this to be false).
already_running() {
  case "$1" in
    iterm2) running iTerm2 ;;
    terminal) running Terminal ;;
    *) return 1 ;;
  esac
}

# list_terminals: say which terminals are installed and running (--list).
list_terminals() {
  local t v state
  for t in $ALL_TERMINALS; do
    if v="$(installed "$t" 2>/dev/null)"; then
      state=""
      case "$t" in
        iterm2|terminal) already_running "$t" && state=" (running now: would be skipped)" ;;
        ghostty) running ghostty && state=" (running now: a separate instance would be started)" ;;
        kitty) running kitty && state=" (running now: a separate instance would be started)" ;;
        wezterm) running wezterm-gui && state=" (running now: a separate instance would be started)" ;;
        alacritty) running alacritty && state=" (running now: a separate instance would be started)" ;;
      esac
      printf '%-10s %s%s\n' "$t" "$v" "$state"
    else
      printf '%-10s not installed\n' "$t"
    fi
  done
}

# check_terminals <list>: die unless every word is one of $ALL_TERMINALS.
check_terminals() {
  local t
  for t in $1; do
    case " $ALL_TERMINALS " in *" $t "*) ;; *) die "no terminal $t (they are: ${ALL_TERMINALS// /, })" ;; esac
  done
}

# machine_slug: the machine as a folder name, for the results.
machine_slug() {
  local id
  id="$(sysctl -n hw.model)-$(sysctl -n machdep.cpu.brand_string)"
  printf '%s' "$id" | tr -c 'A-Za-z0-9._-' '-' | sed 's/--*/-/g; s/-$//'
}

# ---------------------------------------------------------------- scratch dir and cleanup

# What this run started and must stop again: pids of processes it launched, Wormhole instances'
# scratch dirs, AppleScript windows ("app:id").
launched_pids=() instances=() windows=()

# apps_scratch <name>: make this run's scratch dir, $TMPDIR/<name>/<stamp>-<pid>, and have it removed
# (with everything this run started stopped) when the script ends. Sets $scratch and $stamp.
apps_scratch() {
  scratch_name="$1"
  stamp="$(date +%Y%m%d-%H%M%S)"
  local base_dir="${TMPDIR:-/tmp}"
  base_dir="${base_dir%/}/$scratch_name"
  mkdir -p "$base_dir"
  scratch="$(cd "$base_dir" && pwd -P)/$stamp-$$"
  mkdir -p "$scratch"
  case "$scratch/" in "$(cd "$ROOT" && pwd -P)"/*) die "the scratch dir must be outside the repository" ;; esac
  trap apps_cleanup EXIT
  trap 'exit 130' INT TERM
}

# shellcheck disable=SC2329  # runs from the EXIT trap
apps_cleanup() {
  stop_all
  # Exactly the dir this run made (never a glob).
  [[ "$scratch" == */$scratch_name/$stamp-$$ ]] && rm -rf "$scratch"
  return 0
}

# stop_all: close the windows, stop the instances and end the processes this run started.
stop_all() {
  local pid dir w
  for w in ${windows[@]+"${windows[@]}"}; do close_window "$w"; done
  for dir in ${instances[@]+"${instances[@]}"}; do
    "$ROOT/scripts/devctl-instance.sh" stop "$dir" >/dev/null 2>&1 || warn "could not stop the instance in $dir"
  done
  for pid in ${launched_pids[@]+"${launched_pids[@]}"}; do stop_pid "$pid"; done
  windows=() instances=() launched_pids=()
}

# stop_pid <pid>: end a process this run launched: TERM, then KILL after a few seconds.
stop_pid() {
  local pid="$1"
  kill -0 "$pid" 2>/dev/null || return 0
  kill "$pid" 2>/dev/null || true
  for _ in 1 2 3 4 5 6 7 8 9 10; do kill -0 "$pid" 2>/dev/null || return 0; sleep 0.5; done
  kill -9 "$pid" 2>/dev/null || true
}

# ---------------------------------------------------------------- driving one terminal

# osa <seconds> <script>: run AppleScript with a time limit (an unanswered Automation prompt would
# otherwise wait for good).
osa() {
  local seconds="$1" script="$2" pid result
  result="$scratch/osa.out"
  osascript -e "$script" > "$result" 2> "$scratch/osa.err" &
  pid=$!
  for _ in $(seq $((seconds * 5))); do
    if ! kill -0 "$pid" 2>/dev/null; then
      wait "$pid" || { cat "$scratch/osa.err" >&2; return 1; }
      cat "$result"
      return 0
    fi
    sleep 0.2
  done
  kill "$pid" 2>/dev/null || true
  echo "AppleScript did not answer in ${seconds}s (is there an Automation prompt waiting?)" >&2
  return 1
}

# close_window <app:id>: close the window this run opened (the id may be empty: it was not made yet)
# and quit the app, which was not running before (it is skipped otherwise).
close_window() {
  local app="${1%%:*}" id="${1#*:}"
  case "$app" in
    iterm2) [ -z "$id" ] || osa 20 "tell application \"iTerm\" to close (windows whose id is $id)" >/dev/null 2>&1 || true
            osa 20 'tell application "iTerm" to quit' >/dev/null 2>&1 || true ;;
    terminal) [ -z "$id" ] || osa 20 "tell application \"Terminal\" to close (windows whose id is $id) saving no" >/dev/null 2>&1 || true
              osa 20 'tell application "Terminal" to quit' >/dev/null 2>&1 || true ;;
  esac
}

# launch_app <app> <process name> [args...]: a new instance of the app, started through LaunchServices
# (`open -n`, as a click does: it comes to the front, has no controlling terminal, which ttystorm
# looks for in the terminal process, and none of this shell's variables). Sets $term_pid to the new
# process, found as the one of that name that was not there before.
launch_app() {
  local app="$1" proc="$2" before pid
  shift 2
  before=" $(pgrep -x "$proc" | tr '\n' ' ')"
  open -n -a "$app" --args "$@" || return 1
  for _ in $(seq 100); do
    for pid in $(pgrep -x "$proc"); do
      case "$before" in
        *" $pid "*) ;;
        *) term_pid="$pid"; launched_pids+=("$pid"); return 0 ;;
      esac
    done
    sleep 0.2
  done
  warn "$proc did not start"
  return 1
}

# alacritty_app: an Alacritty.app to launch: the installed one, or a bundle around the binary
# `cargo install` made (LaunchServices wants a bundle). Prints its path.
alacritty_app() {
  if [ -d /Applications/Alacritty.app ]; then echo /Applications/Alacritty.app; return; fi
  local app="$scratch/Alacritty.app"
  mkdir -p "$app/Contents/MacOS"
  cp "$ALACRITTY_BIN" "$app/Contents/MacOS/alacritty"
  cat > "$app/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
  <key>CFBundleExecutable</key><string>alacritty</string>
  <key>CFBundleIdentifier</key><string>app.getwormhole.terminal-compare.alacritty</string>
  <key>CFBundleName</key><string>Alacritty</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
  codesign --force --sign - "$app" >/dev/null 2>&1 || true
  echo "$app"
}

# write_runner <command line>: the program the terminal runs ($dir/run.sh): it waits for the go file
# (the window has its size and font by then), notes the size it sees, runs the command line (written
# as the shell should read it, quoting done) and says when it ended, with its exit status.
write_runner() {
  cat > "$dir/run.sh" <<EOF
#!/bin/bash
while [ ! -e "$dir/go" ]; do
  [ -d "$dir" ] || exit 1
  stty size > "$dir/size.tmp" 2>/dev/null && mv "$dir/size.tmp" "$dir/size"
  sleep 0.2
done
stty size > "$dir/size" 2>/dev/null
$1
echo \$? > "$dir/done"
EOF
  chmod +x "$dir/run.sh"
}

# frontmost_pid: the pid of the app in front (empty when unknown).
frontmost_pid() {
  lsappinfo info -only pid "$(lsappinfo front 2>/dev/null)" 2>/dev/null | sed -n 's/.*pid = \([0-9]*\).*/\1/p'
}

# wait_for <seconds> <command>...: poll until the command succeeds; fails when the time is up.
wait_for() {
  local seconds="$1"; shift
  for _ in $(seq $((seconds * 5))); do "$@" && return 0; sleep 0.2; done
  return 1
}
# shellcheck disable=SC2329  # runs through wait_for
exists() { [ -e "$1" ]; }

# start_<terminal>: open the app so that it runs $dir/run.sh in a window of $cols x $rows cells.

# $ghostty_args (words, optional): more Ghostty options. It stops with an error on an option it does not
# know, so a caller adds only what the installed version has.
start_ghostty() {
  # shellcheck disable=SC2086  # words on purpose
  launch_app /Applications/Ghostty.app ghostty --config-default-files=false --window-width="$cols" --window-height="$rows" \
    --font-size="$font_size" --font-family="$font" --command="$dir/run.sh" --confirm-close-surface=false \
    --quit-after-last-window-closed=true --window-save-state=never --wait-after-command=false ${ghostty_args:-}
}

start_kitty() {
  launch_app /Applications/kitty.app kitty --config NONE -o "initial_window_width=${cols}c" -o "initial_window_height=${rows}c" \
    -o "font_size=$font_size" -o "font_family=$font" -o remember_window_size=no -o confirm_os_window_close=0 \
    -o macos_quit_when_last_window_closed=yes -o shell_integration=disabled "$dir/run.sh"
}

start_wezterm() {
  echo 'return {}' > "$dir/wezterm.lua"
  launch_app /Applications/WezTerm.app wezterm-gui --config-file "$dir/wezterm.lua" --config "initial_cols=$cols" \
    --config "initial_rows=$rows" --config "font_size=$font_size" --config "font=wezterm.font(\"$font\")" \
    --config 'window_close_confirmation="NeverPrompt"' --config 'check_for_updates=false' \
    start --always-new-process -- "$dir/run.sh"
}

start_alacritty() {
  : > "$dir/alacritty.toml"
  launch_app "$(alacritty_app)" alacritty --config-file "$dir/alacritty.toml" -o "window.dimensions.columns=$cols" \
    -o "window.dimensions.lines=$rows" -o "font.size=$font_size" -o "font.normal.family=\"$font\"" \
    -e "$dir/run.sh"
}

start_iterm2() {
  local id
  # Started by this run, so quit again by it, whatever happens below.
  windows+=("iterm2:")
  # $iterm2_args (words, optional): preferences for this process alone, as `-Key value` on the command
  # line (macOS reads them before the stored defaults and writes nothing), so the app has to be started
  # with them before AppleScript opens its window.
  if [ -n "${iterm2_args:-}" ]; then
    # shellcheck disable=SC2086  # words on purpose
    open -a "$ITERM_APP" --args $iterm2_args || return 1
    wait_for 30 running iTerm2 || { echo "iTerm2 did not start" >&2; return 1; }
    sleep 2
  fi
  id="$(osa 60 "tell application \"iTerm\"
    activate
    set w to (create window with default profile)
    if w is missing value then error \"iTerm2 made no window: it may be waiting on a dialog, as a downloaded app is on its first launch, so open it once by hand\"
    delay 1
    tell current session of w
      set columns to $cols
      set rows to $rows
      write text \"exec $dir/run.sh\"
    end tell
    return id of w
  end tell")" || return 1
  windows=("${windows[@]/iterm2:/iterm2:$id}")
}

start_terminal() {
  local id
  windows+=("terminal:")
  id="$(osa 60 "tell application \"Terminal\"
    activate
    set t to do script \"exec $dir/run.sh\"
    set font name of t to \"$font\"
    set font size of t to $font_size
    set number of columns of t to $cols
    set number of rows of t to $rows
    return id of front window
  end tell")" || return 1
  windows=("${windows[@]/terminal:/terminal:$id}")
}

# start_wormhole: an isolated release build whose only terminal runs $dir/run.sh. The window is resized
# until the terminal has $cols x $rows cells: the cell size is not known in advance, so the size the
# runner sees (it writes it to $dir/size while it waits) says how far off the window is.
start_wormhole() {
  if [ -z "$wormhole_bin" ]; then
    log "building the release app (devctl) into $TARGET_DIR/compare-build"
    (cd "$ROOT" && CARGO_TARGET_DIR="$TARGET_DIR/compare-build" cargo build --release --features devctl --quiet)
    wormhole_bin="$TARGET_DIR/compare-build/release/$APP_BINARY"
  fi
  [ -x "$wormhole_bin" ] || die "no binary $wormhole_bin"
  instances+=("$dir/instance")
  # DECRQCRA, which esctest2 reads the screen back with, and the window operations (move, resize,
  # the title reports) it tests are off in Wormhole unless config.json asks (a program, also over
  # ssh, could read the screen cell by cell, or type its own title into other programs). They are
  # read when a terminal starts, so the keys are in the throwaway data dir before the instance and
  # its one terminal do.
  # The theme is xterm's own, black on white: esctest2 expects the dynamic colors a terminal starts with
  # to be xterm's defaults (ResetSpecialColor_Dynamic wants #000000 for the foreground), which is a
  # fact about the test's reference terminal, not something the engine should special-case.
  mkdir -p "$dir/instance/data/themes"
  printf '{\n  "name": "xterm defaults",\n  "terminal": { "background": "#ffffff", "foreground": "#000000" }\n}\n' \
    > "$dir/instance/data/themes/xterm-defaults.json"
  printf '{\n  "allow_screen_checksum": true,\n  "allow_window_ops": true,\n  "theme_mode": "dark",\n  "theme": "xterm defaults"\n}\n' \
    > "$dir/instance/data/config.json"
  "$ROOT/scripts/devctl-instance.sh" --bin "$wormhole_bin" --shell "$dir/run.sh" "$dir/instance" > "$dir/start.txt"
  local ctl="$dir/instance/ctl"
  term_pid="$(sed -n 's/^pid=//p' "$dir/start.txt")"
  send "$ctl" "onboarding close" "appearance dark" "set font_family=\"$font\"" "set font_size=$font_size"
  sleep 2
  local w=1400 h=900 got_cols got_rows
  for _ in 1 2 3 4 5 6 7 8; do
    send "$ctl" "resize $w $h"
    sleep 1.5
    read -r got_rows got_cols < "$dir/size" 2>/dev/null || continue
    if [ "$got_cols" = "$cols" ] && [ "$got_rows" = "$rows" ]; then
      send "$ctl" front
      return 0
    fi
    # One cell is about window / cells wide; move by the missing cells.
    w=$(( w + (cols - got_cols) * (w / got_cols) ))
    h=$(( h + (rows - got_rows) * (h / got_rows) ))
  done
  warn "wormhole: the terminal is ${got_cols:-?} x ${got_rows:-?} cells, not $cols x $rows"
  send "$ctl" front
}

# send <ctl dir> <command>...: one batch for the instance's next tick (written whole, then moved).
send() {
  local ctl="$1"; shift
  local waited=0
  while [ -e "$ctl/cmd" ] && [ "$waited" -lt 100 ]; do sleep 0.1; waited=$((waited + 1)); done
  printf '%s\n' "$@" > "$ctl/cmd.tmp"
  mv "$ctl/cmd.tmp" "$ctl/cmd"
}

# ---------------------------------------------------------------- run one terminal

# load1: the 1-minute load average.
load1() { sysctl -n vm.loadavg | awk '{print $2}'; }

# wait_quiet: return when the load is below $max_load, or after 5 minutes with a warning.
wait_quiet() {
  local waited=0
  while ! awk -v l="$(load1)" -v m="$max_load" 'BEGIN { exit !(l < m) }'; do
    if [ "$waited" -ge 300 ]; then warn "the load average is still $(load1): going on, the numbers carry it"; return 0; fi
    [ "$waited" -gt 0 ] || log "waiting for the load average ($(load1)) to fall below $max_load"
    sleep 10
    waited=$((waited + 10))
  done
}

# app_run <terminal> <tag> <command line> <seconds>: start the terminal, have it run the command line
# (see write_runner; its scratch dir is $scratch/<tag><terminal>) and wait up to <seconds> for it to
# end, then stop what was started. Sets:
#   run_state    ok (the command ended: its exit status is $run_exit), failed or skipped
#   run_note     why, unless ok
#   run_version  the app's version; run_seen  "rows cols" the command saw; run_in_front  yes or no
#   run_load     the 1-minute load average when it started
app_run() {
  local t="$1" tag="$2" cmd="$3" limit="$4" started_ok=1 why
  run_state=failed run_note="" run_version="" run_seen="?" run_in_front=yes run_load="" run_exit=""
  if ! run_version="$(installed "$t" 2>/dev/null)"; then
    warn "$t: not installed, skipped"
    run_state=skipped run_note="not installed"
    return 0
  fi
  if already_running "$t"; then
    warn "$t: already running (yours), skipped: it has one process and can't be driven without taking it over"
    run_state=skipped run_note="already running"
    return 0
  fi
  wait_quiet
  run_load="$(load1)"
  dir="$scratch/$tag$t"
  mkdir -p "$dir"
  write_runner "$cmd"
  log "$t $run_version: ${cols}x${rows}, $font $font_size"
  term_pid=""
  : > "$scratch/osa.err"
  "start_$t" || started_ok=0
  # The two AppleScript apps have one process, started by the call above (they were not running):
  # kept to stop it if the app asks to confirm quitting, as iTerm2 does by default.
  case "$t" in
    iterm2) term_pid="$(pgrep -x iTerm2 | head -n1 || true)" ;;
    terminal) term_pid="$(pgrep -x Terminal | head -n1 || true)" ;;
  esac
  [ -z "$term_pid" ] || launched_pids+=("$term_pid")
  if [ "$started_ok" = 0 ]; then
    warn "$t: could not be started"
    # Read why before stopping the app: quitting it is an AppleScript call that overwrites the file.
    why="$(tr '\n|' '  ' < "$scratch/osa.err" | cut -c1-240)"
    stop_all
    run_note="could not be started: $why"
    return 0
  fi
  # Let the window open and take its size before the command reads it.
  sleep 4
  # A window nobody can see is throttled by macOS, and something else (a browser, you) may have taken
  # the front since the app opened: say so when it is not this terminal, the numbers may suffer.
  if [ -n "$term_pid" ] && [ "$(frontmost_pid)" != "$term_pid" ]; then
    run_in_front=no
    warn "$t: another app is in front (pid $(frontmost_pid)); the window may be covered"
  fi
  : > "$dir/go"
  if wait_for "$limit" exists "$dir/done"; then
    run_state=ok
    run_exit="$(cat "$dir/done")"
    run_seen="$(tr ' ' 'x' < "$dir/size" 2>/dev/null || echo '?')"
  else
    warn "$t: no result within ${limit}s"
    run_note="no result within ${limit}s"
  fi
  # Stop what this terminal started before the next one opens.
  stop_all
  sleep 2
}
