# The Rust/Swift contract (wormhole-ffi), version 2

What the Swift side may rely on. It is written from the code (`apple/ffi/src/`); when the code
changes, this file changes in the same commit. Contract version: **2** (`contractVersion()`; 2 added
the `Workspace` object below; a new action or event case is a minor and does not change it). Frame
layout version: **1** (`frameFormatVersion()` returns it; check both at start-up and refuse a
different one).
changes, this file changes in the same commit. Contract version 2 added "Interaction" below (additive; the
frame layout did not change). Frame layout version: **1** (`frameFormatVersion()`
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
func sendKey(press: KeyPress) -> Bool   // see "Keys"
func resize(cols: UInt16, rows: UInt16, cellWidth: Float, cellHeight: Float)
func scroll(lines: Int32)         // positive: up into the history
func scrollToBottom()
func screenText() -> String       // the visible screen as plain text
func isAlive() -> Bool
func close()                      // hangup (3 s grace, on the reader thread: returns at once); explicit, deinit only frees
func frameTiming() -> FrameTiming // Rust-side time of the frames so far (for measuring)

func frameFormatVersion() -> UInt32   // free function
func ping(value: UInt64) -> UInt64    // free function: a call that does nothing
func contractVersion() -> UInt32      // free function: this file's version (2)
```

`program` is run as is (no login-shell wrapping, no `WORMHOLE_SHELL`): `"/bin/cat"` with `[]`,
`ProcessInfo...["SHELL"]` with `["-l"]`, or `ttystorm` with its arguments. The program gets
`TERM=xterm-256color`, `COLORTERM=truecolor`, `TERM_PROGRAM=Wormhole`. `cellWidth`/`cellHeight` are
points; the engine reports them to programs that ask (XTWINOPS) and fits images to them; they do not
affect the grid.

`FfiError` (Swift `Error`): `.notIsolated(reason:)`, `.start(reason:)`, `.closed`. A Rust panic does
not cross as undefined behaviour: UniFFI turns it into a Swift error/crash at the boundary.

`TerminalListener` is a generated Swift protocol `AnyObject, Sendable` with one method, `func wake()`:
implement it in a `final class ... @unchecked Sendable` (or a class holding a `@Sendable` closure). Rust
keeps the listener alive as long as the terminal lives, so capture the view weakly in what it does
(`[weak view]`); otherwise terminal, listener and view hold each other.

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

`KeyPress(key: String, text: String?, shift: Bool, alt: Bool, control: Bool, platform: Bool,
optionAsMeta: Bool)` is a generated record (a struct with a memberwise `init`). `key` follows the app's key table (GPUI's names): `"enter"`, `"tab"`, `"backspace"`, `"escape"`,
`"up"`, `"down"`, `"left"`, `"right"`, `"home"`, `"end"`, `"pageup"`, `"pagedown"`, `"insert"`,
`"delete"`, `"space"`, `"f1"`...`"f24"`, or the single unshifted character (`"a"`, `"["`, `"1"`).
`text` is what the key types with the layout and modifiers (`event.characters`); pass `nil` when
Control is held (as the app does). `platform` is Cmd: a Cmd chord returns `false` and sends nothing
(the application's own shortcuts). `optionAsMeta`: Option+key sends ESC+key. The encoder follows
the terminal's modes (application cursor keys, kitty keyboard, modifyOtherKeys). Returns whether
bytes were sent; sending clears the selection and returns the view to the live screen. For text
that is not a key (IME commit, paste), use `write(bytes:)` (bracketed paste is not wrapped yet).

## The object: `Workspace`

One window's terminals, groups, pins and splits, and what the sidebar lists of them. It wraps core's
sans-IO `WorkspaceModel` (`crates/wormhole-core/src/workspace/`, the rules are core's `view::sidebar`
and `view::panes`) behind one mutex. Swift sends **actions**, Rust answers with **events**, and the
UI **pulls snapshots** when a revision moved: it never re-implements a rule (order, pins, numbers,
drop plans, selection after a close, second lines, VoiceOver words).

```swift
init(config: WorkspaceConfig, listener: WorkspaceListener)   // WorkspaceConfig(home:, numbersFollowList:, startFolder:, paneHeaderHeight:)

func apply(action: Action) -> [WorkspaceEvent]     // the events of THIS call come back from it
func drain() -> [WorkspaceEvent]                   // events caused elsewhere (a terminal's thread); re-arms wake()
func sidebar() -> SidebarSnapshot                  // words as they read now (the workspace's clock)
func sidebarAt(nowMs: UInt64) -> SidebarSnapshot   // ... as they read at a given time (tests, previews)
func panes(width: Float, height: Float) -> PanesSnapshot
func chrome() -> ChromeSnapshot
func revisions() -> Revisions                      // sidebar / layout / chrome counters
func selected() -> Int64?
func dropPlan(dragged: Int64, over: Over) -> DropPlan?   // pure: the line a drag would drop on
func setFacts(id: Int64, facts: TerminalFactsRecord) -> [WorkspaceEvent]   // whole replacement (tests, previews)
func setGit(id: Int64, branch: String?, changes: UInt32?) -> [WorkspaceEvent]
func registerTerminal(id: Int64, terminal: Terminal)   // after Opened and Terminal.start: feeds the row; Closed ends it
func terminal(id: Int64) -> Terminal?
func nowMs() -> UInt64
```

`WorkspaceListener` has one method, `wake()`, with the same rules as `TerminalListener`'s: called from
a Rust thread with no lock held, at most once between two `drain()` calls, and it must only note the
wake and hop to the main queue. **One handler** serves both sources of events: `apply`'s result and
`drain()`'s.

### Events: `WorkspaceEvent`

| Case | Meaning, and what Swift does |
|---|---|
| `opened(id, cwd)` | a terminal was made: `Terminal.start(...)` a program for it (in `cwd` when there is one), then `registerTerminal(id:terminal:)` |
| `closed(id)` | the terminal is gone; its registered `Terminal` has already been closed by Rust. Drop its view |
| `selected(id)` | the selection moved (`nil`: none) |
| `moved(id)` | a row moved or joined a split: animate it (`Landing`/zone change, by id) |
| `listing` | the rows, groups or numbers changed: pull `sidebar()` |
| `layout` | splits or the panes' focus changed: pull `panes(width:height:)` |
| `chrome` | sidebar/panel visibility, panel tab, search or overlay changed: pull `chrome()` |
| `refused(reason)` | the action changed nothing (`unknown`, `tooSmall`, `unchanged`): data, not an error |

Events are coalesced within a call (`listing`/`layout`/`chrome` at most once, `selected` last wins).

### Actions: `Action`

Ids are `Int64` (a terminal's id is its session's; split and terminal ids share one counter and never
repeat), groups are `UInt32` (a group's id is stable while it exists).

| Group | Cases |
|---|---|
| Selection | `select(id)`, `selectStep(delta)` (next/previous as listed, wraps, skips folded), `selectNumber(n)` (⌘1...⌘9; a number past the last selects the last) |
| Terminals | `newTerminal` (in the selected one's folder, selected, search ends), `close(id)` (the app's rule: selection by the window's order, a split's heir; closing the last starts a new one), `rename(id, name)` (blank gives the name back to what runs in it), `setPinned`, `togglePin` (a split is pinned as a whole) |
| Groups | `newGroup(members)` (with the rest of their splits; needs terminals that exist), `renameGroup` (blank keeps the old name), `toggleGroup` (fold), `ungroup`, `setGroupPinned`, `addToGroup(id, group)`, `removeFromGroup(id)` |
| Moves | `dropTerminal(dragged, over)`, `dropGroup(group, over)` (core's `place` rules; `Over` is by id: `row(id, part)`, `head(group, part)`, `after(card)`, `tail`), `moveTerminal(id, landing)`, `moveGroup(group, landing)` (`Landing.top(before: Item?)` / `.group(group, before: Int64?)`) |
| Splits | `splitBeside(anchor, axis)` (a new terminal in the anchor's folder, selected; refused `tooSmall` without room), `joinSplit(id, target, zone)`, `leaveSplit(id)`, `focusPane(id)`, `toggleZoom` |
| Chrome | `toggleSidebar`, `togglePanel`, `showPanelTab(tab)` (shows the panel), `search(text)`, `showOverlay(overlay?)`, `setPaneArea(width, height)` (zero until laid out: nothing is refused for want of room) |

### Snapshots

All are Swift value types with the fields named here, built on demand and never mirrored.

- `SidebarSnapshot(revision, selected: Int64?, sections: [SidebarSection], terminalCount, refreshInMs: UInt64?)`:
  `refreshInMs` is how long until words that move with time ("Working · 3m", a command's clock) read
  differently: arm **one** timer for it, `nil` asks for none.
- `SidebarSection(key, head: GroupHead?, rows: [SidebarRow], terminalCount, alertCount)`: `head == nil`
  is a run of terminals in no group. `rows` is empty while the group is folded (`terminalCount` still says
  what is folded away; `alertCount` is the needs-input and failed ones, for the head's dot). While a search
  is on, only matching rows and sections that still have one are listed.
- `GroupHead(id: UInt32, name, color, pinned, folded, folder, position, of, spoken)`.
- `SidebarRow(id, name, number: UInt32?, keycap: String?, selected, pinned, state: RowState, agent: AgentInfo?, message, branch, folder, unread, bell, failureCode: Int32?, changes: UInt32?, notes, secondLine: SecondLine, split: SplitInfo, position, of, spoken)`:
  `number` is the ⌘ number the row holds (kept with the terminal under "Stay with the terminal"; they do
  not renumber under a search, as ⌘n still selects the same terminal). `spoken` is the whole VoiceOver label.
  `RowState` is `needsYou, working, finished, failed, idle, exited`.
- `SecondLine(text, tone: Tone, mono, trail)`: `Tone` is `plain, attention, done, failed`; `mono` means a
  command (code font); `trail` sits at the line's right.
- `SplitInfo(id, bracket: Bracket?, chain, onScreen, members)`: `bracket` is `start/mid/end` along the rows of a
  split whose members are next to each other, `chain` a member that is not together in the list.
- `PanesSnapshot(revision, split: Int64?, panes: [PaneRect(id, x, y, w, h, focused)], focused, zoomed)`:
  the selected terminal's split in reading order, or the zoomed pane alone, or the lone terminal.
- `ChromeSnapshot(revision, sidebarVisible, panelVisible, panelTab: TabId, search, overlay)`; `TabId` is
  `changes, history, notes, usage, project, files`.

### What feeds a row

`registerTerminal` makes the terminal's own events facts of its row, on its event thread: the title its
program set (`title`), the shell's folder (OSC 7), the program in front and when it started (OSC 133 marks),
a command that failed with its exit code, the bell and output nobody has seen while another terminal
is selected. Selecting a terminal (and the panes shown with it) hears the bell and spends a finished or failed
result. What does not feed yet: a coding agent's status (its hooks), git (`setGit`), notes (`setFacts`).

### Threads and locks

- `apply`, `drain`, the snapshots and `dropPlan` lock the model for microseconds; they may be called
  from any thread and are meant for main. The model's lock is **never held** across `wake()`, a call into a
  `Terminal`, or a system call: a terminal's thread probes the process first, then locks, changes, unlocks and
  only then wakes.
- A registered `Terminal` keeps its own hot path (`frame`, `sendKey`, `resize`): none of it takes the model's lock.
- `Closed` is the one place a registered terminal's program ends: Rust calls its `close()`.
## Interaction: pointer, wheel, selection, links, paste, focus

`apple/ffi/src/interaction/`. The app's behaviour for what a person does besides typing a key, written once
over `wormhole-term`'s encoders; the Swift view only turns `NSEvent`s into these calls. **Positions are
viewport cells** (0-based, row 0 at the top of what the last frame showed, columns from the left); Rust
turns them into grid lines with the last frame's display offset, so take a frame before relying on them.

```swift
func reportPointer(event: PointerEvent) -> Bool     // the program took it (mouse modes 9, 1000...1006, 1016, locator)
func takesPointer() -> Bool                         // a program asked for the mouse: a click is not a selection
func scrollWheel(step: WheelStep)                   // see below
func paste(text: String)                            // bracketed when the program asked; view returns to the live screen
func focusChanged(focused: Bool)                    // DECSET 1004 reports
func droppedText(paths: [String]) -> String         // shell-escaped paths, each followed by a space
func clearHistory()

func selectionStart(kind: SelectionKind, row: UInt32, col: UInt32, side: CellSide)  // .cell .word .line .block
func selectionUpdate(row: UInt32, col: UInt32, side: CellSide)
func selectionFinish()                              // a click without a drag selects nothing
func selectAll()
func selectionText() -> String?
func selectionSpans() -> [RowSpan]                  // per viewport row of the LAST frame: columns first...last (inclusive)

func linkAt(row: UInt32, col: UInt32) -> LinkHit?   // OSC 8 hyperlink, else a URL in the text; spans per viewport row
func openTarget(uri: String) -> OpenTarget          // .url (http, https, mailto) / .reveal(path) (file: on this host) / .refused
```

- `PointerEvent { button: .left/.middle/.right/.none, action: .press/.release/.motion, col, row, x, y, shift,
  alt, control }`; `x`/`y` are points from the text area's corner, used only when the program asked for
  SGR pixels (1016). `reportPointer` returns `false` when the program did not ask for the mouse or Shift is
  held (Shift takes the pointer back for selecting): the view then selects. Motion within one cell is
  reported once (consumed, nothing sent) unless the program wants pixels.
- `WheelStep { lines, columns, col, row, x, y, shift, alt, control }`: movement in **cells** (positive: up,
  left; fractional, Rust keeps the remainder so a trackpad's steps add up). A program that asked for the mouse
  hears wheel buttons (at most 10 per call); on the alternate screen with alternate-scroll the arrow keys
  (application cursor keys honoured); otherwise the history scrolls. Shift always scrolls the history.
- `selectionSpans()` reads the snapshot the last `frame()` took, not the engine: call it after the frame it
  belongs to. A selection moves with the text when output scrolls, and typing clears it (`sendKey`), so keep
  calling it after each drawn frame while one exists.
- The scroll bar needs no call: `history size` and `display offset` are in every frame header; scrolling to an
  offset is `scroll(lines: offset - current)`.
- Cursor shape, blink request and wide-ness are in the frame header; the bell is `TermEvent.bell`.
- Not here yet: a program's selection that ⌘V should replace (needs the OSC 52 clipboard event), command
  blocks, find.

Tests: `apple/ffi/tests/interaction.rs` runs each against the real engine and a PTY (kitty keyboard,
modifyOtherKeys and Option-as-Meta included); `apple/.cargo/config.toml` gives them an isolated data folder.

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

**Not in frame version 1** (the renderer must not expect them yet): zero-width and combining characters
(`RowExtras`: the cell carries only its base character), underline colours (SGR 58), hyperlinks,
inline-image placements, command-block spans, selection ranges, the program's OSC palette changes.
Each arrives as an additive change to this file (a new header flag or a second call), with the
version bumped only if an existing field changes meaning.

## What the Rust side does not do for you

Cell size and font metrics, glyph shaping, the palette, blink timing, the scroll bar, selection
drawing and mouse reporting are Swift's (or later seam calls). The seam gives the engine's grid
as it is; the engine does wrapping, reflow on `resize`, scrollback and all escape-sequence meaning.
