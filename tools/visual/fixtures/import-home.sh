#!/usr/bin/env bash
# A folder standing in for a home with Ghostty, iTerm2 and Alacritty settings in it
# (`fixtures/import/home`), for the Settings → Import scenario: `import-scan ~/import-home` reads it as
# the home. The files' ages are set so the cards always come in the same order (newest first).
set -euo pipefail

dest="${1:?usage: $0 <directory to create>}"
[ ! -e "$dest" ] || [ -z "$(ls -A "$dest")" ] || { echo "error: $dest exists and is not empty" >&2; exit 1; }
mkdir -p "$dest"
cp -R "$(dirname "$0")/import/home/." "$dest/"

# Not a repository: the scenario's first terminal opens in the folder, which needs no history.
touch -t 202610260900 "$dest/.config/ghostty/config"
touch -t 202610250900 "$dest/Library/Preferences/com.googlecode.iterm2.plist"
touch -t 202610240900 "$dest/.config/alacritty/alacritty.toml"
