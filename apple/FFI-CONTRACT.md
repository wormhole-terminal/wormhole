# The Rust/Swift contract (wormhole-ffi), version 1

What the Swift side may rely on. It is written from the code (`apple/ffi/src/`); when the code
changes, this file changes in the same commit. Frame layout version: **1** (`frameFormatVersion()`
returns it; check it at start-up and refuse a different one).

## What you link, and where it is

`apple/scripts/build-ffi.sh` builds the Rust crate `wormhole-ffi` (a nested Cargo workspace under
`apple/`, its own `Cargo.lock` and target dir `apple/target/`, git-ignored) and writes, to
`apple/dist/` (git-ignored; in the worktree that built it):

| File | What |
|---|---|
| `dist/WormholeFFI.xcframework` | the static library `libwormhole_ffi.a` (arm64, macOS 26.0 deployment target) with the C header `WormholeFFI.h` and `module.modulemap` (C module name `WormholeFFI`) |
| `dist/WormholeBindings.swift` | the generated Swift (module `WormholeBindings` once it is a target of its own); it does `import WormholeFFI` |

SwiftPM wants a binary target and its sources inside the package folder, so a package links them with
symlinks (`scripts/build-ffi.sh <package dir>...` makes them): `<package>/WormholeFFI.xcframework`
and `<package>/Sources/WormholeBindings/WormholeBindings.swift`. In `Package.swift`
(`apple/ffi-bench/Package.swift` is a working example):

```swift
.binaryTarget(name: "WormholeFFI", path: "WormholeFFI.xcframework"),
.target(name: "WormholeBindings", dependencies: ["WormholeFFI"],
        swiftSettings: [.swiftLanguageMode(.v5)],          // generated code; yours stays Swift 6
        linkerSettings: [.linkedFramework("Security"), .linkedFramework("CoreFoundation"),
                         .linkedFramework("CoreServices"), .linkedFramework("SystemConfiguration"),
                         .linkedLibrary("iconv")]),         // what the Rust crates behind it link
```

Then `import WormholeBindings` in your code. The library built in one worktree can be used from
another by symlinking its `dist/` (an absolute path is fine for a symlink); the Rust crates are
the same source on both branches until the contract changes.

## Isolation: the library refuses to run on real data

`Terminal.start` throws `FfiError.notIsolated` unless the environment variable `WORMHOLE_DATA_DIR`
is set (before the call: `setenv` in Swift's `main`) to a folder whose last component starts with
`Wormhole-swift` (`Wormhole-swift`, `Wormhole-swift-test-7`, ...). Core derives every path it uses
(data folder, control-socket name) from that variable, so the check is on what core will use.
The Swift app should refuse to start the same way, and use its own bundle id
(`app.getwormhole.terminal.swift`).

## The object: `Terminal`

Generated Swift (`open class Terminal: TerminalProtocol, @unchecked Sendable`):

```swift
// start (blocks for a few milliseconds: it spawns the PTY and the program; off the main thread in a real app)
static func start(program: String, args: [String], cols: UInt16, rows: UInt16,
                  cellWidth: Float, cellHeight: Float, listener: TerminalListener) throws -> Terminal

func frame() -> Data              // the rows that changed since the last call (see "Frame")
func fullFrame() -> Data          // every row, whatever changed (a new surface lost its picture)
func drainEvents() -> [TermEvent] // everything queued since the last drain; re-arms wake()
func write(bytes: Data)           // typed or pasted bytes; the view returns to the live screen
func sendKey(key: String, text: String?, shift: Bool, alt: Bool, control: Bool,
             platform: Bool, optionAsMeta: Bool) -> Bool   // see "Keys"
func resize(cols: UInt16, rows: UInt16, cellWidth: Float, cellHeight: Float)
func scroll(lines: Int32)         // positive: up into the history
func scrollToBottom()
func screenText() -> String       // the visible screen as plain text
func isAlive() -> Bool
func close()                      // hangup (3 s grace, on the reader thread: returns at once); explicit, deinit only frees
func frameTiming() -> FrameTiming // Rust-side time of the frames so far (for measuring)

func frameFormatVersion() -> UInt32   // free function
func ping(value: UInt64) -> UInt64    // free function: a call that does nothing
```

`program` is run as is (no login-shell wrapping, no `WORMHOLE_SHELL`): `"/bin/cat"` with `[]`,
`ProcessInfo...["SHELL"]` with `["-l"]`, or `ttystorm` with its arguments. The program gets
`TERM=xterm-256color`, `COLORTERM=truecolor`, `TERM_PROGRAM=Wormhole`. `cellWidth`/`cellHeight` are
points; the engine reports them to programs that ask (XTWINOPS) and fits images to them; they do not
affect the grid.

`FfiError` (Swift `Error`): `.notIsolated(reason:)`, `.start(reason:)`, `.closed`. A Rust panic does
not cross as undefined behaviour: UniFFI turns it into a Swift error/crash at the boundary.

### Threads and locks

- Rust runs two threads per terminal that Swift never sees: the **PTY reader** (reads, parses into
  the screen) and an **event thread** (`wormhole-ffi-events`) that turns the engine's events into the
  queue behind `drainEvents`.
- `TerminalListener.wake()` is called **on the event thread**, with no Rust lock held, **at most once
  between two `drainEvents()` calls** (a flood of a million events is one `wake()`; the first event
  after a drain wakes again). It must return quickly: set a flag and hop to the main queue
  (`DispatchQueue.main.async`), then un-pause the display link. Do not call `frame()` inside it,
  and do not block it on anything the main thread holds.
- Every other method may be called from **any thread** (they lock internally) and is meant for the
  **main thread**: `frame()`, `write`, `sendKey`, `resize`, `scroll` and `drainEvents` are each
  microseconds to a few hundred microseconds (numbers in the spike report). `frame()` and `resize`
  take the engine's **FairLock**, the lock the reader thread parses under; it is handed over in
  asking order, so the main thread waits at most for the one parse slice that holds it (2 to 3 µs
  on average under a flood, measured in `pty/lock.rs`). Never call a `Terminal` method from inside
  `wake()`: the event thread would wait on the main thread's turn.
- `frame()` is serialized by an internal mutex: one consumer per terminal. The engine's damage is
  consumed by the call, so two renderers cannot share one terminal (call `fullFrame()` to give a
  new one its picture, but the first would then miss the damage).
- After `close()`, `frame()` and `fullFrame()` return empty `Data`, other calls do nothing.

### Events: `drainEvents()`

```swift
public enum TermEvent: Equatable, Hashable, Sendable {
    case output                    // new content: take a frame (at most one per drain: "something is new", not a count)
    case title(title: String)      // OSC 0/2
    case resetTitle
    case bell
    case exited(code: Int32?)      // the program is gone; the screen stays readable
}
```

The engine has more events (clipboard, working directory, command marks, notifications, program
status, window operations); they are added as the app needs them, each as a new case (a minor
change: a Swift `switch` needs `@unknown default`, and unknown ones are simply not delivered today).

Protocol for the display link: `wake()` -> un-pause the link; each tick: `drainEvents()` (this
re-arms `wake()`), then `frame()`; when a tick finds nothing to draw, pause the link. An idle
terminal calls nothing and its threads sleep.

### Keys: `sendKey`

`key` follows the app's key table (GPUI's names): `"enter"`, `"tab"`, `"backspace"`, `"escape"`,
`"up"`, `"down"`, `"left"`, `"right"`, `"home"`, `"end"`, `"pageup"`, `"pagedown"`, `"insert"`,
`"delete"`, `"space"`, `"f1"`...`"f24"`, or the single unshifted character (`"a"`, `"["`, `"1"`).
`text` is what the key types with the layout and modifiers (`event.characters`); pass `nil` when
Control is held (as the app does). `platform` is Cmd: a Cmd chord returns `false` and sends nothing
(the application's own shortcuts). `optionAsMeta`: Option+key sends ESC+key. The encoder follows
the terminal's modes (application cursor keys, kitty keyboard, modifyOtherKeys). Returns whether
bytes were sent; sending clears the selection and returns the view to the live screen. For text
that is not a key (IME commit, paste), use `write(bytes:)` (bracketed paste is not wrapped yet).

## Frame

`frame()` returns `Data` of little-endian `u32` words, no padding. UniFFI copies the Rust `Vec<u8>`
into a Swift `Data` (one allocation and one memcpy per frame; the buffer is 16-byte aligned, so
`withUnsafeBytes` + `assumingMemoryBound(to: UInt32.self)` is fine at any word offset).

```
offset  size  field
0       4     magic: 0x31464857 ("WHF1" as bytes)
4       4     format version (1)
8       4     cols
12      4     rows
16      4     changed: number of row records that follow
20      4     flags: bit 0 = full (every row is in this frame: the first, after a resize,
                after a scroll, after the engine cleared history)
24      4     cursor row (0xFFFFFFFF = no cursor: hidden, or scrolled out of view)
28      4     cursor column
32      4     cursor style: bits 0-7 shape (0 block, 1 underline, 2 beam, 3 hollow block);
                bit 8 = over a wide character; bit 9 = the program asked for blinking
36      4     display offset: lines the view is scrolled up into history (0 = live screen)
40      4     history size: lines of history above the live screen
44      4     reserved (0)
48      ...   `changed` row records, ascending by row
```

Row record (`8 + cols * 16` bytes):

```
0   4   row index (0 = top of the viewport)
4   4   row size: 0 normal, 1 double-width, 2 double-height top half, 3 double-height bottom half
8   cols * 16 bytes: cells
```

A frame is **complete for the rows it carries and silent about the others**: keep what you built
for the rows it does not mention. The first frame, and any frame after `cols`/`rows` changed, is
full (reallocate your row storage when `cols` or `rows` differs from yours). **Always read the cursor
and display offset from the header**, even when `changed == 0`: a cursor move alone may not damage
a row. A frame with `changed == 0` is 48 bytes. Scrolling through history re-sends every row (there
is no scroll-delta hint). Whether the engine reported a row damaged does not mean its content
differs; unchanged rows are certainly the same.

### Cell (16 bytes, four `u32`)

```
word 0  char:   the Unicode scalar (a blank cell holds 0x20; the first cell of a wide character holds it)
word 1  fg:     colour (below), as the program set it, not resolved
word 2  bg:     colour (below)
word 3  flags:  the engine's cell flags (below)
```

Colour word: `tag << 24 | value`:

| tag | meaning | value |
|---|---|---|
| 0 | named | palette slot 0...258: 0-15 the ANSI colours (8-15 bright), **256 default foreground, 257 default background, 258 cursor** |
| 1 | indexed | 0...255 (SGR 38;5 / 48;5): the 16 ANSI, then the 6x6x6 cube (levels 0, 95, 135, 175, 215, 255), then 24 greys (8 + 10 i) |
| 2 | truecolor | `r << 16 \| g << 8 \| b` |

Slots and indices share one table: a renderer keeps a 259-entry palette (`0x00RRGGBB`) and looks up
`value` for tags 0 and 1; tag 2 is the colour itself. So a theme change rebuilds no row. The colours
programs set at run time (OSC 4, 10, 11, 12) are **not** in the frame yet (they would be a palette
delta in a later minor). **Reverse video (INVERSE) is the renderer's job**: swap fg and bg of that
cell; `HIDDEN` draws the text in the background colour; `DIM` is the renderer's to apply.

Flags (bit values of the engine's `Flags`):

| bit | name | meaning |
|---|---|---|
| 0 | BOLD | |
| 1 | DIM | |
| 2 | ITALIC | |
| 3 | UNDERLINE | single; the underline styles are one flag each and at most one is set |
| 4 | DOUBLE_UNDERLINE | |
| 5 | UNDERCURL | |
| 6 | DOTTED_UNDERLINE | |
| 7 | DASHED_UNDERLINE | |
| 8 | BLINK | |
| 9 | INVERSE | swap fg and bg |
| 10 | HIDDEN | draw in the background colour |
| 11 | STRIKEOUT | |
| 12 | OVERLINE | |
| 13 | WIDE_CHAR | first of the two cells of a double-width character: it holds the character, draw it two cells wide |
| 14 | WIDE_CHAR_SPACER | the second cell: holds nothing (ignore its char); its fg/bg are the character's |
| 15 | LEADING_WIDE_CHAR_SPACER | a blank at a row's end where a wide character did not fit and moved to the next row |
| 16 | WRAPLINE | on a row's last cell: the line continues in the next row (a soft wrap) |

**Not in version 1** (the renderer must not expect them yet): zero-width and combining characters
(`RowExtras`: the cell carries only its base character), underline colours (SGR 58), hyperlinks,
inline-image placements, command-block spans, selection ranges, the program's OSC palette changes.
Each arrives as an additive change to this file (a new header flag or a second call), with the
version bumped only if an existing field changes meaning.

## What the Rust side does not do for you

Cell size and font metrics, glyph shaping, the palette, blink timing, the scroll bar, selection
drawing and mouse reporting are Swift's (or later seam calls). The seam gives the engine's grid
as it is; the engine does wrapping, reflow on `resize`, scrollback and all escape-sequence meaning.
