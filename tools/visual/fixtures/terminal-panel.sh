#!/usr/bin/env bash
# The History and Project panels with the sidebar's neighbours: repo-panel's history and .wormhole/
# project as ~/terminal, with terminal.sh's neighbours (api-server, website, dotfiles) for the sidebar.
# The folder the runner names after the fixture is a link to ~/terminal, so the first terminal's band
# reads "terminal" (a process's folder is its real path).
#   tools/visual/fixtures/g3-terminal-panel.sh <home>/g3-terminal-panel
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
link="${1:?usage: $0 <directory to create>}"
home="$(dirname "$link")"
"$here/repo-panel.sh" "$home/terminal"
# terminal.sh puts its neighbours next to the repository it makes: make that one in a side folder
# and move the neighbours the sidebar shows next to ours.
"$here/terminal.sh" "$home/.neighbours/terminal"
mv "$home/.neighbours/api-server" "$home/.neighbours/website" "$home/.neighbours/dotfiles" "$home/"
[ ! -e "$link" ] || rmdir "$link"
ln -s terminal "$link"
