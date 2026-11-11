# Wormhole Swift renderer (experiment)

A small AppKit app whose terminal view is Metal, written in Swift, fed either by a synthetic source or by
the real Rust engine over the FFI. It is a spike renderer, not the product: see "What is missing" below.
This branch is never pushed.

## Run

```sh
cd macos/swift
swift build -c release                      # builds without the FFI: the mock source only
export WORMHOLE_DATA_DIR=/tmp/Wormhole-swift-run   # required; the folder must be named Wormhole-swift*
.build/release/Wormhole --mock mixed        # a window: the gallery on top, scrolling text below
.build/release/Wormhole --help
```

It refuses to start without its own bundle identifier (`app.getwormhole.terminal.swift`, embedded in the
executable) and a data folder called `Wormhole-swift*`. It never reads the real app's folder.

The real engine: `macos/scripts/build-ffi.sh macos/swift` links `WormholeFFI.xcframework` and the generated
bindings into this package, and the next `swift build` adds `Sources/UniFFISource`; then
`WORMHOLE_SHELL=/bin/zsh .build/release/Wormhole` (or `--shell <program>`) runs it on the real terminal.
With the FFI linked the window is the Rust workspace's too: `WorkspaceBridge` keeps the terminal list (order, pins, groups,
splits, numbers, wording) in `wormhole-core`, starts the program of every terminal it opens and feeds each row from it; without
the FFI (or without `--shell`) `MockWorkspaceBackend` stands in over the same `WorkspaceStore`. `WORMHOLE_DEVCTL` gains `rows`,
`rename`, `pin`, `group`, `drop`, `plan`, `split` and `window-shot` (see `AppController+Workspace.swift`).

Several windows: ⌘N (or `Wormhole new-window` over the hook socket) opens another, each over a workspace of its own in the
core (`WindowsBridge` keeps the registry: ids no two windows share, what each held, where each was); "Move group to new window"
takes a group's terminals along running. Windows, frames, columns, groups, splits and the selection are saved as they change
and come back at launch (`WindowSaver`, `AppController+Windows.swift`). `WORMHOLE_DEVCTL` gains `new-window [folder]`,
`sessions`, `use-window <id>` (which window the commands act on), `move-to-window <ids>`, `groupwindow <group>`,
`save-windows` and `saved-windows`.

Screenshots without a window or Screen Recording permission (a texture readback of the same instance data):

```sh
.build/release/Wormhole --render-png out/gallery --mock unicode --grid 100x30           # out/gallery-0001.png
.build/release/Wormhole --render-png out/matrix --mock matrix --frames 40 --png-every 40
```

## Layout

| Target | What |
|---|---|
| `TerminalCore` | the packed frame (reader, writer), the `FrameSource` and `TerminalInteraction` protocols (the pointer, selection, links seam), `MockFrameSource` and `MockInteraction` (scenarios `text`, `flood`, `noise`, `matrix`, `unicode`, `cursor`, `mixed`), the xterm key encoder, the palette. Foundation only. |
| `TerminalMetal` | the view and the renderer: `FontSet` and `GlyphRaster` (CoreText), `WordCache` (shaping once per word), `GlyphAtlas` (R8 coverage and BGRA colour), `BoxDrawing` (exact box, block and braille cells), `RowBuilder` (cells to instances), `TerminalScene` (rows kept between frames), `MetalRenderer` (three layers in one pass, three frames in flight), `MetalTerminalView` (display link), and the view's input in extensions: `Pointer/` (mouse, wheel, selection, links, Command shortcuts, context menu), `Keyboard/` (key table, input method with marked text, paste and drop), `Scroller/` (a real `NSScroller`), `CursorBlink` and `TextBlink`, `TerminalBell`, `TerminalFocus`. A font change swaps the scene's `TextContext` (faces, atlas, word cache, row builder; `TextContexts` keeps the last four): `TerminalFontSettings` from Settings, `MetalTerminalView+Zoom` for ⌘+ ⌘- ⌘0 (`TextZoomEase` in `TerminalCore`); `RowLayout` and `Stretch` draw double-width and double-height rows; `RowMarks` paints the selection, find's matches and the hovered link. |
| `UniFFISource` | `UniFFIFrameSource` over the generated bindings; present only when the FFI is linked. |
| `FilesBridge` | the right panel's `FilesBackend`, `ProjectBackend`, `UsageBackend` and the notes' persistence over `wormhole-core` (`files_panel` in the FFI crate: trust enforced in Rust); `FilesBridge.makeBackends(authenticator:effects:)`. Present only when the FFI is linked. |
| `Wormhole` | the executable: isolation check, options, window, control folder, `--render-png`. |

## How a frame goes

`wake()` (any thread, once per drain) sets an atomic and hops to the main queue, which un-pauses the view's
display link (`NSView.displayLink`). Each tick: `drainEvents()`, then `frame()`, parse, rebuild only the rows
the frame names (plus the cursor's row), copy every row's instances into one of three GPU buffers, encode,
commit. A tick that finds nothing pauses the link. No timer runs while idle; a blinking cursor in the key
window is one one-shot timer per flip.

## Control folder and measuring

`WORMHOLE_DEVCTL=<dir>` turns on a folder the app watches with a dispatch source (never a timer): write
`cmd.tmp`, rename it to `cmd`; results go to `out.txt`. Commands: `frames N`, `shot <png>`, `state`,
`screen`, `type <text>`, `quit`, and the window's: `new`, `close`, `select <id>`, `split [row|down]`, `zoom`, `join <id>`,
`leave`, `focus <left|right|up|down>`, `tab <notes|changes|history|usage|project|files>`, `diff [n]` (the n-th changed file over the
terminals, none closes it), `find [text]`, `notice [n]`, `sidebar`, `panel`, `palette`, `shortcuts`, `settings`,
`action <keymap id>` (any menu action), `menus` (the menu bar as text), `size WxH`, `window-shot <png> [title]` (the window's
chrome; Metal grids are not in it, `shot` writes the grid), `about`, `licenses`, `hosts`, `trust`, `access`,
`updates`, `onboarding`, `quick-panel`, and the look's (`theme`, `appearance`, `status-bar`, `tabs`, `gaps`,
`blur`, `glass`, `dividers`, `opacity`, `reduce-transparency`, `increase-contrast`). `state` ends with the panes
(`panes=row(1,col(14,15)) focused=15 zoomed=none cover=none`). `frames N` follows `macos/MEASURING.md`: forced draws on the main thread, 2 ms
apart, the first 3 dropped, one line appended to `frames.txt`. Its fields (all milliseconds):

- `avg` `max` `first`: **build**, from just before `frame()` to the GPU buffers being complete (fetch, parse
  and row rebuild, buffer assembly). The part comparable to GPUI's `window.draw`.
- `encode_avg`: command buffer creation, encoding, `commit`. `total_*`: build plus encode.
- `cpu_avg`: the thread's CPU time over the whole draw. `gpu_avg`: `gpuEndTime - gpuStartTime`.
- `wait_avg`: blocked on a free frame or a drawable (vsync-paced): not CPU work, in neither build nor encode.
- Then `fetch_avg`, `apply_avg`, `assemble_avg`, rows rebuilt, words shaped, glyphs rasterised, grid.

`scripts/mock-measure.sh` runs the scenarios of the mock this way. Its numbers are the renderer's alone, with
no PTY, no engine and no FFI: they must never be set beside the GPUI app's.

Environment: `WORMHOLE_SHELL` (a program for the real source), `WORMHOLE_SWIFT_GRID=200x69` (fixed grid, window
sized to fit), `WORMHOLE_BACKGROUND=1` (accessory app, window behind others, nothing takes focus),
`--onboarding` forces the welcome (it shows by itself once per data folder, never with `--background`).
`WORMHOLE_FONT_DIR` (the four JetBrains Mono files; found by walking up from the repository otherwise).

## What is missing against the GPUI renderer

- Right-to-left text, cell pinning for combining marks, zero-width characters and grapheme clusters (the frame
  does not carry them yet, contract v1), font fallback beyond the family's `fallbacks` list and what CoreText picks.
- Rounded box corners are square; dashes are plain. A curly underline cut by a double-height row's edge is cropped, not redrawn.
- Images (kitty, Sixel, iTerm2), the minimap, command-block marks, the blurred scrollback
  under the title, transparency and vibrancy.
- Input: a program's own selection that Command-V should replace (needs the OSC 52 clipboard event), command
  blocks and prompt jumps, find; the theme's selection colours reach the renderer only once the host builds
  `TerminalPalette(... selection:selectionText:)` from the theme.
- Subpixel glyph variants (cells are whole pixels, so none are needed), the system's font smoothing is only what a
  grey CoreGraphics context gives, colours set at run time (OSC 4, 10, 11, 12), underline colours (SGR 58).
- A theme (the palette is a fixed dark one), settings, more than one terminal, a window per display scale
  (the scale is fixed at start), live-resize synchronisation (the drawable is resized on layout).
- VoiceOver: the terminal has no accessibility node. The input method is minimal: marked text is drawn at
  the cursor and the candidate window opens at its cell; attributed ranges and reconversion are not done.
- The row cache does not find scrolled rows again by content (a scroll rebuilds every row, from the word
  cache); a word cache hit costs a hash and a compare, not shaping.
