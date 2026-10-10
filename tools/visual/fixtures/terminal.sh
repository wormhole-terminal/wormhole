#!/usr/bin/env bash
# The folders the sidebar shows, next to each other under one home:
#   terminal/      a repository on `ui-unify` with three changed files (the first terminal's folder)
#   api-server/    on `feature/auth`, one change
#   website/       on `main`, clean
#   dotfiles/      a plain folder, no repository
#   bisect/        a repository with a detached HEAD
#   terminal-worktrees/badge/   a worktree of terminal/ on `worktree-badge`, seven changes
#   customer-onboarding-service/   long folder and branch names
#   tools/visual/fixtures/terminal.sh <home>/terminal
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"

home="$(dirname "$repo")"

write README.md <<'EOF'
# terminal

A terminal for coding agents.
EOF
write src/sidebar.rs <<'EOF'
pub fn render() {}
EOF
write src/style.rs <<'EOF'
pub const ROW_H: f32 = 28.0;
EOF
commit 3 "Start the sidebar"
git -C "$repo" checkout -q -b ui-unify
printf '\npub fn bands() {}\n' >> "$repo/src/sidebar.rs"
printf '\npub const BAND_H: f32 = 32.0;\n' >> "$repo/src/style.rs"
write src/hints.rs <<'EOF'
pub fn keycaps() {}
EOF

# Another repository next to the first: `other <name> <branch>`, one commit on `main`, then the
# branch. Files and commit dates as in lib.sh.
other() {
  local dir="$home/$1" when=$(( today - 2 * 86400 ))
  mkdir -p "$dir"
  git -C "$dir" init -q -b main
  git -C "$dir" config core.excludesFile /dev/null
  git -C "$dir" config commit.gpgSign false
  printf '# %s\n' "$1" > "$dir/README.md"
  git -C "$dir" add -A
  GIT_AUTHOR_DATE="@$when +0000" GIT_COMMITTER_DATE="@$when +0000" git -C "$dir" commit -q -m "Start $1"
  [ "$2" = main ] || git -C "$dir" checkout -q -b "$2"
}

other api-server feature/auth
printf 'token = "rotate"\n' > "$home/api-server/auth.toml"
other website main
other bisect main
printf 'fn clear() {}\n' > "$home/bisect/clear.rs"
git -C "$home/bisect" add -A
when=$(( today - 1 * 86400 ))
GIT_AUTHOR_DATE="@$when +0000" GIT_COMMITTER_DATE="@$when +0000" \
  git -C "$home/bisect" commit -q -m "fix(terminal): erase scrollback when running clear"
git -C "$home/bisect" checkout -q --detach HEAD
other customer-onboarding-service bugfix/duplicate-welcome-email
for n in 1 2 3; do printf 'change %s\n' "$n" > "$home/customer-onboarding-service/change-$n.txt"; done

mkdir -p "$home/dotfiles"
printf 'export EDITOR=vim\n' > "$home/dotfiles/zshrc"

mkdir -p "$home/terminal-worktrees"
git -C "$repo" worktree add -q -b worktree-badge "$home/terminal-worktrees/badge" main
for n in 1 2 3 4 5 6 7; do printf 'badge %s\n' "$n" > "$home/terminal-worktrees/badge/badge-$n.txt"; done
