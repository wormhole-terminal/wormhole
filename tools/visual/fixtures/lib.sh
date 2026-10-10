# Shared by the fixture scripts. Source it, don't run it.
# shellcheck shell=bash disable=SC2034  # `repo` is for the fixture scripts
#
# A fixture is a throwaway git repository that looks the same on every run: no global or system
# git config, one identity, files written byte for byte, and commit dates anchored to the start
# of the current day (UTC). History shows ages ("3d ago"), so a date relative to "now" is what
# keeps the picture stable; anchoring it to the day keeps the commit hashes stable too, so two
# runs on the same UTC day are identical.

set -euo pipefail

repo="${1:?usage: $0 <directory to create>}"
[ ! -e "$repo" ] || [ -z "$(ls -A "$repo")" ] || { echo "error: $repo exists and is not empty" >&2; exit 1; }
mkdir -p "$repo"

export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null GIT_CONFIG_NOSYSTEM=1
export GIT_AUTHOR_NAME="Ada Lovelace" GIT_AUTHOR_EMAIL="ada@example.com"
export GIT_COMMITTER_NAME="Ada Lovelace" GIT_COMMITTER_EMAIL="ada@example.com"
export TZ=UTC LC_ALL=C

today=$(( $(date +%s) / 86400 * 86400 ))

git -C "$repo" init -q -b main
git -C "$repo" config core.excludesFile /dev/null
git -C "$repo" config core.autocrlf false
git -C "$repo" config commit.gpgSign false

# write <path> <<'EOF' … EOF: the file, from stdin, with its folders.
write() {
  mkdir -p "$(dirname "$repo/$1")"
  cat > "$repo/$1"
}

# commit <days ago (1 or more)> <message>: everything in the folder, dated that many days before
# today's start. A commit from today would age while the run lasts.
commit() {
  local when=$(( today - $1 * 86400 ))
  git -C "$repo" add -A
  GIT_AUTHOR_DATE="@$when +0000" GIT_COMMITTER_DATE="@$when +0000" git -C "$repo" commit -q -m "$2"
}
