#!/bin/bash
# Installs a commit-msg hook that checks each commit message against the rules in CONTRIBUTING.md
# ("Commits") with scripts/commits.py, so a wrong header is caught while you type it. Optional:
# scripts/check.sh and the pull request check run the same rules over the commits anyway.
#
#   scripts/install-hooks.sh            install (in the hooks folder this clone or worktree uses)
#   scripts/install-hooks.sh --remove   remove it again
#   scripts/install-hooks.sh --force    replace a commit-msg hook that isn't ours
set -euo pipefail
. "$(dirname "$0")/lib.sh"

mode=install
case "${1:-}" in
  "") ;;
  --remove) mode=remove ;;
  --force) mode=force ;;
  -h|--help) sed -n '2,9p' "$0"; exit 0 ;;
  *) die "unknown argument: $1 (see --help)" ;;
esac

# git resolves this for worktrees (they share the main clone's hooks) and for core.hooksPath.
hooks="$(cd "$ROOT" && git rev-parse --git-path hooks)"
case "$hooks" in /*) ;; *) hooks="$ROOT/$hooks" ;; esac
hook="$hooks/commit-msg"
marker="# installed by scripts/install-hooks.sh"

if [ "$mode" = remove ]; then
  if [ -f "$hook" ] && grep -q "$marker" "$hook"; then rm -f "$hook"; log "removed $hook"; else log "no hook of ours at $hook"; fi
  exit 0
fi

if [ -e "$hook" ] && ! grep -q "$marker" "$hook" && [ "$mode" != force ]; then
  die "$hook exists and isn't ours; look at it, then re-run with --force to replace it"
fi

mkdir -p "$hooks"
cat > "$hook" <<HOOK
#!/bin/sh
$marker
# Rules: CONTRIBUTING.md, "Commits". Skip once with git commit --no-verify.
root="\$(git rev-parse --show-toplevel)" || exit 0
[ -f "\$root/scripts/commits.py" ] || exit 0
exec python3 "\$root/scripts/commits.py" check-message --strip-comments --allow-fixup "\$1"
HOOK
chmod +x "$hook"
log "installed $hook"
