#!/usr/bin/env bash
# Copies the bundled file icon theme of `wormhole-core` (Zed's icon theme format, Catppuccin and Lucide icons) into the
# Swift UI's `FileIconTheme` resources, so the tree and the Changes rows draw the app's own icons. The theme file is JSONC
# in core and plain JSON here (comment lines dropped); the SVGs are copied as they are. `--check` changes nothing and
# fails when the copy is out of date.
set -euo pipefail

apple="$(cd "$(dirname "$0")/.." && pwd)"
source="$apple/../crates/wormhole-core/assets/icon-themes/wormhole"
target="$apple/swift/Sources/FileIconTheme/Resources"

json() { python3 -I -c 'import sys; print("".join(l for l in open(sys.argv[1]) if not l.lstrip().startswith("//")), end="")' "$source/wormhole-icons.json"; }

if [[ "${1:-}" == "--check" ]]; then
  diff -q <(json) "$target/wormhole-icons.json" >/dev/null && diff -rq "$source/icons" "$target/icons" >/dev/null \
    || { echo "the icon theme copy is stale: run apple/scripts/dump-icon-theme.sh" >&2; exit 1; }
  echo "the icon theme copy is current"
else
  mkdir -p "$target/icons"
  json > "$target/wormhole-icons.json"
  cp -R "$source/icons/dark" "$source/icons/light" "$source/icons/LICENSES.md" "$target/icons/"
fi
