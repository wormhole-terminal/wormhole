#!/bin/bash
# Records the raw output of a real command with macOS `script`, to measure and compare terminal
# engines on what programs really send (vim, less, htop, tmux, `git log --color`, a coding agent's
# TUI) next to the generated streams. Not run by anything: it needs a terminal, and the result is
# reviewed by hand before it is committed.
#
#   scripts/vt-record.sh [options] <name> -- <command> [args...]
#     --size <COLSxROWS>   the terminal size the command sees (default 100x30)
#     --term <TERM>        the TERM the command sees (default xterm-256color)
#     --dir <dir>          where to put it (default tools/vt-corpus)
#     --force              replace an existing recording of that name
#     --show <name>        play a recording back to this terminal (cat) instead of recording
#     -h, --help
#
# Writes <dir>/<name>.raw (every byte the command wrote to its terminal, with the line ends the
# terminal driver made: CR LF) and <dir>/<name>.meta (the size, TERM, command, date and macOS
# version, one `key=value` per line). Run it from a terminal, give interactive programs the keys
# you want in the recording, quit them as you normally do, and the recording ends with the command.
#
#   scripts/vt-record.sh git-log -- git -C ~/some/repo log --color --stat -n 30
#   scripts/vt-record.sh --size 120x40 htop -- htop
#
# Read the file before committing it: it holds whatever was on the screen (paths, host names, file
# contents, tokens a command printed). `vtbench` measures every *.raw in tools/vt-corpus on its own.
set -euo pipefail
. "$(dirname "$0")/lib.sh"

size=100x30
term=xterm-256color
dir="$ROOT/tools/vt-corpus"
force=0
show=""
while [ $# -gt 0 ]; do
  case "$1" in
    --size) size="${2:?--size needs COLSxROWS}"; shift 2 ;;
    --term) term="${2:?--term needs a value}"; shift 2 ;;
    --dir) dir="${2:?--dir needs a folder}"; shift 2 ;;
    --force) force=1; shift ;;
    --show) show="${2:?--show needs a name}"; shift 2 ;;
    -h|--help) sed -n '2,25p' "$0"; exit 0 ;;
    --) break ;;
    -*) die "unknown option: $1 (see --help)" ;;
    *) break ;;
  esac
done

valid_name() { [[ "$1" =~ ^[a-z0-9][a-z0-9-]*$ ]] || die "the name '$1' must be lowercase letters, digits and dashes"; }

if [ -n "$show" ]; then
  valid_name "$show"
  [ -f "$dir/$show.raw" ] || die "no recording $dir/$show.raw"
  cat "$dir/$show.raw"
  exit 0
fi

name="${1:-}"
[ -n "$name" ] && [ "${2:-}" = "--" ] || die "usage: scripts/vt-record.sh [options] <name> -- <command> [args...]"
valid_name "$name"
shift 2
[ $# -gt 0 ] || die "no command to record after --"
[[ "$size" =~ ^[1-9][0-9]*x[1-9][0-9]*$ ]] || die "the size '$size' must look like 100x30"
cols="${size%x*}"
rows="${size#*x}"
[ -t 0 ] || die "run this from a terminal: script needs one to give the command a pty of its own"
need script "script comes with macOS"

mkdir -p "$dir"
raw="$dir/$name.raw"
meta="$dir/$name.meta"
if [ -e "$raw" ] && [ "$force" -ne 1 ]; then
  die "$raw exists (use --force to replace it)"
fi

log "recording '$*' at ${cols}x${rows}, TERM=$term into $raw"
# stty sets the pty's size before the command starts; -t 0 makes script write every byte at once.
TERM="$term" COLUMNS="$cols" LINES="$rows" \
  script -q -t 0 "$raw" sh -c 'stty cols "$1" rows "$2"; shift 2; exec "$@"' sh "$cols" "$rows" "$@" || true

[ -s "$raw" ] || die "nothing was recorded"
{
  echo "cols=$cols"
  echo "rows=$rows"
  echo "term=$term"
  printf 'command='
  printf '%q ' "$@"
  echo
  echo "recorded=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "os=$(sw_vers -productVersion 2>/dev/null || uname -sr)"
  echo "bytes=$(wc -c < "$raw" | tr -d ' ')"
} > "$meta"
log "recorded $(wc -c < "$raw" | tr -d ' ') bytes; read $raw before committing it (it holds what was on screen)"
