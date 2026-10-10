#!/bin/bash
# PostToolUse hook (Write|Edit), run by rustfmt-edited.sh once the file is formatted: flag GPUI
# mistakes in edited app files that compile but misbehave, and tell Claude through
# additionalContext. Never blocks or fails the tool call.
# By hand: .claude/hooks/gpui-lint.sh <file.rs>...  (prints the findings, exits 1 if any)

# Binding a Task or Subscription to `_` (or an unused `_name`) drops it: the task is cancelled
# and the subscription ends at once (or at the end of the scope). Both types are #[must_use], so
# `let _ =` (or the bare `_ =`) is exactly how the warning gets "fixed".
DROPPED='(^|[^[:alnum:]_])(let[[:space:]]+_[[:alnum:]_]*|_)[[:space:]]*=[[:space:]]*(cx|window)\.'
DROPPED_TASK="$DROPPED"'((spawn|spawn_in|background_spawn)\(|background_executor\(\)\.spawn\()'
DROPPED_SUB="$DROPPED"'(subscribe|subscribe_in|subscribe_self|observe|observe_in|observe_self|observe_global|observe_global_in|observe_release|observe_release_in|on_release|on_release_in|on_focus|on_focus_in|on_blur|on_focus_lost|on_focus_out|observe_keystrokes|observe_window_[a-z_]+)(::<[^>]*>)?\('
# GPUI is reached through gpui_kit; a `gpui::` path is a version from memory or another project.
GPUI_PATH='(^|[^[:alnum:]_:])gpui::'

lint() {
  # Patterns go through the environment: `awk -v` would interpret their backslashes.
  F="$1" TASK="$DROPPED_TASK" SUB="$DROPPED_SUB" PATHS="$GPUI_PATH" awk '
    BEGIN { f = ENVIRON["F"]; task = ENVIRON["TASK"]; sub_ = ENVIRON["SUB"]; path = ENVIRON["PATHS"] }
    # The line without comments: strings emptied first, so a // inside one starts no comment.
    { code = $0; gsub(/"([^"\\]|\\.)*"/, "\"\"", code); sub(/\/\/.*/, "", code) }
    code ~ task { printf "%s:%d: a Task bound to `_` is dropped, which cancels it: keep it in a field or call .detach()\n", f, FNR }
    code ~ sub_ { printf "%s:%d: a Subscription bound to `_` is dropped, which ends it: keep it (e.g. in _subscriptions) or call .detach()\n", f, FNR }
    code ~ path { printf "%s:%d: use gpui_kit:: paths (and #[gpui_kit::test]); gpui:: is not this crate\n", f, FNR }
  ' "$1" 2>/dev/null
}

if [ "$#" -gt 0 ]; then
  out=""
  for f in "$@"; do out+="$(lint "$f")"$'\n'; done
  out="$(printf '%s' "$out" | sed '/^$/d')"
  [ -z "$out" ] && exit 0
  printf '%s\n' "$out"
  exit 1
fi

file=$(jq -r '.tool_input.file_path // empty' 2>/dev/null) || exit 0
case "$file" in
  */crates/wormhole/*.rs) ;;
  *) exit 0 ;;
esac
[ -f "$file" ] || exit 0
findings="$(lint "$file" | head -n 10)"
[ -n "$findings" ] || exit 0
jq -n --arg msg "GPUI lint (see the gpui skill, \"Mistakes that compile\"):
$findings" '{hookSpecificOutput: {hookEventName: "PostToolUse", additionalContext: $msg}}'
exit 0
