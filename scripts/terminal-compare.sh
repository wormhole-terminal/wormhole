#!/usr/bin/env bash
# End-to-end comparison of terminal apps: `ttystorm bench --save` run inside each installed terminal,
# one at a time, then `ttystorm compare --markdown` over the results.
#
# WARNING: this opens windows on screen, one after the other, for several minutes each. Leave the
# keyboard and the mouse alone, keep the display awake (a window nobody can see is throttled by
# macOS) and close what is busy: a build running in the background shows in every number. The
# load average at the start is printed and kept with the results.
#
#   scripts/terminal-compare.sh [options]
#     --only <a,b>      ghostty kitty wezterm iterm2 terminal alacritty wormhole (default: every
#                       one that is installed)
#     --out <dir>       where the results go (default tools/bench/results/<date>-<machine>/)
#     --cols <n> --rows <n>     the terminal's size in cells (default 120 x 40)
#     --font <name>     the font family where the app lets a command line set it (default Menlo)
#     --font-size <n>   the font size in points, likewise (default 13)
#     --bench "<args>"  more arguments for `ttystorm bench` (for example "--only realistic --runs 1")
#     --wormhole-bin <path>   a release build of Wormhole with devctl instead of building HEAD
#     --timeout <s>     how long one terminal may take (default 1500)
#     --max-load <n>    wait (up to 5 minutes) before each terminal until the 1-minute load average
#                       is below this (default 4; the machine has other things to do, and they
#                       show in every number)
#     --list            say which terminals are installed and running, and stop
#     -h, --help
#
# What each app is given (anything else is the app's own default; run.md in the results says what
# each actually got, and ttystorm's result files say the size in cells):
#   ghostty    a new instance (`open -n`): --window-width/--window-height (cells), --font-size,
#              --font-family, --command, no user config files
#   kitty      a new instance, --config NONE: initial_window_width/height in cells, font_size,
#              font_family
#   wezterm    a new instance: --config-file with nothing in it, initial_cols, initial_rows,
#              font_size, font
#   alacritty  a new instance (`cargo install --locked alacritty`, or an Alacritty.app), no user
#              config: window.dimensions in cells, font.size, font.normal.family. A cargo-installed
#              binary is wrapped in a bundle in the scratch dir so LaunchServices can start it
#   iterm2     one window opened by AppleScript with the default profile, its columns and rows set;
#              the font is the profile's (AppleScript can't set it). iTerm2 has one process, so it is
#              skipped when it is already running
#   terminal   one window opened by AppleScript: number of columns and rows, font name and size of the
#              tab. Terminal.app has one process, so it is skipped when it is already running
#   wormhole   a release build of this checkout (with devctl) started isolated by
#              scripts/devctl-instance.sh: its own data dir, HOME and shell, never yours; font and
#              size from its config, the window resized until the terminal has the cells asked for
# AppleScript needs the Automation permission of the app this script runs in, for iTerm2 and
# Terminal; macOS asks the first time, and an answer nobody gives makes that one time out and
# be reported as failed. A downloaded iTerm2 that was never opened waits on a dialog on its first
# launch and makes no window: open it once by hand.
#
# What it touches. Apps: it starts each one's own new process and stops that process again by its PID
# (a TERM, then a KILL), never one you had open; the two apps with a single process are skipped when
# running, and the one this script starts is quit afterwards. Files: a scratch dir under $TMPDIR (the
# apps' run scripts and state, removed at the end), a release build of Wormhole in
# $CARGO_TARGET_DIR/compare-build, ttystorm built in tools/bench/target, and the results in --out.
# Nothing of your own data, config or shell is read or written, and no app's preferences are changed.
# macOS may keep a window from coming to the front of the app that runs this script; run.md says
# for each terminal whether it was in front when its benchmark started.
set -euo pipefail
# shellcheck disable=SC2034,SC2154  # the variables this script and terminal-apps.sh share
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"
# shellcheck source=terminal-apps.sh
. "$(dirname "$0")/terminal-apps.sh"

max_load=4 only="" out="" cols=120 rows=40 font=Menlo font_size=13 bench_args="" wormhole_bin="" timeout_s=1500 list=0
while [ $# -gt 0 ]; do
  case "$1" in
    --only) only="${2:?--only needs a list}"; shift ;;
    --out) out="${2:?--out needs a folder}"; shift ;;
    --cols) cols="${2:?--cols needs a number}"; shift ;;
    --rows) rows="${2:?--rows needs a number}"; shift ;;
    --font) font="${2:?--font needs a name}"; shift ;;
    --font-size) font_size="${2:?--font-size needs a number}"; shift ;;
    --bench) bench_args="${2:?--bench needs arguments}"; shift ;;
    --wormhole-bin) wormhole_bin="${2:?--wormhole-bin needs a path}"; shift ;;
    --timeout) timeout_s="${2:?--timeout needs seconds}"; shift ;;
    --max-load) max_load="${2:?--max-load needs a number}"; shift ;;
    --list) list=1 ;;
    -h|--help) sed -n '2,/^set -euo/p' "$0" | sed '$d'; exit 0 ;;
    -*) die "unknown option: $1 (see --help)" ;;
    *) die "unexpected argument: $1" ;;
  esac
  shift
done
[ "$(uname -s)" = Darwin ] || die "this compares macOS apps and needs a display"
need python3; need osascript
terminals="$ALL_TERMINALS"
if [ -n "$only" ]; then
  terminals="${only//,/ }"
  check_terminals "$terminals"
fi

if [ "$list" = 1 ]; then
  list_terminals
  exit 0
fi

# ---------------------------------------------------------------- scratch dir, results, cleanup

machine_id="$(machine_slug)"
[ -n "$out" ] || out="$ROOT/tools/bench/results/$(date +%F)-$machine_id"
mkdir -p "$out"
out="$(cd "$out" && pwd -P)"

apps_scratch wormhole-terminal-compare

# ---------------------------------------------------------------- building the tools

log "building ttystorm"
(cd "$ROOT/tools/bench" && cargo build --release --quiet)
ttystorm="$ROOT/tools/bench/target/release/ttystorm"
[ -x "$ttystorm" ] || die "no $ttystorm"

# ---------------------------------------------------------------- run every terminal

load_start="$(uptime | sed 's/.*load averages*: //')"
log "load average at the start: $load_start (anything busy in the background is in every number)"
status_lines=() ran=()
for t in $terminals; do
  rm -f "$out/$t.json"
  app_run "$t" "" "\"$ttystorm\" bench $bench_args --save \"$out/$t.json\" --label \"$t\" 2> \"$scratch/$t/ttystorm.err\"" "$timeout_s"
  case "$run_state" in
    skipped) status_lines+=("$t|skipped|$run_note"); continue ;;
    failed) status_lines+=("$t|failed|$run_note"); continue ;;
  esac
  if [ ! -s "$out/$t.json" ]; then
    warn "$t: ttystorm ended with $run_exit and saved nothing: $(head -c 300 "$scratch/$t/ttystorm.err" 2>/dev/null)"
    status_lines+=("$t|failed|ttystorm saved nothing")
  else
    log "$t: done (the runner saw $run_seen rows x cols)"
    status_lines+=("$t|ok|$run_version|$run_seen|$run_in_front|$run_load")
    ran+=("$t")
  fi
done

# ---------------------------------------------------------------- report

files=()
for t in $ALL_TERMINALS; do
  for r in ${ran[@]+"${ran[@]}"}; do
    if [ "$r" = "$t" ]; then files+=("$out/$t.json"); fi
  done
done

{
  echo "# Terminal comparison"
  echo
  echo "\`ttystorm bench\` (ttystorm $(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/tools/bench/Cargo.toml" | head -n1)) run inside each app by scripts/terminal-compare.sh."
  echo
  echo "- date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "- machine: $(sysctl -n hw.model), $(sysctl -n machdep.cpu.brand_string), macOS $(sw_vers -productVersion)"
  echo "- load average at the start: $load_start"
  echo "- checkout: $(git -C "$ROOT" rev-parse --short HEAD)$([ -z "$(git -C "$ROOT" status --porcelain --untracked-files=no)" ] || echo ' (with uncommitted changes)')"
  echo "- asked for: $cols x $rows cells, $font $font_size pt (an app that can't be told from the command line has its own)"
  [ -z "$bench_args" ] || echo "- extra ttystorm arguments: $bench_args"
  echo
  echo "| terminal | result | version | rows x cols seen | in front when it started | load average (1 min) when it started |"
  echo "|---|---|---|---|---|---|"
  for line in "${status_lines[@]+"${status_lines[@]}"}"; do
    IFS='|' read -r name state a b c d <<<"$line"
    if [ "$state" = ok ]; then echo "| $name | ok | $a | ${b// /x} | $c | $d |"; else echo "| $name | $state: $a | | | | |"; fi
  done
  echo
  echo "Per app: ghostty, kitty, wezterm and alacritty ran as a separate process each, with their user"
  echo "configuration left out, size and font from their command line; iterm2 and terminal were driven by"
  echo "AppleScript (columns and rows, and for terminal the font) and use their profile's font otherwise;"
  echo "wormhole is a release build of this checkout in an isolated instance, font and size from its config."
} > "$out/run.md"

if [ ${#files[@]} -ge 2 ]; then
  "$ttystorm" compare --markdown "${files[@]}" > "$out/compare.md"
  echo
  cat "$out/compare.md"
  echo
  log "results: $out"
else
  warn "fewer than two terminals produced a result: nothing to compare"
  [ ${#files[@]} -eq 0 ] || log "the one result: ${files[0]}"
fi
