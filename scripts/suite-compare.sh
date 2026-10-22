#!/usr/bin/env bash
# Outside conformance suites run inside each installed terminal app, one at a time, and one markdown
# table per suite: a second matrix next to tools/vt-compare's own (which we wrote and pass 100%).
#
#   esctest2    Thomas Dickey's continuation of George Nachman's esctest: ~560 automated tests of escape
#               sequences against xterm's behaviour. It reads the screen back with DECRQCRA checksums,
#               one cell at a time, so a terminal that does not answer those can only be measured on the
#               tests that never look at the screen (the table says how many that is).
#   ucs-detect  Jeff Quast's measurement of how a terminal sizes Unicode (wide, ZWJ emoji, VS16, VS15,
#               flags, combining, languages), with cursor position reports. ucs-detect publishes results
#               for many terminals; the table shows them next to this run's for the same terminal.
#
# WARNING: this opens windows on screen, one after the other, for several minutes each (twice per app:
# a fresh window per suite). Leave the keyboard and the mouse alone, keep the display awake and close
# what is busy, as for scripts/terminal-compare.sh, whose launching code this shares
# (scripts/terminal-apps.sh).
#
#   scripts/suite-compare.sh [options]
#     --only <a,b>      wormhole ghostty alacritty kitty wezterm iterm2 terminal (default: every one
#                       that is installed)
#     --suites <a,b>    esctest ucs-detect (default both)
#     --probe-only      only ask each app which of the reports esctest2 needs it answers (about 20 s
#                       per app), to see what can be measured before a full run; implies --suites esctest
#     --out <dir>       where the results go (default tools/vt-compare/results/<date>-<machine>/suites/)
#     --cols <n> --rows <n>     the terminal's size in cells (default 80 x 25: esctest2 resets to it)
#     --font <name> --font-size <n>   where the app lets a command line set them (default Menlo 13)
#     --esctest-include <regex>   only the esctest2 tests whose Class.test_name matches (for example
#                       "^(CUP|ECH)Tests"; default all)
#     --esctest-args "<args>"   more arguments for tools/vt-compare/suites/esctest_run.py, as shell text
#                       (quote inside, for example "--clipboard"; `esctest_run.py --help` lists them)
#     --ucs-args "<args>"       more arguments for ucs-detect, as shell text (for example "--all", the
#                       whole Unicode tables and not the contested sequences: much slower)
#     --iterm2-args "<args>"  preferences for the iTerm2 process alone, as `-Key value` words (default
#                       "-NoSyncDisableDECRQCRA NO": iTerm2 answers DECRQCRA only when that advanced
#                       setting is off, and esctest2 reads the screen with it; the command line sets it
#                       for this process and writes nothing; "" to leave iTerm2 as it is)
#     --wormhole-bin <path>   a release build of Wormhole with devctl instead of building HEAD
#                       (its throwaway config turns `allow_screen_checksum` on: Wormhole answers DECRQCRA only
#                       then, and esctest2 reads the screen with it; `allow_window_ops`, for the window
#                       operations, and `allow_title_report`, for the title reports esctest2 tests)
#     --timeout <s>     how long one suite may take in one terminal (default 1800)
#     --max-load <n>    wait (up to 5 minutes) before each terminal until the 1-minute load average
#                       is below this (default 4)
#     --report-only     make the tables again from what --out holds, run nothing
#     --list            say which terminals are installed and running, and stop
#     -h, --help
#
# tools/vt-compare is the terminal engine's (wormhole-terminal/wormhole-vt): a checkout of it next to
# this one (../wormhole-vt), or VT_COMPARE=<its tools/vt-compare>. The suites are fetched at the commits
# in its suites.pin into the cache (its fetch-suites.sh), never into either repository. Wormhole needs
# --wormhole-bin to be a release build with devctl; without it HEAD is built (several minutes).
#
# Results in <out>: esctest/<terminal>.jsonl (+ .log, .log.trace), ucs-detect/<terminal>.yaml, runs.tsv
# and meta.tsv (how each run went), and the tables: esctest.md, esctest-tests.md, ucs-detect.md, run.md.
# An app that cannot be started (iTerm2 or Terminal refusing Apple events, an app that is not installed,
# one that is already running with a single process) is reported in run.md and the others go on.
#
# What it touches: as scripts/terminal-compare.sh (apps it starts are stopped by PID; a scratch dir under
# $TMPDIR, removed at the end; nothing of your own data, config or shell). The suites' sequences run in
# the apps' own throwaway windows: esctest2 sets the title, the colors and tab stops, asks the window to
# resize, move and minimize (xterm's window operations), and would write the clipboard (those tests are
# skipped unless --esctest-args "--clipboard"). Python from the suites' venv in the cache runs them.
set -euo pipefail
# shellcheck disable=SC2034,SC2154  # the variables this script and terminal-apps.sh share
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"
# shellcheck source=terminal-apps.sh
. "$(dirname "$0")/terminal-apps.sh"

VT_COMPARE="${VT_COMPARE:-$ROOT/../wormhole-vt/tools/vt-compare}"
[ -d "$VT_COMPARE/suites" ] || die "no tools/vt-compare at $VT_COMPARE: clone wormhole-terminal/wormhole-vt next to this checkout, or set VT_COMPARE"
SUITES_DIR="$VT_COMPARE/suites"
max_load=4 only="" out="" cols=80 rows=25 font=Menlo font_size=13 wormhole_bin="" timeout_s=1800 list=0
iterm2_args="-NoSyncDisableDECRQCRA NO"
suites="esctest ucs-detect" esctest_include=".*" esctest_args="" ucs_args="" report_only=0 probe_only=0
while [ $# -gt 0 ]; do
  case "$1" in
    --only) only="${2:?--only needs a list}"; shift ;;
    --suites) suites="${2:?--suites needs a list}"; suites="${suites//,/ }"; shift ;;
    --probe-only) probe_only=1 ;;
    --out) out="${2:?--out needs a folder}"; shift ;;
    --cols) cols="${2:?--cols needs a number}"; shift ;;
    --rows) rows="${2:?--rows needs a number}"; shift ;;
    --font) font="${2:?--font needs a name}"; shift ;;
    --font-size) font_size="${2:?--font-size needs a number}"; shift ;;
    --esctest-include) esctest_include="${2:?--esctest-include needs a regex}"; shift ;;
    --esctest-args) esctest_args="${2:?--esctest-args needs arguments}"; shift ;;
    --ucs-args) ucs_args="${2:?--ucs-args needs arguments}"; shift ;;
    --iterm2-args) iterm2_args="${2-}"; [ $# -ge 2 ] || die "--iterm2-args needs a value"; shift ;;
    --wormhole-bin) wormhole_bin="${2:?--wormhole-bin needs a path}"; shift ;;
    --timeout) timeout_s="${2:?--timeout needs seconds}"; shift ;;
    --max-load) max_load="${2:?--max-load needs a number}"; shift ;;
    --report-only) report_only=1 ;;
    --list) list=1 ;;
    -h|--help) sed -n '2,/^set -euo/p' "$0" | sed '$d'; exit 0 ;;
    -*) die "unknown option: $1 (see --help)" ;;
    *) die "unexpected argument: $1" ;;
  esac
  shift
done
[ "$probe_only" = 0 ] || suites="esctest"
for s in $suites; do
  case "$s" in esctest|ucs-detect) ;; *) die "no suite $s (they are: esctest, ucs-detect)" ;; esac
done
terminals="$ALL_TERMINALS"
if [ -n "$only" ]; then
  terminals="${only//,/ }"
  check_terminals "$terminals"
fi
[ "$(uname -s)" = Darwin ] || die "this drives macOS apps and needs a display"
if [ "$list" = 1 ]; then
  list_terminals
  exit 0
fi

# ---------------------------------------------------------------- the suites, results folder

# fetch-suites.sh fetches what is missing and prints where everything is.
SUITES_PYTHON="" ESCTEST_DIR="" UCS_DETECT_DIR=""
env_lines="$("$VT_COMPARE/fetch-suites.sh" --env)"
while IFS='=' read -r key value; do
  case "$key" in
    ESCTEST_DIR) ESCTEST_DIR="$value" ;;
    UCS_DETECT_DIR) UCS_DETECT_DIR="$value" ;;
    SUITES_PYTHON) SUITES_PYTHON="$value" ;;
  esac
done <<<"$env_lines"
[ -d "$ESCTEST_DIR/esctest" ] && [ -d "$UCS_DETECT_DIR/ucs_detect" ] && [ -x "$SUITES_PYTHON" ] ||
  die "fetch-suites.sh did not leave the suites in the cache"
# Ghostty answers DECRQCRA from 1.4 on, and only when asked to (the older ones stop on the option).
ghostty_args=""
ghostty_version="$(installed ghostty 2>/dev/null || true)"
if [ -n "$ghostty_version" ] && [ "$(printf '%s\n1.4.0\n' "${ghostty_version%%-*}" | sort -V | head -n1)" = 1.4.0 ]; then
  ghostty_args="--vt-xt-checksum-report=true"
fi
pin() { sed -n "s/^$1=//p" "$VT_COMPARE/suites.pin" | head -n1; }

[ -n "$out" ] || out="$VT_COMPARE/results/$(date +%F)-$(machine_slug)/suites"
mkdir -p "$out/esctest" "$out/ucs-detect"
out="$(cd "$out" && pwd -P)"

make_report() {
  "$SUITES_PYTHON" -I "$SUITES_DIR/report.py" --out "$out" --ucs-detect "$UCS_DETECT_DIR"
}

if [ "$report_only" = 1 ]; then
  make_report
  exit 0
fi
need osascript

apps_scratch wormhole-suite-compare

# ---------------------------------------------------------------- record keeping

# record <suite> <terminal> <state> <note> <version> <seen> <in front> <load> <exit> <seconds>: one line
# in runs.tsv, replacing an earlier run of the same suite in the same terminal.
record() {
  local f="$out/runs.tsv" tmp="$scratch/runs.tsv"
  if [ -f "$f" ]; then awk -F'\t' -v s="$1" -v t="$2" '!($1 == s && $2 == t)' "$f" > "$tmp"; else : > "$tmp"; fi
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$1" "$2" "$3" "$(printf '%s' "$4" | tr '\t\n' '  ')" \
    "$5" "$6" "$7" "$8" "$9" "${10}" >> "$tmp"
  mv "$tmp" "$f"
}

load_start="$(uptime | sed 's/.*load averages*: //')"
{
  printf 'date\t%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'machine\t%s, %s, macOS %s\n' "$(sysctl -n hw.model)" "$(sysctl -n machdep.cpu.brand_string)" "$(sw_vers -productVersion)"
  printf 'checkout\t%s%s\n' "$(git -C "$ROOT" rev-parse --short HEAD)" \
    "$([ -z "$(git -C "$ROOT" status --porcelain --untracked-files=no)" ] || echo ' (with uncommitted changes)')"
  printf 'asked_for\t%s x %s cells, %s %s pt (an app that can'"'"'t be told from the command line has its own)\n' \
    "$cols" "$rows" "$font" "$font_size"
  printf 'load\t%s\n' "$load_start"
  printf 'esctest_rev\t%s\n' "$(pin esctest2_rev)"
  printf 'ucs_detect_rev\t%s\n' "$(pin ucs_detect_rev)"
} > "$out/meta.tsv"
log "load average at the start: $load_start (anything busy in the background slows the replies)"

# ---------------------------------------------------------------- run every suite in every terminal

# run_one <suite> <terminal>: one window, one suite; records how it went.
run_one() {
  local suite="$1" t="$2" cmd result start tag version name="$2"
  version="$(installed "$t" 2>/dev/null || true)"
  case "$suite" in
    esctest)
      tag="esctest-"
      result="$out/esctest/$t.jsonl"
      rm -f "$result" "$out/esctest/$t.log" "$out/esctest/$t.log.trace"
      cmd="cd \"$scratch/$tag$t\" && PYTHONDONTWRITEBYTECODE=1 \"$SUITES_PYTHON\" \"$SUITES_DIR/esctest_run.py\" \
--esctest \"$ESCTEST_DIR/esctest\" --out \"$result\" --log \"$out/esctest/$t.log\" --label \"$t\" \
--include $(printf '%q' "$esctest_include") $([ "$probe_only" = 0 ] || echo --probe-only) $esctest_args 2> \"$out/esctest/$t.err\""
      ;;
    ucs-detect)
      tag="ucs-"
      result="$out/ucs-detect/$t.yaml"
      rm -f "$result"
      # ucs-detect talks to stderr (its default stream), so that is left alone.
      cmd="cd \"$scratch/$tag$t\" && PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=\"$UCS_DETECT_DIR\" \"$SUITES_PYTHON\" \
-c 'import ucs_detect; ucs_detect.main()' --save-yaml \"$result\" --set-software-name \"$name\" \
--set-software-version \"${version:-unknown}\" --no-final-summary $ucs_args"
      ;;
  esac
  log "$suite in $t"
  start=$SECONDS
  app_run "$t" "$tag" "$cmd" "$timeout_s"
  local state="$run_state" note="$run_note"
  if [ "$state" = ok ]; then
    case "$suite" in
      esctest)
        if [ "$probe_only" = 1 ]; then
          [ -s "$result" ] || { state=failed; note="the probe wrote nothing (exit $run_exit): $(head -c 200 "$out/esctest/$t.err" 2>/dev/null | tr '\n' ' ')"; }
        elif ! grep -q '"type": "summary"' "$result" 2>/dev/null; then
          state=failed
          note="esctest did not finish (exit $run_exit): $(tail -n 3 "$out/esctest/$t.err" 2>/dev/null | tr '\n' ' ' | cut -c1-200)"
        fi ;;
      ucs-detect)
        [ -s "$result" ] || { state=failed; note="ucs-detect saved nothing (exit $run_exit)"; } ;;
    esac
  fi
  [ "$state" = ok ] || warn "$t: $suite: $state: $note"
  record "$suite" "$t" "$state" "$note" "$run_version" "$run_seen" "$run_in_front" "$run_load" "$run_exit" "$((SECONDS - start))"
}

for suite in $suites; do
  for t in $terminals; do
    run_one "$suite" "$t"
  done
done

# ---------------------------------------------------------------- tables

make_report
echo
for f in esctest ucs-detect; do
  [ ! -f "$out/$f.md" ] || { cat "$out/$f.md"; echo; }
done
log "results: $out"
