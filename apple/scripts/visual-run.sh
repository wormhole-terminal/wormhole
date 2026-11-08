#!/usr/bin/env bash
# The visual comparison of the terminal grid and the Settings pages, GPUI app against Swift app, start to finish.
#
#   apple/scripts/visual-run.sh <scratch dir>
#
# Needs a devctl build of the GPUI app (`cargo build -p wormhole -p wormhole-host --features wormhole/devctl`, its
# target dir in GPUI_TARGET, default target/) and the linked Swift build (`apple/swift`: `swift build -c release`).
# Both start as background instances with scratch data (the repo's own isolation rules), the same scripted content
# (visual-scenes.py) runs in each at the same window size and scale, dark and light, and visual-compare.py writes
# <scratch>/out/results.json, one PNG sheet per scenario (GPUI | Swift | difference) and prints the table.
# GPUI's cells are 16.8 px wide and Swift's 17 (see visual-compare.py), so the GPUI crop is stretched to Swift's cell.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
scratch="$(mkdir -p "${1:?usage: visual-run.sh <scratch dir>}" && cd "$1" && pwd)"
swift_bin="${SWIFT_BIN:-$root/apple/swift/.build/release/Wormhole}"
out="$scratch/out" ctl_g="$scratch/gpui/ctl" ctl_s="$scratch/swift-ctl"
rm -rf "$out"; mkdir -p "$out" "$ctl_s"

# One command batch to an instance, then its answer.
send() { # <ctl dir> <command>...
  local ctl="$1"; shift
  : > "$ctl/out.txt"; printf '%s\n' "$@" > "$ctl/cmd.tmp"; mv "$ctl/cmd.tmp" "$ctl/cmd"; sleep 1.5; cat "$ctl/out.txt"
}
g() { send "$ctl_g" "$@"; }
s() { send "$ctl_s" "$@"; }

python3 "$here/visual-scenes.py" "$scratch/run" > /dev/null
mkdir -p "$scratch/repo"; git -C "$scratch/repo" init -q 2> /dev/null || true

CARGO_TARGET_DIR="${GPUI_TARGET:-$root/target}" "$root/scripts/devctl-instance.sh" --shell "$scratch/run/feed-gpui.sh" "$scratch/gpui" "$scratch/repo" > /dev/null
WORMHOLE_DATA_DIR="$scratch/Wormhole-swift-visual" WORMHOLE_SHELL="$scratch/run/feed-swift.sh" WORMHOLE_DEVCTL="$ctl_s" WORMHOLE_BACKGROUND=1 \
  "$swift_bin" --background --appearance dark > "$scratch/swift.log" 2>&1 &
swift_pid=$!
cleanup() { kill "$swift_pid" 2> /dev/null || true; "$root/scripts/devctl-instance.sh" stop "$scratch/gpui" > /dev/null 2>&1 || true; }
trap cleanup EXIT
sleep 5

# Same window, no translucency, no right panel (Swift opens it), terminal grid only.
g "onboarding close" "resize 1320 820" "reduce-transparency on" > /dev/null
s "size 1320x820" "reduce-transparency on" "panel" > /dev/null
# shellcheck disable=SC2054 # the commas are in the values visual-compare.py parses (`722,84`)
GRID=(--origin 722,84 --size 1904,1548 --cell 17,36 --gpui-cell-w 16.8)

scene() { # <name> <prefix>: the scene in both, then a shot of each
  echo "$1" > "$scratch/run/gpui.fifo"; echo "$1" > "$scratch/run/swift.fifo"; sleep 1.5
  g "frames 2" > /dev/null; s "frames 2" > /dev/null
  g "shot $2-$1" > /dev/null; s "shot $ctl_s/$2-$1.png" > /dev/null
}

for mode in dark light; do
  g "appearance $mode" > /dev/null; s "appearance $mode" > /dev/null; sleep 1.5
  for name in text pad cursor-block cursor-underline cursor-bar images blocks; do
    scene "$name" "g$mode"
    python3 "$here/visual-compare.py" compare "$mode-$name" "$ctl_g/g$mode-$name.png" "$ctl_s/g$mode-$name.png" --out "$out" "${GRID[@]}" > /dev/null
  done
  # A selection from the left of row 3, column 5 to the right of row 5, column 20 (GPUI by the mouse at the same cells).
  echo text > "$scratch/run/gpui.fifo"; echo text > "$scratch/run/swift.fifo"; sleep 1.5
  g "mouse-down 396 87" > /dev/null; g "mouse-drag 470 105" > /dev/null; g "mouse-drag 529 123" > /dev/null; g "mouse-up 529 123" > /dev/null
  s "select-cells 2 4 4 19" > /dev/null; g "frames 2" > /dev/null; s "frames 2" > /dev/null
  g "shot g$mode-selection" > /dev/null; s "shot $ctl_s/g$mode-selection.png" > /dev/null
  python3 "$here/visual-compare.py" compare "$mode-selection" "$ctl_g/g$mode-selection.png" "$ctl_s/g$mode-selection.png" --out "$out" "${GRID[@]}" > /dev/null

  # Settings: GPUI's window set to the size Swift opens at (920 x 1084 pt), every page's first screen.
  g "settings-section appearance" > /dev/null; g "settings-resize 920 1084" > /dev/null
  s "settings-section appearance" > /dev/null; sleep 1
  for page in Profile Appearance Fonts Terminal Sessions Git Agents Import Notifications MenuBar Hotkey Updates Advanced; do
    id="$(printf '%s' "${page:0:1}" | tr '[:upper:]' '[:lower:]')${page:1}"
    g "settings-section $page" > /dev/null; s "settings-section $id" > /dev/null; sleep 1
    g "settings-shot s$mode-$page" > /dev/null; s "window-shot $ctl_s/s$mode-$page.png Settings" > /dev/null
    python3 "$here/visual-compare.py" compare "settings-$mode-$page" "$ctl_g/s$mode-$page.png" "$ctl_s/s$mode-$page.png" --out "$out" > /dev/null
  done
done
python3 "$here/visual-compare.py" table "$out"
echo "sheets: $out/<scenario>.png (GPUI | Swift | difference)"
