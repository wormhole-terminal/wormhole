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
# Both apps size the grid by the core's cell (16.8 x 36 px at 14 pt, 2x): the crops are compared as they are, cell for cell,
# and each scene's features (visual-scenes.py REGIONS) must pass on their own.
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
# Swift's shot is the pane as drawn: its padding (4 pt, 8 px at scale 2: `MetalRenderer.padding`) around the grid, which GPUI's crop leaves out.
# The size is Swift's grid (its `state`), whole cells of 16.8 x 36; the ruler scene shows GPUI's grid is the same.
grid="$(s state | sed -n 's/^grid=\([0-9]*\)x\([0-9]*\) .*/\1 \2/p' | head -1)"
read -r cols rows <<< "$grid"
size="$(python3 -c "print(f'{round($cols * 16.8)},{$rows * 36}')")"
echo "Swift grid ${cols}x${rows}: comparing ${size} px"
REG=(--regions "$scratch/run/regions.json")
scene() { # <name> <prefix>: the scene in both, then a shot of each
  echo "$1" > "$scratch/run/gpui.fifo"; echo "$1" > "$scratch/run/swift.fifo"; sleep 1.5
  g "frames 2" > /dev/null; s "frames 2" > /dev/null
  g "shot $2-$1" > /dev/null; s "shot $ctl_s/$2-$1.png" > /dev/null
}
# Where GPUI's grid starts in its window (the pane's place is the window's chrome, not the grid's): the ruler scene found by a
# search of 4 px around the nominal 722,84; every scene is then compared there with no search.
scene ruler locate
shift="$(python3 "$here/visual-compare.py" compare locate-ruler "$ctl_g/locate-ruler.png" "$ctl_s/locate-ruler.png" --out "$scratch/locate" \
  --origin 722,84 --size "$size" --cell 16.8,36 --swift-origin 8,8 --search 4 | python3 -c 'import json,sys; print(*json.load(sys.stdin)["shift"])')"
read -r sx sy <<< "$shift"; ox=$(( 722 + sx )) oy=$(( 84 + sy ))
echo "GPUI grid at ${ox},${oy}"
# shellcheck disable=SC2054 # the commas are inside the values (x,y pairs), not between elements
GRID=(--origin "$ox,$oy" --size "$size" --cell 16.8,36 --swift-origin 8,8)

for mode in dark light; do
  g "appearance $mode" > /dev/null; s "appearance $mode" > /dev/null; sleep 1.5
  for name in ruler text pad cursor-block cursor-underline cursor-bar images blocks; do
    scene "$name" "g$mode"
    python3 "$here/visual-compare.py" compare "$mode-$name" "$ctl_g/g$mode-$name.png" "$ctl_s/g$mode-$name.png" --out "$out" "${GRID[@]}" "${REG[@]}" --scene "$name" > /dev/null
  done
  # Find: Swift's query is set whole (`find`; typed a key at a time its fresh searches land elsewhere: not compared) and the
  # caret put after it, as GPUI's typed field has it. The matches painted in the text; the bar is chrome (GPUI's is part of its view, Swift's an overlay), so the top four rows (with the bar's shadow) are left out and the bar is compared in a shot of the whole window.
  echo scrollback > "$scratch/run/gpui.fifo"; echo scrollback > "$scratch/run/swift.fifo"; sleep 4
  g "key cmd-f" > /dev/null; g "text line 18" > /dev/null; s "find line 18" > /dev/null; s "key right" > /dev/null; sleep 1
  g "frames 2" > /dev/null; s "frames 2" > /dev/null
  g "shot g$mode-find" > /dev/null; s "shot $ctl_s/g$mode-find.png" > /dev/null
  python3 "$here/visual-compare.py" compare "$mode-find" "$ctl_g/g$mode-find.png" "$ctl_s/g$mode-find.png" --out "$out" --origin "$ox,$(( oy + 144 ))" --size "${size%,*},$(( rows * 36 - 144 ))" --cell 16.8,36 --swift-origin 8,152 "${REG[@]}" --scene find --first-row 4 > /dev/null
  s "window-shot $ctl_s/g$mode-find-window.png" > /dev/null
  python3 "$here/visual-compare.py" compare "$mode-find-bar" "$ctl_g/g$mode-find.png" "$ctl_s/g$mode-find-window.png" --out "$out" --origin 1750,84 --size 890,108 --cell 16,16 --swift-origin 1750,84 > /dev/null
  g "key escape" > /dev/null; s "key esc" > /dev/null

  # Increase Contrast on the text scene, then the minimap on the scrollback.
  echo text > "$scratch/run/gpui.fifo"; echo text > "$scratch/run/swift.fifo"; sleep 1.5
  # The setting must show: each app's window before and after (GPUI's shot is its window), and the places it changed compared.
  g "frames 2" > /dev/null; g "shot g$mode-contrast-off" > /dev/null; s "window-shot $ctl_s/g$mode-contrast-off.png" > /dev/null
  g "increase-contrast on" > /dev/null; s "increase-contrast on" > /dev/null; sleep 1
  g "frames 2" > /dev/null; s "frames 2" > /dev/null
  g "shot g$mode-contrast" > /dev/null; s "shot $ctl_s/g$mode-contrast.png" > /dev/null; s "window-shot $ctl_s/g$mode-contrast-on.png" > /dev/null
  python3 "$here/visual-compare.py" compare "$mode-contrast" "$ctl_g/g$mode-contrast.png" "$ctl_s/g$mode-contrast.png" --out "$out" "${GRID[@]}" "${REG[@]}" --scene contrast > /dev/null
  python3 "$here/visual-compare.py" delta "$mode-contrast-delta" "$ctl_g/g$mode-contrast-off.png" "$ctl_g/g$mode-contrast.png" \
    "$ctl_s/g$mode-contrast-off.png" "$ctl_s/g$mode-contrast-on.png" --out "$out/delta" >> "$out/deltas.txt"
  g "increase-contrast off" > /dev/null; s "increase-contrast off" > /dev/null
  # Reduce Transparency, the same way (the run keeps it on; off and back on here).
  g "reduce-transparency off" > /dev/null; s "reduce-transparency off" > /dev/null; sleep 1
  g "frames 2" > /dev/null; s "frames 2" > /dev/null
  g "shot g$mode-transparency-off" > /dev/null; s "window-shot $ctl_s/g$mode-transparency-off.png" > /dev/null
  g "reduce-transparency on" > /dev/null; s "reduce-transparency on" > /dev/null; sleep 1
  g "frames 2" > /dev/null; s "frames 2" > /dev/null
  g "shot g$mode-transparency-on" > /dev/null; s "window-shot $ctl_s/g$mode-transparency-on.png" > /dev/null
  python3 "$here/visual-compare.py" delta "$mode-transparency-delta" "$ctl_g/g$mode-transparency-off.png" "$ctl_g/g$mode-transparency-on.png" \
    "$ctl_s/g$mode-transparency-off.png" "$ctl_s/g$mode-transparency-on.png" --out "$out/delta" >> "$out/deltas.txt"
  g "set minimap=true" > /dev/null; s "set minimap=true" > /dev/null; sleep 1.5
  echo scrollback > "$scratch/run/gpui.fifo"; echo scrollback > "$scratch/run/swift.fifo"; sleep 4
  g "frames 2" > /dev/null; s "frames 2" > /dev/null
  g "shot g$mode-minimap" > /dev/null; s "shot $ctl_s/g$mode-minimap.png" > /dev/null
  python3 "$here/visual-compare.py" compare "$mode-minimap" "$ctl_g/g$mode-minimap.png" "$ctl_s/g$mode-minimap.png" --out "$out" "${GRID[@]}" "${REG[@]}" --scene minimap > /dev/null
  # The strip itself is a SwiftUI view over the Metal one, so it is compared in a shot of the whole window, at its right edge.
  s "window-shot $ctl_s/g$mode-minimap-window.png" > /dev/null
  python3 "$here/visual-compare.py" compare "$mode-minimap-strip" "$ctl_g/g$mode-minimap.png" "$ctl_s/g$mode-minimap-window.png" --out "$out" --origin 2400,84 --size "240,$(( rows * 36 ))" --cell 16,36 --swift-origin 2400,84 > /dev/null
  g "set minimap=false" > /dev/null; s "set minimap=false" > /dev/null

  # Text zoom (T17): two steps in (Cmd =), the text scene at the zoomed cell (Swift's `state`, the core's cell either way), then back.
  g "key cmd-=" "key cmd-=" > /dev/null; s "key cmd-=" "key cmd-=" > /dev/null; sleep 2
  zoomed="$(s state | sed -n 's/^grid=\([0-9]*\)x\([0-9]*\) cell=\([0-9.]*\)x\([0-9]*\)px.*/\1 \2 \3 \4/p' | head -1)"
  read -r zc zr zw zh <<< "$zoomed"
  scene text "z$mode"
  python3 "$here/visual-compare.py" compare "$mode-zoom" "$ctl_g/z$mode-text.png" "$ctl_s/z$mode-text.png" --out "$out" --origin "$ox,$oy" \
    --size "$(python3 -c "print(f'{round($zc * $zw)},{$zr * $zh}')")" --cell "$zw,$zh" --swift-origin 8,8 "${REG[@]}" --scene text > /dev/null
  g "key cmd-0" > /dev/null; s "key cmd-0" > /dev/null; sleep 2

  # A selection from the left of row 3, column 5 to the right of row 5, column 20 (GPUI by the mouse at the same cells).
  echo text > "$scratch/run/gpui.fifo"; echo text > "$scratch/run/swift.fifo"; sleep 1.5
  g "mouse-down 396 87" > /dev/null; g "mouse-drag 470 105" > /dev/null; g "mouse-drag 529 123" > /dev/null; g "mouse-up 529 123" > /dev/null
  s "select-cells 2 4 4 19" > /dev/null; g "frames 2" > /dev/null; s "frames 2" > /dev/null
  g "shot g$mode-selection" > /dev/null; s "shot $ctl_s/g$mode-selection.png" > /dev/null
  python3 "$here/visual-compare.py" compare "$mode-selection" "$ctl_g/g$mode-selection.png" "$ctl_s/g$mode-selection.png" --out "$out" "${GRID[@]}" "${REG[@]}" --scene selection > /dev/null

  # Cmd held over the OSC 8 link on the text scene's row 25 (T08): the underline both apps draw under the whole link. Window
  # points: the grid's corner (GPUI's, found above) plus half a cell into column 13.
  hx=$(( ox / 2 + 109 )) hy=$(( oy / 2 + 441 ))
  echo text > "$scratch/run/gpui.fifo"; echo text > "$scratch/run/swift.fifo"; sleep 1.5
  g "hold-cmd" "hover $hx $hy" > /dev/null; s "hover-cmd $hx $hy" > /dev/null; sleep 0.5
  g "frames 2" > /dev/null; s "frames 2" > /dev/null
  g "shot g$mode-cmd-hover" > /dev/null; s "shot $ctl_s/g$mode-cmd-hover.png" > /dev/null
  python3 "$here/visual-compare.py" compare "$mode-cmd-hover" "$ctl_g/g$mode-cmd-hover.png" "$ctl_s/g$mode-cmd-hover.png" --out "$out" "${GRID[@]}" "${REG[@]}" --scene text > /dev/null
  g "release-cmd" "hover 1200 700" > /dev/null; s "hover 1200 700" > /dev/null

  if [ -z "${SKIP_SETTINGS:-}" ]; then  # SKIP_SETTINGS=1: the terminal scenes only
  # Settings: GPUI's window set to the size Swift opens at (920 x 1084 pt), every page's first screen.
  g "settings-section appearance" > /dev/null; g "settings-resize 920 1084" > /dev/null
  s "settings-section appearance" > /dev/null; sleep 1
  for page in Profile Appearance Fonts Terminal Sessions Git Agents Import Notifications MenuBar Hotkey Updates Advanced; do
    id="$(printf '%s' "${page:0:1}" | tr '[:upper:]' '[:lower:]')${page:1}"
    g "settings-section $page" > /dev/null; s "settings-section $id" > /dev/null; sleep 1
    g "settings-shot s$mode-$page" > /dev/null; s "window-shot $ctl_s/s$mode-$page.png Settings" > /dev/null
    python3 "$here/visual-compare.py" compare "settings-$mode-$page" "$ctl_g/s$mode-$page.png" "$ctl_s/s$mode-$page.png" --out "$out" > /dev/null
  done
  fi
done
python3 "$here/visual-compare.py" table "$out"
cat "$out/deltas.txt" 2> /dev/null || true
echo "sheets: $out/<scenario>.png (GPUI | Swift | difference)"
