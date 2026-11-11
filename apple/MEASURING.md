# Measuring the Swift terminal against the GPUI app

How spike step 4 compares the two, and what the Swift app has to offer so the same definitions
apply. The GPUI side is `scripts/perf.sh` and its parts (`scripts/perf_lib.sh`,
`scripts/perf_tools.py`); the Swift side is driven by `apple/scripts/perf-compare.sh`.

## What the GPUI numbers are

- **Draw time per frame** (`draw_avg_ms`): devctl `frames 60`. It runs 60 forced draws on the main
  thread, `window.draw(cx)` followed by clearing the arena, timed with `Instant`, with a 2 ms
  pause between draws; the first 3 are dropped; the average and the maximum are written as one
  line `frames n=60 avg=1.58ms max=2.38ms first=...ms` to `frames.txt`. Three runs, the medians
  are reported. A draw builds the view tree, takes the terminal's snapshot (the copy under the
  lock), rebuilds the rows that changed, shapes new words and paints into GPUI's scene. **The Metal
  encoding and the GPU's work happen afterwards, outside the timed region.**
- The animations run with `--fps 0` (new output is always pending, so every forced draw
  re-renders what changed), in a window of 2400 x 1300 points, which gives the terminal **200 x 69
  cells** (JetBrains Mono 14 pt, line height 1.3, ligatures on). The Swift grid is 200 x 69 too:
  the grid, not the window, is what a frame's work depends on.
- **Main-thread busy**: `sample <pid> 5`, the thread `com.apple.main-thread`, the share of samples
  whose innermost frame is not an idle wait (`perf_tools.py`).
- **Throughput**: `ttystorm bench --only logflood,colorflood,cursorstorm --size 16 --runs 3` run
  inside the app, medians in MB/s.

## What the Swift app offers (the contract between the two agents)

A control folder, `WORMHOLE_DEVCTL=<dir>`, off unless the variable is set, and every command
event-driven (a `DispatchSource` on the folder, never a timer): the caller writes `cmd.tmp` and
renames it to `cmd`; the app reads and removes `cmd`, runs its lines, appends results to `out.txt`.

| Command | Does |
|---|---|
| `frames N` | N forced draws on the main thread, 2 ms between them, the first 3 dropped. Each one is: take `frame()` from the source, parse it, rebuild the changed rows, fill the instance buffers, encode and commit the command buffer. Appends one line to `<dir>/frames.txt` (below). |
| `shot <png path>` | the last frame, rendered into an offscreen texture and read back (no Screen Recording permission). |
| `state` | one line: grid, cell size in pixels, scale, frames drawn, whether the display link is running. |
| `quit` | exits cleanly. |

The `frames.txt` line starts exactly like GPUI's, so `perf_tools.py` reads it unchanged, and
carries the Swift-only parts after it:

```
frames n=60 avg=<build ms> max=<build ms> first=<ms> encode_avg=<ms> total_avg=<ms> total_max=<ms> cpu_avg=<ms> gpu_avg=<ms> wait_avg=<ms>
```

- `avg`/`max`/`first`: the **build** part, from just before `frame()` to the end of the instance
  buffer assembly. This is the part comparable to GPUI's `window.draw`.
- `encode_avg`: creating, encoding and committing the command buffer (CPU only).
- `total_*`: build plus encode; what GPUI's timed region leaves out is included, so comparing it with
  GPUI's `draw_avg_ms` is the harsher comparison for Swift.
- `cpu_avg`: the thread's CPU time (`CLOCK_THREAD_CPUTIME_ID`) for build plus encode, against wall time.
- `gpu_avg`: `gpuEndTime - gpuStartTime` of the committed command buffers (GPUI does not report it).
- `wait_avg`: time spent waiting for a free frame buffer or a drawable; excluded from build and
  encode (a GPU-bound wait is not CPU work), reported so it is visible.

Environment the app reads: `WORMHOLE_DATA_DIR` (required, a folder named `Wormhole-swift*`),
`WORMHOLE_SHELL` (the program to run, no arguments), `WORMHOLE_SWIFT_GRID=200x69` (a fixed grid; the
window is sized to fit), `WORMHOLE_BACKGROUND=1` (accessory app, the window opens behind others and
takes no focus).

## Run conditions

The comparison needs a quiet machine and is run one thing at a time, GPUI and Swift back to back:
the load average is printed with each run, and the processes using more than a few percent of a
core are listed. Where the machine was not quiet, the result says so. A number goes in a table
only with the method above.

## Visual comparison (looks)

`apple/scripts/visual-run.sh <scratch dir>` starts a devctl build of the GPUI app and the Swift app as background
instances with scratch data, runs the same scripted terminal content in both (`visual-scenes.py`: colours, styles,
underlines, wide and emoji cells, box/block/braille, cursors, selection, images, command blocks) at the same window
size and scale in dark and light, shoots every Settings page at one window size, and has `visual-compare.py` write a
sheet per scenario (GPUI | Swift | difference) and a table. A cell (block) is the same when its mean colour is within 12 levels in both;
the definition is the header of `visual-compare.py`.

Both apps size the grid by the core's cell (`view::layout::cell`: the font's `m` advance, unrounded; GPUI's `prepare.rs` and Swift's
`FontSet` through the FFI), so JetBrains Mono 14 pt at 2x is 16.8 x 36 px in each, the same window holds the same columns and rows, and
block c of a crop spans the pixels from round(16.8 c) to round(16.8 (c + 1)), where both apps put a cell's edge. Nothing is stretched and
no pixel is left out. Glyphs are rasterised as GPUI's text system does (`GlyphRaster`: an alpha-only bitmap, the face at its point size under the scale, quarter-pixel x variants, and CoreGraphics' stem dilation only for text whose colour's luminance asks for it, in quarters), so light text on a dark terminal and dark text on a light one carry the same ink in both. The Swift shot is the pane as drawn, with its 4 pt padding (8 px), so its grid starts at `--swift-origin 8,8`;
where GPUI's grid starts in its window is found once per run by the `ruler` scene (numbered rows past the bottom and a 300-character
line that wraps at the last column, searched within 4 px), which also proves the two grids are the same size. A scene-wide share cannot
carry a feature (2 % of a grid is a hundred blocks), so each scene names the cells its features occupy (`REGIONS` in `visual-scenes.py`:
the cursor's cell, each picture, the ligature, emoji and Indic rows, each underline style, the selected cells, find's labels) and every
block of every region must pass; their failing blocks are listed in the table. Settings that change the window rather than the grid
(Increase Contrast, Reduce Transparency) are compared by where they change each window (`visual-compare.py delta`: the pixels that
differ between off and on in each app, and how much of that overlaps). The scenes: ruler, text (colours, styles, underline styles,
wide and emoji cells, box, blocks, braille, ligatures, links), pad, the three cursors, images (kitty, iTerm2, Sixel), blocks (OSC 133
gutter), selection, find (the matches in the grid, and the bar in a shot of the whole window), Increase Contrast, text zoom (two steps
in), and the minimap (the grid, and its strip in a shot of the whole window). `SKIP_SETTINGS=1` runs the terminal scenes only. Not compared (a person must look): the native scroll bar (an NSScroller,
not in the Metal shot), the unfocused hollow cursor, a Cmd-hover underline, the context menu (native NSMenu).
