#!/bin/bash
# PostToolUse hook (Write|Edit): keep edited Rust files rustfmt-clean, using the repo's rustfmt.toml,
# then run the GPUI lint on the result. The lint runs from here, not as a second hook, because hooks
# of one event run in parallel and it would read the file while rustfmt rewrites it.
# Never fails the tool call: a file mid-edit may not parse yet, and that is not an error here.
input=$(cat)
file=$(printf '%s' "$input" | jq -r '.tool_input.file_path // empty' 2>/dev/null) || exit 0
case "$file" in
  *.rs) [ -f "$file" ] && rustfmt --edition 2024 "$file" >/dev/null 2>&1 ;;
esac
# Its stdout (findings as additionalContext JSON) is this hook's output.
printf '%s' "$input" | "$(dirname "${BASH_SOURCE[0]}")/gpui-lint.sh"
exit 0
