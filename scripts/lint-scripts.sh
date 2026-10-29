#!/bin/bash
# Lints the shell scripts and the GitHub workflows, in seconds, building nothing:
#   - shellcheck: every tracked script with a sh or bash shebang, following what it sources (.shellcheckrc)
#   - actionlint: the workflows: syntax, expressions, contexts, job outputs, and shellcheck on each run: step
#   - zizmor: the workflows' security: injection, permissions, persisted credentials, cache poisoning
# Zed's CI runs the same three. A tool that isn't installed is skipped with a warning here (brew install
# actionlint shellcheck zizmor); CI passes --require, so there a missing one fails.
#
#   scripts/lint-scripts.sh [--require]
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
. scripts/lib.sh

require=0
case "${1:-}" in
  "") ;;
  --require) require=1 ;;
  -h|--help) sed -n '2,9p' "$0"; exit 0 ;;
  *) die "unknown argument: $1 (see --help)" ;;
esac

have() {
  command -v "$1" >/dev/null 2>&1 && return 0
  [ "$require" = 1 ] && die "$1 is not installed"
  warn "$1 is not installed (brew install $1): skipped"
  return 1
}

# tools/bench is its own workspace with its own checks; nothing under crates/ is a script of ours.
scripts=()
while IFS= read -r -d '' f; do
  case "$f" in tools/bench/*|crates/*) continue ;; esac
  head -n 1 "$f" 2>/dev/null | grep -Eq '^#!.*[/ ](ba)?sh( |$)' && scripts+=("$f")
done < <(git ls-files -z)

failed=0
if have shellcheck; then
  shellcheck "${scripts[@]}" || failed=1
fi
if have actionlint; then
  actionlint || failed=1
fi
if have zizmor; then
  # Offline: the same answer on every machine, no token needed. The online audits (known-vulnerable
  # actions, impostor commits) would add a network call to every run.
  zizmor --offline --quiet .github/workflows || failed=1
fi
[ "$failed" = 0 ] && log "workflows and ${#scripts[@]} scripts: ok"
exit "$failed"
