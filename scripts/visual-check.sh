#!/usr/bin/env bash
# Visual check: run devctl scenarios in isolated Wormhole instances and collect screenshots that
# are the same on every run, optionally next to another revision's.
#
#   scripts/visual-check.sh [options] <scenario>...   names in tools/visual/scenarios, a path, or `all`
#     --out <dir>       where results go (default: $TMPDIR/wormhole-visual/<time>); not in the repo
#     --against <rev>   also run them on <rev> (built once in a temporary worktree, cached by commit)
#                       and diff every screenshot, branch against rev
#     --repeat          run them twice on this tree and diff the two runs (a determinism check;
#                       exits with 1 when they differ)
#     --no-build        use the binary already built in the target dir
#
# Every scenario gets its own instance (lib.sh's start_isolated): a scratch data dir, HOME and agent
# home, the Finder's PATH and no agent counted as installed but a `@shell` copy, /bin/cat (or that
# copy, named like an agent) for a shell, Touch ID answered "cancel", notifications logged rather
# than shown, background mode (behind your windows, never taking the focus; a revision from before
# it comes to the front), and the first terminal in the scenario's fixture repository.
# The script stops only the processes it started, by PID, and deletes nothing but the temporary
# worktree of --against. Format and examples: tools/visual/README.md.
set -euo pipefail
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"

VISUAL="$ROOT/tools/visual"
BATCH_TIMEOUT=60   # seconds a batch may take (a wait-idle gives up after 5 s on its own)

out="" against="" repeat=0 build=1
scenarios=()
while [ $# -gt 0 ]; do
  case "$1" in
    --out) out="${2:?--out needs a directory}"; shift ;;
    --against) against="${2:?--against needs a revision}"; shift ;;
    --repeat) repeat=1 ;;
    --no-build) build=0 ;;
    -h|--help) sed -n '2,18p' "$0"; exit 0 ;;
    -*) die "unknown option: $1" ;;
    *) scenarios+=("$1") ;;
  esac
  shift
done
[ ${#scenarios[@]} -gt 0 ] || die "name a scenario (tools/visual/scenarios), or \`all\`"
[ "$(uname -s)" = Darwin ] || die "the visual check needs macOS and a display"
need cargo; need git; need codesign; need sips

if [ "${scenarios[0]}" = all ]; then
  scenarios=()
  for file in "$VISUAL"/scenarios/*.txt; do scenarios+=("$file"); done
fi
files=()
for name in "${scenarios[@]}"; do
  if [ -f "$name" ]; then files+=("$name")
  elif [ -f "$VISUAL/scenarios/$name.txt" ]; then files+=("$VISUAL/scenarios/$name.txt")
  else die "no scenario $name (see tools/visual/scenarios)"
  fi
done

out="${out:-${TMPDIR:-/tmp}/wormhole-visual/$(date +%Y%m%d-%H%M%S)-$$}"
mkdir -p "$out"
out="$(cd "$out" && pwd -P)"
case "$out/" in "$(cd "$ROOT" && pwd -P)"/*) die "--out must be outside the repository" ;; esac
summary="$out/summary.txt"

if python3 -c 'import PIL, numpy' >/dev/null 2>&1; then
  pixel_diff=1
else
  pixel_diff=0
  warn "no Pillow/numpy for python3: comparing files byte for byte, without pictures of the change"
fi

# ---------------------------------------------------------------- builds

alive() { [ -n "${1:-}" ] && kill -0 "$1" 2>/dev/null; }

# copy_binary <built binary> <folder>: our own copy, so a later build can't replace it while it runs.
# The session host built beside it comes along: the app looks for it next to its own executable.
copy_binary() {
  mkdir -p "$2"
  cp -c "$1" "$2/$APP_BINARY" 2>/dev/null || cp "$1" "$2/$APP_BINARY"
  copy_host_beside "$1" "$2"
}

src_worktree=""
pid="" ctl=""

# shellcheck disable=SC2329  # the EXIT trap calls it
cleanup() {
  stop_instance
  if [ -n "$src_worktree" ]; then
    git -C "$ROOT" worktree remove --force "$src_worktree" 2>/dev/null || warn "left worktree $src_worktree"
  fi
}
trap cleanup EXIT
trap 'exit 130' INT TERM HUP

build_branch() {
  if [ "$build" = 1 ]; then
    log "building this tree (--features devctl)"
    set_app_build_args devctl "$ROOT"
    (cd "$ROOT" && cargo build "${APP_BUILD_ARGS[@]}") || die "the build failed"
  fi
  [ -x "$TARGET_DIR/debug/$APP_BINARY" ] || die "no $TARGET_DIR/debug/$APP_BINARY (build first)"
  copy_binary "$TARGET_DIR/debug/$APP_BINARY" "$out/bin/branch"
}

# build_rev <rev> <label>: that commit's binary, built once in a temporary worktree that shares the
# target dir (scripts/worktree.sh keeps its crates apart) and kept by commit for the next run.
build_rev() {
  local sha cache
  sha="$(git -C "$ROOT" rev-parse --verify --quiet "$1^{commit}")" || die "unknown revision: $1"
  cache="$TARGET_DIR/visual-check-bins/$sha"
  if [ ! -x "$cache/$APP_BINARY" ]; then
    src_worktree="$out/src-${sha:0:12}"
    log "building $1 (${sha:0:12}) in $src_worktree"
    git -C "$ROOT" worktree add --detach --quiet "$src_worktree" "$sha"
    "$ROOT/scripts/worktree.sh" "$src_worktree" >/dev/null 2>&1
    set_app_build_args devctl "$src_worktree"
    (cd "$src_worktree" && CARGO_TARGET_DIR="$TARGET_DIR" cargo build "${APP_BUILD_ARGS[@]}") \
      || die "the build of $1 failed"
    copy_binary "$TARGET_DIR/debug/$APP_BINARY" "$cache"
    # That build left its binary where this tree's was: put this tree's back, or a later
    # --no-build (or a devctl run from the target dir) runs the other revision. Unlink it
    # first: cargo may hard-link it to its own copy, which a plain cp would overwrite too.
    rm -f "$TARGET_DIR/debug/$APP_BINARY" "$TARGET_DIR/debug/$HOST_BINARY"
    copy_binary "$out/bin/branch/$APP_BINARY" "$TARGET_DIR/debug"
    git -C "$ROOT" worktree remove --force "$src_worktree"
    src_worktree=""
  fi
  copy_binary "$cache/$APP_BINARY" "$out/bin/$2"
}

# ---------------------------------------------------------------- scenarios

json_string() {
  local s="${1//\\/\\\\}"
  printf '"%s"' "${s//\"/\\\"}"
}

# parse <file>: what the instance needs before it starts, and the steps after.
parse() {
  fixture="" shell_name="" envs=() configs=() size_w=1440 size_h=900 steps=()
  local line
  while IFS= read -r line || [ -n "$line" ]; do
    line="${line%$'\r'}"
    line="${line#"${line%%[![:space:]]*}"}"
    case "$line" in
      ''|'#'*) ;;
      '@fixture '*) fixture="${line#@fixture }" ;;
      '@shell '*) shell_name="${line#@shell }" ;;
      '@env '*) envs+=("${line#@env }") ;;
      '@config '*) configs+=("${line#@config }") ;;
      '@theme '*) configs+=("theme=$(json_string "${line#@theme }")") ;;
      '@size '*) read -r size_w size_h <<<"${line#@size }" ;;
      *) steps+=("$line") ;;
    esac
  done < "$1"
  [ -z "$fixture" ] || [ -x "$VISUAL/fixtures/$fixture.sh" ] || die "$1: no fixture $fixture"
  case "$shell_name" in */*) die "$1: @shell takes a name, not a path" ;; esac
}

write_config() {
  local kv
  {
    # No banners on the desktop, no update check: neither belongs in a screenshot. The first terminal
    # is a plain one, not the terminal welcome, whose banner would be in every picture: a scenario
    # that wants the welcome says `@config feature_flags={"terminal_welcome":true}` (or `welcome …`).
    printf '{\n  "notifications": false,\n  "auto_update": false,\n  "feature_flags": {"terminal_welcome": false}'
    for kv in ${configs[@]+"${configs[@]}"}; do printf ',\n  %s: %s' "$(json_string "${kv%%=*}")" "${kv#*=}"; done
    printf '\n}\n'
  } > "$1"
}

# start_instance <binary> <run dir>
start_instance() {
  local bin="$1" run="$2" home start shell
  mkdir -p "$run/data" "$run/ctl" "$run/home" "$run/agent-home" "$run/bin"
  ctl="$run/ctl"
  # Paths under HOME read ~/…, so the scratch folder's name never shows in a screenshot.
  home="$(cd "$run/home" && pwd -P)"
  start="$home"
  if [ -n "$fixture" ]; then
    "$VISUAL/fixtures/$fixture.sh" "$home/$fixture" >"$run/fixture.log" 2>&1 \
      || die "fixture $fixture failed (see $run/fixture.log)"
    start="$home/$fixture"
  fi
  shell=/bin/cat
  local agent_path=""
  if [ -n "$shell_name" ]; then
    # A plain copy of a system binary is killed on launch; an ad-hoc signature lets it run.
    cp /bin/cat "$run/bin/$shell_name"
    codesign -s - -f "$run/bin/$shell_name" 2>/dev/null || die "cannot sign the $shell_name shell"
    shell="$run/bin/$shell_name"
    # The one agent that counts as installed (Settings → Coding agents, the first-run sheet).
    agent_path="$run/bin"
  fi
  write_config "$run/data/config.json"
  start_isolated "$bin" "$run" "$start" "$shell" TZ=UTC WORMHOLE_AGENT_PATH="$agent_path" \
    ${envs[@]+"${envs[@]}"}
}

stop_instance() {
  alive "$pid" || { pid=""; return 0; }
  stop_isolated "$pid" "$ctl" || true
  wait "$pid" 2>/dev/null || true
  if alive "$pid"; then warn "instance $pid is still running"; else log "instance $pid stopped"; fi
  pid=""
}

batch=() seq=0 legacy=0 missing=""

# flush: send the pending lines and a sync, and wait until the instance has run all of them.
flush() {
  local token out
  seq=$((seq + 1))
  token="b$seq-$$"
  out="$ctl/out.txt"
  rm -f "$out"
  { [ ${#batch[@]} -eq 0 ] || printf '%s\n' "${batch[@]}"; printf 'sync %s\n' "$token"; } > "$ctl/cmd.tmp"
  mv "$ctl/cmd.tmp" "$ctl/cmd"
  local deadline=$((SECONDS + BATCH_TIMEOUT))
  # A build from before `sync` answers "unknown command: sync" at the end of the batch instead.
  until grep -qxE "sync $token|unknown command: sync" "$out" 2>/dev/null; do
    alive "$pid" || die "the instance exited (see $run/app.log)"
    [ "$SECONDS" -lt "$deadline" ] || die "batch $seq got no answer in ${BATCH_TIMEOUT}s (see $run/devctl.log)"
    sleep 0.05
  done
  { printf -- '--- batch %s\n' "$seq"; [ ${#batch[@]} -eq 0 ] || printf '> %s\n' "${batch[@]}"; cat "$out"; } \
    >> "$run/devctl.log"
  grep -qx 'unknown command: sync' "$out" && legacy=1
  local unknown
  while IFS= read -r unknown; do
    case " sync $missing " in *" $unknown "*) ;; *) missing="${missing:+$missing }$unknown" ;; esac
  done < <(sed -n 's/^unknown command: //p' "$out")
  local problem
  while IFS= read -r problem; do warn "$scenario: $problem"; done < <(grep -E '^(timeout|[a-z-]+ failed)' "$out" || true)
  batch=()
}

# settle: what a step that needs a quiet window adds to the batch.
settle() {
  if [ "$legacy" = 1 ]; then
    # No wait-idle in that build: give the git work a moment, then draw twice.
    flush; sleep 1.5; batch+=("frames 2"); flush; sleep 0.5
  else
    batch+=("wait-idle${1:+ $1}")
  fi
}

# shot <label> [main|settings|studio]
shot() {
  local label="$1" which="${2:-main}" name file
  name="$scenario-$label"
  settle
  case "$which" in
    main) batch+=("shot $name") ;;
    settings) batch+=("settings-shot $name") ;;
    studio) batch+=("studio-shot $name") ;;
    *) die "$scenario: @shot $label $which: expected main, settings or studio" ;;
  esac
  flush
  file="$ctl/$name.png"
  for _ in $(seq 100); do [ -s "$file" ] && break; sleep 0.05; done
  [ -s "$file" ] || die "$scenario: no screenshot $name (see $run/devctl.log)"
  cp "$file" "$dest/$name.png"
  if [ -z "${scale:-}" ] && [ "$which" = main ]; then
    scale="$(sips -g pixelWidth "$file" | awk -v w="$size_w" '/pixelWidth/ { printf "%g", $2 / w }')"
  fi
  printf '%s\t%s\t%s\n' "$label_now" "$name" "$dest/$name.png" >> "$summary"
}

# run_scenario <file> <binary> <label>: screenshots land in $out/<label>/.
run_scenario() {
  local file="$1" bin="$2" step words
  label_now="$3"
  scenario="$(basename "$file" .txt)"
  dest="$out/$label_now"
  run="$out/$label_now/$scenario"
  mkdir -p "$dest"
  parse "$file"
  log "$label_now: $scenario"
  start_instance "$bin" "$run"
  legacy=0 missing=""
  batch=()
  flush
  if [ "$legacy" = 1 ]; then
    warn "$label_now has no sync/wait-idle (an older build): waiting by time, animations not held"
  else
    batch+=("reduce-motion on")
    # A fresh data dir is a first launch: the onboarding sheet would cover every shot. A scenario
    # that wants it opens it again with `onboarding <step>`.
    batch+=("onboarding close")
    # What the Mac says doesn't reach the shots: a dark system (Match system would show the light
    # pick in Light Mode) with full transparency and the theme's own contrast. A scenario that
    # wants otherwise says `appearance light`, `reduce-transparency on` or `increase-contrast on`.
    batch+=("appearance dark" "reduce-transparency off" "increase-contrast off")
  fi
  batch+=("resize $size_w $size_h")
  for step in ${steps[@]+"${steps[@]}"}; do
    read -r -a words <<<"$step"
    case "$step" in
      '@wait-idle'|'@wait-idle '*) settle "${words[1]:-}" ;;
      '@batch') flush ;;
      '@sleep '*) flush; sleep "$(awk -v ms="${words[1]}" 'BEGIN { print ms / 1000 }')" ;;
      '@shot '*) shot "${words[1]}" "${words[2]:-}" ;;
      '@'*) die "$scenario: unknown directive: $step" ;;
      *) batch+=("$step") ;;
    esac
  done
  [ ${#batch[@]} -eq 0 ] || flush
  [ -z "$missing" ] || warn "$label_now lacks devctl commands: $missing"
  stop_instance
}

# ---------------------------------------------------------------- comparing

# compare <before.png> <after.png> <name> <labels>
compare() {
  if [ "$pixel_diff" = 1 ]; then
    python3 "$VISUAL/diff.py" pair "$1" "$2" --out "$out/diff" --name "$3" --scale "${scale:-2}" \
      --labels "$4"
  elif cmp -s "$1" "$2"; then
    echo "$3: identical"
  else
    echo "$3: differs"
  fi
}

# ---------------------------------------------------------------- main

build_branch
rev_label=""
if [ -n "$against" ]; then
  rev_label="rev-$(printf '%s' "$against" | tr -c 'A-Za-z0-9._-' '-')"
  build_rev "$against" "$rev_label"
fi

printf 'label\tshot\tfile\n' > "$summary"
for file in "${files[@]}"; do
  run_scenario "$file" "$out/bin/branch/$APP_BINARY" branch
  [ "$repeat" = 0 ] || run_scenario "$file" "$out/bin/branch/$APP_BINARY" branch-again
  [ -z "$rev_label" ] || run_scenario "$file" "$out/bin/$rev_label/$APP_BINARY" "$rev_label"
done

status=0
results="$out/results.txt"
: > "$results"
while IFS=$'\t' read -r label name path; do
  [ "$label" = branch ] || continue
  if [ -n "$rev_label" ]; then
    if [ -f "$out/$rev_label/$name.png" ]; then
      compare "$out/$rev_label/$name.png" "$path" "$name" "$against,this tree" | tee -a "$results"
    else
      echo "$name: missing on $rev_label" | tee -a "$results"
    fi
  fi
  if [ "$repeat" = 1 ]; then
    line="$(compare "$out/branch-again/$name.png" "$path" "$name-repeat" "second run,first run")"
    echo "$line" | tee -a "$results"
    case "$line" in *": 0 px changed"*|*": identical") ;; *) status=1 ;; esac
  fi
done < "$summary"

log "screenshots: $out/branch${rev_label:+ and $out/$rev_label}; scale ${scale:-?} (device px per logical px)"
[ ! -d "$out/diff" ] || log "pictures of the differences: $out/diff"
[ ! -s "$results" ] || log "results: $results"
[ "$status" = 0 ] || warn "the two runs differ: the scenario is not deterministic"
exit "$status"
