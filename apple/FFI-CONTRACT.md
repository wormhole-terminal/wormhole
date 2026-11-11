# The Rust/Swift contract (wormhole-ffi), version 2

What the Swift side may rely on. It is written from the code (`apple/ffi/src/`); when the code
changes, this file changes in the same commit. Contract version: **2** (`contractVersion()`; 2 added
the `Workspace` object below; a new action or event case is a minor and does not change it). Frame
layout version: **2** (`frameFormatVersion()` returns it; check both at start-up and refuse a
different one; 2 added the colours programs set, "Round S3, engine" below).
changes, this file changes in the same commit. Contract version 2 added "Interaction" below (additive; the
frame layout did not change). Frame layout version: **2** since round S3 (`frameFormatVersion()`
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

static func startIn(program: String, args: [String], cwd: String?, cols: UInt16, rows: UInt16,
                    cellWidth: Float, cellHeight: Float, listener: TerminalListener) throws -> Terminal
                                  // `start` in a folder (a missing one starts where the process is): what `opened(id, cwd)` asks for

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
func terminals() -> TerminalsSnapshot              // EVERY terminal (folded ones too), counts; revision `terminals`
func title(bootVolume: String) -> TitleSnapshot    // the selected terminal's title row as data
func revisions() -> Revisions                      // sidebar / layout / chrome / terminals counters
func selected() -> Int64?
func dropPlan(dragged: Int64, over: Over) -> DropPlan?   // pure: the line a drag would drop on (folder fences included)
func groupDropPlan(group: UInt32, over: Over) -> Landing? // pure: where a dragged group would land
func edgeDrag(edge: Edge, extent: Float) -> EdgeDrag      // pure: where a drag of a column's edge would leave it, with its readout
func canLayout(preset: Preset) -> Bool                    // pure: the area has room for the layout's panes
func groupFolderOffer(group: UInt32) -> String?           // pure: the folder to ask about before a terminal opens in a group with none
func setFacts(id: Int64, facts: TerminalFactsRecord) -> [WorkspaceEvent]   // whole replacement (tests, previews)
func setGit(id: Int64, branch: String?, changes: UInt32?) -> [WorkspaceEvent]   // previews and tests only: a registered terminal's git is read by the workspace
func hook(id: Int64, agent: String, event: String, message: String?) -> [WorkspaceEvent]   // an agent's hook, see "What feeds a row"
func setWindowState(active: Bool, seen: Bool) -> [WorkspaceEvent]   // the window in front / visible at all
func setNotes(id: Int64, tasksDone: [Bool], notes: [String]) -> [WorkspaceEvent]   // what the row's notes mark counts
func retryGit()                                    // git was missing and may be there now
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
| `savedTextForgotten(id)` | "Forget text" was set on the terminal: erase what was saved of its screen |
| `groupToWindow(group, terminals)` | the group is to move to a window of its own: open it; `terminals` are the group's and their splits' (nothing moves in this model yet) |
| `refused(reason)` | the action changed nothing (`unknown`, `tooSmall`, `unchanged`): data, not an error |

Events are coalesced within a call (`listing`/`layout`/`chrome` at most once, `selected` last wins).

### Actions: `Action`

Ids are `Int64` (a terminal's id is its session's; split and terminal ids share one counter and never
repeat), groups are `UInt32` (a group's id is stable while it exists).

| Group | Cases |
|---|---|
| Selection | `select(id)`, `selectStep(delta)` (next/previous as listed, wraps, skips folded), `selectNumber(n)` (⌘1...⌘9; a number past the last selects the last) |
| Terminals | `newTerminal` (in the selected one's folder, selected, search ends), `close(id)` (the app's rule: selection by the window's order, a split's heir; closing the last starts a new one), `rename(id, name)` (blank gives the name back to what runs in it), `setPinned`, `togglePin` (a split is pinned as a whole), `setAgentHidden(id, hidden)` / `toggleAgentHidden(id)` ("Hide from Agents": the model keeps and shows it, the shell enforces it), `setForgetText(id, forget)` / `toggleForgetText(id)` (turning it on also emits `savedTextForgotten`) |
| Groups | `newGroup(members)` (with the rest of their splits; needs terminals that exist), `newTerminalInNewGroup`, `newTerminalInGroup(group)` (last in it, in its folder, else where it was last used, else beside the selected one; selected), `renameGroup` (blank keeps the old name), `toggleGroup` (fold), `ungroup`, `setGroupPinned`, `addToGroup(id, group)` (refused `unchanged` by a group that keeps only its folder's terminals), `removeFromGroup(id)`, `setGroupColor(group, color: GroupColor?)` (the nine names), `setGroupCustomColor(group, r, g, b)`, `setGroupFolder(group, folder: String?)` (a folder pins the group; clearing counts as saying no), `setOnlyInFolder(group, on)` (the terminals outside leave now), `declineGroupFolder(group)`, `closeGroup(group)` (closes every terminal, the group goes unless pinned; the shell asks first when `workingCount > 0`), `moveGroupToWindow(group)` (emits `groupToWindow`) |
| Moves | `dropTerminal(dragged, over)`, `dropGroup(group, over)` (core's `place` rules; `Over` is by id: `row(id, part)`, `head(group, part)`, `after(card)`, `tail`), `moveTerminal(id, landing)`, `moveTerminalAs(id, landing, leaveSplit, withMates)` (a `dropPlan`'s `move` applied as it said), `moveGroup(group, landing)` (`Landing.top(before: Item?)` / `.group(group, before: Int64?)`) |
| Splits | `splitBeside(anchor, axis)` (a new terminal in the anchor's folder, selected; refused `tooSmall` without room), `joinSplit(id, target, zone)`, `showBeside(id, other)` (other joins id's split on its right), `leaveSplit(id)`, `focusPane(id)`, `toggleZoom`, `applyLayout(preset)` (View → Layout: new terminals fill the slots, the panes over leave the split and keep running), `dragDivider(path, index, position)`, `equalizeDivider(path, index)` |
| Chrome | `toggleSidebar`, `togglePanel`, `showPanelTab(tab)` (shows the panel, the sidebar steps aside when there is no room; a tab whose feature is off is refused), `search(text)`, `showOverlay(overlay?)`, `setPaneArea(width, height)` (zero until laid out: nothing is refused for want of room), `setWindowWidth(width)`, `dragEdge(edge, extent)`, `resetEdge(edge)`, `setFeature(feature, on)` (a tab comes or goes), `setTitleRow(parts)`, `setNumbersFollowList(bool)` (⌘ numbers follow the list, or stay with their terminal) |

### Snapshots

All are Swift value types with the fields named here, built on demand and never mirrored.

- `SidebarSnapshot(revision, selected: Int64?, sections: [SidebarSection], terminalCount, refreshInMs: UInt64?)`:
  `refreshInMs` is how long until words that move with time ("Working · 3m", a command's clock) read
  differently: arm **one** timer for it, `nil` asks for none.
- `SidebarSection(key, head: GroupHead?, rows: [SidebarRow], terminalCount, alertCount, tiles: [MemberTile], alert: Urgency?, workingCount)`:
  while the group is **folded** `tiles` is what the head draws in the place of rows (every terminal that passes
  the search, the loudest first: `MemberTile(id, name, glyph: TileGlyph, state, agent, urgency)`), `alert` is
  the loudest alert for the head's dot (`needsYou` or `failed`), `workingCount` is what to ask about before
  closing the group. `head == nil`
  is a run of terminals in no group. `rows` is empty while the group is folded (`terminalCount` still says
  what is folded away; `alertCount` is the needs-input and failed ones, for the head's dot). While a search
  is on, only matching rows and sections that still have one are listed.
- `GroupHead(id: UInt32, name, color, pinned, folded, folder, position, of, spoken, folderPath: String?, onlyInFolder)`:
  `folder` is as the list shows it (`~/app`), `folderPath` the full path.
- `SidebarRow(id, name, number: UInt32?, keycap: String?, selected, pinned, state: RowState, agent: AgentInfo?, message, branch, folder, unread, bell, failureCode: Int32?, changes: UInt32?, notes, secondLine: SecondLine, split: SplitInfo, position, of, spoken, repo: String?, dirty, ahead: UInt32, behind: UInt32, repoFiles: UInt32?, glyph: TileGlyph, agentHidden, forgetText)`
  (the last eight are additive, still contract version 2):
  `number` is the ⌘ number the row holds (kept with the terminal under "Stay with the terminal"; they do
  not renumber under a search, as ⌘n still selects the same terminal). `spoken` is the whole VoiceOver label.
  `RowState` is `needsYou, working, finished, failed, idle, exited`.
- `SecondLine(text, tone: Tone, mono, trail)`: `Tone` is `plain, attention, done, failed`; `mono` means a
  command (code font); `trail` sits at the line's right.
- `SplitInfo(id, bracket: Bracket?, chain, onScreen, members)`: `bracket` is `start/mid/end` along the rows of a
  split whose members are next to each other, `chain` a member that is not together in the list.
- `PanesSnapshot(revision, split: Int64?, panes: [PaneRect(id, x, y, w, h, focused)], focused, zoomed, dividers: [DividerInfo(path, index, axis, position, x, y, w, h)])`:
  the selected terminal's split in reading order, or the zoomed pane alone, or the lone terminal. A divider is
  named to `dragDivider`/`equalizeDivider` by `path` and `index`.
- `ChromeSnapshot(revision, sidebarVisible, panelVisible, panelTab: TabId, search, overlay, panelWanted, panelToggle: PanelToggle, windowWidth, sidebarWidth, terminalWidth, panelWidth, sidebarChosenWidth, panelChosenWidth, sidebarMaxNow, panelMaxNow, limits: ColumnLimits, tabs: [PanelTab(tab, key, label, enabled, selected)], layout: LayoutInfo(presets: [PresetInfo(preset, title, slots, current, enabled)], canSplitRow, canSplitColumn, paneCount, zoomed), selected, zoomed, numbersFollowList)`;
  `TabId` is `changes, history, notes, usage, project, files`. The columns are core's `view::layout::Panes`
  fitted to the window's width (`setWindowWidth`): a hidden or yielding column is 0 wide, the panel gives way
  first and `panelToggle` says `open`, `closed(hidesSidebar)` or `waiting` (wanted, no room). `EdgeDrag(open, width, stop: Stop?, readout)`
  from `edgeDrag(edge:extent:)` is the live readout of a drag ("430 · max", "Close") without moving anything.
- `TerminalsSnapshot(revision, selected, terminals: [TerminalSummary], counts: TerminalCounts(needsYou, working, finished, failed, quiet))`:
  **every** terminal in the list's order, folded groups' too (`TerminalSummary.folded`, no `number` then),
  for the palette, the status bar and menus. `revision` is `Revisions.terminals`.
- `TitleSnapshot(revision, terminal: TitleTerminal?, left: [TitleSlot], right: [TitleSlot], hidden: [TitlePart], pathSteps: [PathStep(parents, last)], pathMenu: [PathFolder(path, name)])`:
  the parts of the title row before and after its flexible space, each `TitleSlot(part, present, chevronBefore)`
  (`present` says the model has something for it; gauge, ports and the pull request come with their feeds).
  `pathSteps` are the ways to draw the folder, widest first: measure each in the row's fonts and give the widths
  to core's `fit_row` (an FFI function is still to come). Name clashes in Swift: `Axis`, `Edge` and `Part` also exist in SwiftUI; qualify them.

Contract version stays 2: everything above is additive (new cases, new records, fields at the end of
existing records). Added with phase 2b: `Revisions.terminals`; `SidebarRow.{glyph, agentHidden, forgetText}`;
`GroupHead.{folderPath, onlyInFolder}`; `SidebarSection.{tiles, alert, workingCount}`; `PanesSnapshot.dividers`;
the new `ChromeSnapshot` fields; the `Action` and `WorkspaceEvent` cases and the snapshots, queries and
records named above.

### What feeds a row

`registerTerminal` makes the terminal's own events facts of its row, on its event thread, through core's
**status machine** (`workspace::StatusMachine`, the rules of the app's `terminal_view/status.rs`):

- the title its program set (`title`, renamed at most every 250 ms), the shell's folder (OSC 7), the program in
  front and since when (a look at the process after output, after the shell's OSC 133 marks, every 0.4 s in a
  terminal's first 3 s and then every 5 s: the system sends no event for it), a command that failed with its exit code (`failureCode`; ⌃C is none) and the
  command that just finished while its result is unseen (it still names the terminal);
- the **coding agent** in front (found by its process: `agent`), its **status** from its hooks (`hook(...)`:
  `UserPromptSubmit` working, `PermissionRequest` needs input with the prompt's words, `Stop` finished, ...;
  a turn that ends while background work still runs is still working), or, for an agent without hooks, from
  the output (working while it prints, finished after 1.8 s of quiet, needs input when it rings the bell) and
  for any program that reports its own status (OSC 7501): `state` is `RowState`, `message` the words;
- the bell and output nobody has seen while the terminal is not in front; what answers the person's typing,
  a resize or a focus change is not work;
- selecting a terminal (and the panes shown with it), and the window coming to the front
  (`setWindowState(active: true, ...)`), **looks**: a finished or failed result is spent and a bell heard;
  a question the agent asks stays until it is answered (Return, a choice or Esc typed into the terminal
  answers it: `Terminal.sendKey` tells the machine).

**Git** is read by the workspace itself for every terminal's folder (`git::status_summary` through core's
hardened `GitCmd`, two worker threads, a hung repository blocks only its own read): `branch`, `repo` (the
repository's root: rows with the same one share it), `dirty`, `ahead`/`behind` its upstream, `repoFiles` (files
changed in the repository) and `changes`, the files **this terminal** changed (`git::attribution`: whose they are
from when each terminal was working; a file first seen in the first picture of a repository is nobody's). A
repository is followed through its files (FSEvents, `git::watch` rules, 150 ms debounce) so a read comes when
something changed; the periods (1.2 s in front without a watch, 30 s with one, longer behind) are the backstop.
A folder in no repository has `repo == nil` and costs a `stat` walk, not git.

What does not feed yet: the hook **socket** (a hook script's `Wormhole hook` reaches the app's control socket,
which the Swift app does not serve yet: the app that serves it calls `hook(...)`), the Settings that decide what a
row counts (plain terminals' working: finishing commands; shared files: every terminal counts them), the
Changes panel's full status, and notes (`setNotes`).

### Threads and locks

- `apply`, `drain`, the snapshots and `dropPlan` lock the model for microseconds; they may be called
  from any thread and are meant for main. The model's lock is **never held** across `wake()`, a call into a
  `Terminal`, or a system call: a terminal's thread probes the process first, then locks, changes, unlocks and
  only then wakes.
- A registered `Terminal` keeps its own hot path: `frame` takes none of the model's locks; `write`, `sendKey` and
  `resize` tell the status machine what the person did (typing makes the program echo, which is not work), under
  the model's lock for microseconds and with nothing else held.
- One **pump thread** per workspace sleeps until the next deadline (a busy terminal's quiet, a title's turn, a
  look at a process, a git read, a debounce) and is woken by what may move one; it holds the workspace weakly and
  ends with it (`Workspace` deinit). Its events are queued behind `wake()` like any other thread's.
- `Closed` is the one place a registered terminal's program ends: Rust calls its `close()`.
## Interaction: pointer, wheel, selection, links, paste, focus

`apple/ffi/src/interaction/`. The app's behaviour for what a person does besides typing a key, written once
over `wormhole-term`'s encoders; the Swift view only turns `NSEvent`s into these calls. **Positions are
viewport cells** (0-based, row 0 at the top of what the last frame showed, columns from the left). Rust
turns them into grid lines: a selection with the engine's display offset now (a drag that scrolled the
history follows it before the next frame is taken), a link with the last frame's (what the person points at).

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

## Files, Project, Usage and Notes (the right panel's seams)

`apple/ffi/src/files_panel/`. Four objects over `wormhole-core`, each behind a Swift protocol of `PanelModel`
(`FilesBackend`, `ProjectBackend`, `UsageBackend`) or the Notes tab's persistence; `Sources/FilesBridge` implements
them (`FilesBridge.makeBackends(authenticator:effects:)`). Every constructor throws `FfiError.notIsolated` unless
the data folder is `Wormhole-swift*` (the stores, the pins and the reports live in it). **Every call but the
constructors and `drain`/`stop` blocks on the disk (or, for `approve`, on the owner): run it off the main thread**
(the bridge does). All paths from Swift are text; a name that is not UTF-8 reads with U+FFFD, so a path made from it
names nothing and every call answers "gone".

**`FilesService`**: `list(root, folder) throws(FilesFailure)` (`""` is the root; the folder's entries sorted folders
first in natural order, labels cleaned of control and bidi characters, cut at core's 5,000 with `more`/`atLeast`),
`read(root, path) -> FileContentRecord` (text capped at 2 MiB with `truncated`, images up to 50 MiB, binary by NUL;
`unavailable(gone|outside|notAFile|denied|other)`), `rename(root, path, name) throws(RenameFailure) -> newPath`
(`name|exists|denied|gone|other`; pins of the renamed folder follow), `pins()`, `setPinned(folder, pinned)`,
`watch(root, listener) -> FolderWatch`. Paths are relative to `root` and plain (no `..`, not absolute); a link is
never walked through, a file link out of `root` reads `outside`, a rename goes through no link and replaces nothing.
Pins are `files_pinned` in `config.json`, each edit through a fresh read of the file.

**`FolderWatch`**: FSEvents on a local volume, plus a timer thread as the fallback (2 s without a stream, 30 s with
one). `PanelListener.wake()` is called at most once between two `drain()`s (same rules as `TerminalListener`); the
bridge turns it into an `AsyncStream<Void>` coalesced to one element per 250 ms. `isLive()` says whether events are
followed; `stop()` (or dropping it) ends it.

**`ProjectService(authenticator)`**: `project(folder) -> ProjectStateRecord?` (the `.wormhole/` governing a terminal's
folder, found as the app does: up to the repository's root; trust, what it would run, the switches with their hints,
and the actions, prompts and layout only where they are allowed), `folderPaint(folder)`, `setFolderColor(folder,
colour?)` (a name or `#rrggbb`; the person's own in `config.json`, or the project's `project_color` by a splice that
keeps comments; a locked or unreadable project is `refused`), `createProject(folder)`, `setFeature(root, key, on)`,
`approve(root) -> approved|declined|failed`, `revoke(root)`, `runAction(root, id)`, `sendPrompt(root, id)`,
`promptText(root, id)`, `openLayout(root)`, `watch(folder, listener)` (the project's folder, and the service's own
changes of trust and switches), and the notes.md sync below. Errors are `ProjectFailure` (`refused(reason)`,
`notFound(what)`, `failed(message)`).
**Trust is enforced here.** An action, prompt or layout acts only where the file is there, the person has it on and
the content is approved (core's `TrustStore`, bound to a fingerprint of everything that can run or type something): a
snapshot is read at the moment of the call and the verdict and the content come from the same bytes. What the
service says is *effects* (`command(place, command)`, `paste(place, text, submit)`, `openUrl`, `openTerminals`),
data for the window to perform; Rust never types into a terminal. `approve` asks the owner through the
`OwnerAuthenticator` Swift gives (Touch ID, an Apple Watch or the password: it blocks the calling thread until the
person answers) and trusts exactly what the tab last showed, so a file changed meanwhile reads `changed`. The
bridge wires `Authenticating`, which answers from `WORMHOLE_AUTH` in a test instance. Reads of project files are
capped and never follow a link out of the project (a link out is an error line, and the file can neither run nor
be trusted).

**Notes**: `SessionNotesStore` (`load(session)`, `save(session, NotesSaved)`) keeps a terminal's notes, steps and folds
in the app's own tables, without the rest of the terminal's record (core's `Store::save_session_notes`; a blank entry
is never saved; the folds are written only once the window has saved that terminal's row). `ProjectService.syncNotes(
session, root, local, editing) -> NotesSyncOutcome` is one step of the three-way merge with the project's `notes.md`
(core's `plan_sync`), only where the person wants it and the project is trusted: untrusted, the file is neither read
nor written and the Project tab says so. `apply` is the document to show instead and `noteSlots` says which existing note
each applied note is (so an editor and its fold survive). Also `notesSessionClosed(session)` and
`notesIdsStartAt(firstFree)` for the merge bases. The bridge's `RustNotesPersistence` does load, save and `sync`.

**`UsageService(listener)`**: `reports()` (latest per agent, kept across restarts in the database's `meta`),
`recordStatusLine(input, now)` (Claude Code's status line JSON), `recordHook(agent, event, input, home, now)` (Codex,
Gemini; a transcript is read only from under `home`), `recordReport(json)` (a `Usage` as the app's hook socket carries
it), `drain()`; `wake()` at most once until the drain. Where the reports arrive from (the status line command, the
hooks) is the host's socket: it hands the JSON to these.
## Auxiliary windows: update, session hosts, project trust, agent access, first run, licenses

Additive (no contract version change). Code: `ffi/src/aux_backends/`; Swift side: the `AuxBridge` module
(`swift/Sources/AuxBridge`, linked builds only) implements `AuxModel`'s protocols over these objects.

**Shape of every object.** A constructor that touches the data folder takes `data_dir: Option<String>`: `None` is
core's own, which must be `Wormhole-swift*` (`WORMHOLE_DATA_DIR`, as for `Terminal`); a given folder must be named
`Wormhole-swift*` too (tests and previews pass their own). Failures are `AuxError` (`NotIsolated`, `Failed{reason}`).
Each object takes an `AuxListener` (`wake()`): called on a Rust thread, no lock held, **at most once until the object's
state is read again** (`state()` / `report()` / `review()` / `next_question()` re-arm it); it must only note the wake and
hop to the main queue. Blocking calls (`HostsDesk.report`, `FirstRun.scan_agents`, `import_scan`) run off the main thread.

| Object | What it is | Calls |
|---|---|---|
| `UpdateEngine` | core `update`: feed (`<base>/<channel>/latest.json`, GitHub fallback), version compare, download with size + sha256 + minisign check against the key given, bundle swap beside the running one, relaunch helper | `new(UpdateSetup, listener)`, `state() -> UpdateStateRecord`, `check(manual)`, `install(version)`, `skip(version)`, `set_channel(Stable\|Preview)`, `next_check_in_seconds()`, `due_at_launch()`, `restart()` |
| `HostsDesk` | core `ipc::hosts` survey + report, said in the catalog's words (cards come with their facts as text) | `new(data_dir, listener)`, `set_attachments`, `set_persistent_terminals`, `follow()/unfollow()` (folder watch -> wake), `report()`, `end(id)`, `forget(id)` |
| `TrustDesk` | core `project` trust: one project's lines, standing against the profile's `trusted-projects.json`, approval | `new(root, profile, data_dir, listener)`, `review()`, `approve_reviewed()`, `revoke()`, `follow()/unfollow()` |
| `AccessDesk` | core `ipc::access` queue, grants and scopes for one profile | `new(profile, data_dir, listener)`, `next_question()`, `answer(id, AccessAnswer)`, `withdraw(id)`, `held()`, `revoke(id)`, `waiting()`; Rust-side `ask(resolved, need, about, waiter)` for the control socket's gate |
| `FirstRun` | core `onboarding` + `agents::integrations` + settings | `new(agent_home, data_dir)`, `places(home, recents)`, `scan_agents()`, `install_hooks(id)`, `write_look(LookRecord)`, `should_show(saved_windows)`, `finish()` |
| free functions | core `import`, `update::licenses` | `import_scan(home)`, `import_apply(source, home, ticked, data_dir)`, `licenses() -> LicensesRecord` |

**Threading.** `UpdateEngine.check` and `install` return at once and run on threads of their own (`wormhole-ffi-update-*`);
progress wakes at most every 100 ms; a change of channel bumps a generation so a check that began before it does not
speak, and calls an on-its-way build of the channel left off (`update::on_channel_switch`). `HostsDesk.report` gives each
host 400 ms to answer, all at once. `AccessDesk.answer` settles under one mutex and tells every request that waited
(`Decision::Retry` or a refusal that says not to retry).

**Rules that are not obvious.**
- Nothing here quits the app: `UpdateEngine.restart` starts core's relaunch helper (it waits for this process to exit) and
  the caller terminates right after. The Swift side never calls it in tests.
- `TrustDesk.approve_reviewed` trusts exactly the request the last `review()` returned (a file that changed since reads
  `Changed` again). It asks nothing: Touch ID is the platform's (`Authenticating`), asked first by the Swift side.
- The `Preview` channel is core's `UpdateChannel::Unstable` (its rename is pending; the feed path is still `unstable/`).
- Link counters of a host (`LinkStats`) are the GPUI app's own terminals': a card of an attached host says "no connection
  counters" until the Swift workspace holds hosted terminals.
- Not here: the control socket's gate that raises questions (only its Rust-side `ask`), the access modes and "Always" list in
  Settings, the anonymous update count (off), watching the running bundle being replaced from outside.

## Frame

`frame()` returns `Data` of little-endian `u32` words, no padding. UniFFI copies the Rust `Vec<u8>`
into a Swift `Data` (one allocation and one memcpy per frame; the buffer is 16-byte aligned, so
`withUnsafeBytes` + `assumingMemoryBound(to: UInt32.self)` is fine at any word offset).

```
offset  size  field
0       4     magic: 0x31464857 ("WHF1" as bytes)
4       4     format version (2)
8       4     cols
12      4     rows
16      4     changed: number of row records that follow
20      4     flags: bit 0 = full (every row is in this frame: the first, after a resize,
                after a scroll, after the engine cleared history); bit 1 = a placements section
                follows the rows (see "Inline images" below)
24      4     cursor row (0xFFFFFFFF = no cursor: hidden, or scrolled out of view)
28      4     cursor column
32      4     cursor style: bits 0-7 shape (0 block, 1 underline, 2 beam, 3 hollow block);
                bit 8 = over a wide character; bit 9 = the program asked for blinking
36      4     display offset: lines the view is scrolled up into history (0 = live screen)
40      4     history size: lines of history above the live screen
44      4     reserved (0); with flag bit 1, the number of placement records
48      ...   `changed` row records, ascending by row, then (flag bit 1) the placement records
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
kitty's Unicode-placeholder pictures (they need the cells' underline colour and combining marks), command-block spans, selection ranges, the program's OSC palette changes.
Each arrives as an additive change to this file (a new header flag or a second call), with the
version bumped only if an existing field changes meaning.

## The object: `SettingsHandle` (settings)

One profile's settings as the Settings window holds them, over core's `Config` (`config.json`: merged,
sanitised, written in place with comments kept, never over a file that does not read). Keys are
`config.json`'s (a path for one inside an object: `motion.insert`), values are JSON text; no Swift type
mirrors `Settings`. The schema the rows are drawn from is a free function. Code: `apple/ffi/src/settings/`.
Errors are `SettingsError` (`notIsolated`, `invalid(reason)`, `profile(reason)`, `agents(reason)`; `reason`
is core's sentence).

```swift
static func open(profile: String?, listener: SettingsListener) throws -> SettingsHandle   // nil: the profile Wormhole is in (profiles.json); isolated only; may create config.json
func values() -> String            // JSON object: every schema key -> effective value (project overrides on top)
func defaults() -> String          // the same keys at their defaults
func set(key: String, value: String) throws          // core refuses unknown keys and values a setting does not read
func setMany(entries: String) throws                 // JSON object, one save, all or nothing
func reset(keys: [String]);  func resetAll() -> UInt32
func changedKeys() -> [String];  func changedRows() -> [SettingsChangedRow]   // person's own; Changed page rows
func setProjectOverrides(overrides: String?) throws; func overriddenEntries() -> [String]   // "set by project"
func search(query: String) -> [SettingsHit]          // core's ranking; spans are UTF-8 byte offsets
func revision() -> UInt64;  func drain() -> UInt64   // compare; drain re-arms wake()
func health() -> SettingsHealth                      // error sentence, file unreadable
func shellOptions() / iconThemeOptions() -> [SettingsOptionRow];  func paths() -> SettingsPaths
func settingsSchemaJson() -> String                  // the schema (format 1), as `Resources/settings.json`
```

`SettingsListener.wake()` is called from a Rust thread with no lock held, at most once between two
`drain()` calls, only when the file changed **outside this handle** (a hand edit, a script, another window):
the OS watch on the profile folder (settle 40 ms, FSEvents) with a 10 s timer as backstop (2 s when the
watch could not start). Our own writes do not wake; the caller pulls `values()` after `set`/`reset`. The
state lock is held for a `stat` or one atomic write and never across `wake()`.

Free functions (no handle): `checkShortcut(configKey:spec:) -> ShortcutVerdict` (core's recorder rules and
conflicts), `settingsAgentRows()` / `settingsSetAgentHooks(agent:install:)` (hooks write **only** under
`WORMHOLE_AGENT_HOME`, never the real home; reading uses it or the home), `settingsImportRows(home:)` (read-only
scan; `nil` is the real home), `settingsProfiles()`, `settingsSwitchProfile`, `settingsCreateProfile`,
`settingsDeleteProfile` (`profiles.json`; isolated only).

## Inline images and the terminal's text (`inline_images/`, `contents.rs`)

Additive to frame layout version 1 (the version stays 1: a reader that does not know bit 1 of the flags reads
the rows and stops; the Swift reader ignores unknown flag bits and trailing bytes). The engine decodes kitty,
iTerm2 and Sixel pictures and owns where they are; Swift draws them as textures.

**Placements.** When the header's flag bit 1 is set, `reserved` holds a count and that many 64-byte records follow
the rows, 16 words each: `key`, `z` (i32), `line` (i32: grid line, 0 the live screen's top row, negative history;
add the header's display offset for the viewport row), `column`, `cols`, `rows` (cells, already cut to the screen's
width), source `x y width height` (pixels of the picture; zero size means all of it), offset `x y` (points from
the first cell's corner), extent kind (0 fill, 1 fit inside the cells keeping shape, 2 pixels) with its `width
height` in points (kind 2 only: a cell is as many points as `start`/`resize` said), one reserved word. The list is
those placements that touch the view, in the engine's order (lowest `z` first). A frame carries it when it differs
from what the last frame carried (a scroll that brings one into view, a reflow, a new picture, an erase) and always
in `full_frame()`; a frame without the flag says "as before". Pixel offsets and extents are in the same points as
the cell size, so the view multiplies them by its scale.

**Placeholders (kitty's `U=1`).** The cells that name a picture are not carried as records of their own: a run of
consecutive tiles on a row becomes one placement record (`rows` 1, `cols` the run, the source rectangle the
share of the picture the run covers, fill extent), appended after the engine's placements, so the view draws them
as it draws any placement and the picture is stretched over the placement's cells as the GPUI app does. The cell's
character (U+10EEEE) is never a glyph and never text.

**Pixels.** `Terminal.inline_image(key) -> InlineImage?` (`width`, `height`, straight RGBA, `width * height * 4`
bytes) returns the picture a record names, copied once; ask once per key and keep the texture. `nil` while the
engine is still decoding it (it wakes the view with an `Output` event when done: ask again on the next frame) and
for a picture its store has let go of (the engine's budget: at most 1,024 images, 128 MiB per terminal, 512 MiB
for all terminals, the least recently drawn going first). A key is never reused for other pixels. A frame that
carries a list touches the pictures it names in the store, so what is on screen is the last to be evicted.

**Text.** `Terminal.contents(scope) -> String` with `ContentsScope.screen` (the live screen, one line per row, as
`screen_text`) or `.everything` (history and screen, soft-wrapped rows joined, a picture's placeholder cells as
blanks; the screen alone while a full-screen program has the alternate screen). Print and Export Text As use it.

## Cell extras (`frame_extras.rs`)

Additive to frame layout version 1, like the placements. When the header's flag bit 2 (value 4) is set, after the
rows and the placements: a count `n`, then `n` records of little-endian words: `row`, `column`, underline colour (the
frame's packed colour; `0xFFFFFFFF` when the text's own), the number of characters `m`, then `m` scalar values. They
are what a cell carries beyond its character: the zero-width characters that join into one picture (an accent, a
variation selector, a family emoji's joiners, a flag's second letter, a keycap, a skin tone, an Indic vowel sign) and
SGR 58's underline colour. The view shapes the character with its marks as one cluster in its own cell. A changed row
with no record has none, so the reader forgets what it kept for every row a frame names. A placeholder cell's marks
(they name a tile) are not sent. Hyperlinks stay `link_at`'s.

## What the Rust side does not do for you

Cell size and font metrics, glyph shaping, the palette, blink timing, the scroll bar, selection
drawing and mouse reporting are Swift's (or later seam calls). The seam gives the engine's grid
as it is; the engine does wrapping, reflow on `resize`, scrollback and all escape-sequence meaning.

## Git: `Git`, `GitWatch`, `WorktreeService`

`apple/ffi/src/git/`. The right panel's Changes, History and diff, and the worktree per terminal, over core's
`git/` domain (every command through `GitCmd`; nothing here spawns git). Swift side: `GitBridge`
(`GitBridge.makeBackends(worktreeAllowed:...)` returns a `RealGitBackend` and a `RealWorktreeBackend`).

**Threading.** Every call is blocking (a few git processes, each with core's timeout and output cap): call from
a background queue (`GitQueue` does). The objects are `Send + Sync`, hold no repository open and keep two small
lists (the last status per repository and compared revision, the files of the commits opened) so that a diff can
be asked for by path alone. **Errors:** `GitBackendError` = `NotIsolated` / `NotARepository` / `Failed(message)`,
the message in git's own words (`log_failure` adds "the branch was deleted"). Constructors refuse outside an
isolated data folder.

```swift
// Git (read-only)
func isRepository(folder) -> Bool                  // a look for .git, no process
func status(folder, compare: String?) -> GitStatus // files + branch, dirty, upstream, ahead/behind, compareMissing
func diff(folder, path, compare: String?, loadAnyway: Bool) -> GitDiff
func log(folder, rev: GitRev?, paths: [String]) -> GitLog          // nil = HEAD; .stashes = the stashes; 300 max
func commitDetails(folder, sha) -> GitCommitDetails                // the block as core finished it, files vs first parent
func compactMessage(folder, sha, lineChars) -> GitCompact?         // the compact message for a width; nil if not opened
func commitDiff(folder, sha, path, loadAnyway) -> GitDiff
func revs(folder, forHistory: Bool) -> [GitRev]    // branches (current first), 200 newest tags, 10 commits, stashes
func setBinaryUnits(binary: Bool)                  // `size_units`

// GitWatch: one thread per watch; listener.changed() once per debounced burst (150 ms quiet, 1 s cap, 1 s gap)
GitWatch.start(folder, listener: GitListener) throws -> GitWatch;  isLive();  stop()   // drop also stops
// a slow poll calls changed() when the OS watch is quiet 30 s (3 s while it could not start; retried every 60 s)

// WorktreeService (the one write; ONLY on these calls, never forced, hooks off)
WorktreeService(base: String?, cloneBuildFolders: Bool) throws;  setOptions(base, cloneBuildFolders)
func problem(folder) -> String?      func plan(branch, folder) -> GitWorktreePlan   // .ready / .refused(reason)
func create(branch, folder) throws -> GitWorktree      // plan, add, clone build folders, ports, remembered
func madeAt(folder) -> GitWorktree?   func list() -> [GitWorktree]   func isDirty(folder) throws -> Bool
func remove(folder) throws                              // only one Wormhole made; git refuses with changes
```

- **Diff = core's parsed hunks.** Core parses once (caps, binary, LFS, too-large rules); `GitDiffFile` carries
  `oldPath` (nil: a new file), `newPath` (nil: deleted) and `hunks` of `GitHunk{header, lines: [GitDiffLine{kind,
  old, new, text, noNewline}]}`, so Swift parses no diff text. Whole files are not read for highlighting (Swift's
  lexer colours the lines). `GitDiff` cases: `text(file)`, `binary(file, sizes)`, `lfs(summary)`, `tooLarge(bytes,
  title, detail)` (ask again with `loadAnyway`), `submodule(label, note, ahead, behind)`: the last three have no
  lines, and the words are core's (`diff_text`, `Submodule::label`).
- **An opened commit is finished by core.** `commitDetails` returns `GitCommitDetails`: the subject, VoiceOver's
  words, the raw `message` and `authorLine` (what the copies put on the clipboard), `people` (author, committer when
  it differs, co-authors) as `GitFact{label, spans: [GitSpan{text, tone}]}`, parents and refs capped with their
  "+N" lines, the body's prose and trailers, the "goes on" / "too long" lines, the `changes` fact and the files. It
  mirrors the GPUI app's `Shown`. `compactMessage(folder, sha, lineChars) -> GitCompact{text, more}?` is core's
  estimate of the three-line view for a line width (nil for a commit not opened), cheap enough to ask while drawing.
- `Terminal.grabPrompt() -> String?`: what is typed in an AI CLI's input box on the screen (`prompt_in`), for
  "Stash prompt"; nil at a plain shell prompt.
- **Compare** takes any branch, tag or commit; the base is the merge-base, as `status` reports it, and `diff` uses
  the same one. A deleted compared branch falls back to HEAD and says `compareMissing`.
- **Worktrees** are remembered under the store's `worktrees` key (core's `Worktrees`, the default profile's
  database). Trust of the *project* is not git's: `create` does not check it, the Swift `worktreeAllowed` closure
  does, before `problem` and again before the write. After `create`, the window opens the terminal
  (`worktreeCreated` gets the folder and `GitWorktree.env`: `WORMHOLE_PORT`, `WORMHOLE_PORTS`).
- Core addition: `git::tags` (newest 200, for "Compare with").

Tests: `apple/ffi/tests/git.rs` (real repositories in a temp dir: renames, binaries, LFS pointers, submodule,
stashes, the over-cap diff, the watch, the worktree round trip) and `Tests/GitBridgeTests` (mapping over canned
records, and the Swift backends against the real FFI).
## Platform seams: keymap and menus, notifications, folder picker, palette search, words

`apple/ffi/src/{keymap,notify,folder_picker}/`, `palette_search.rs`, `words.rs`. Each is core's rules called, never
re-implemented, with plain records on the Swift side (`...Record`). Additive: no contract version change. The Swift
adapters are `PlatformBridge` and `WordsBridge` (linked builds only); the protocols they fill (`MenuProviding`,
`NoticeDeciding`, `FolderProviding`/`FolderDecorating`, `PaletteMatching`, `WordSource`) live in `PlatformModel`,
`PaletteModel` and `WordsModel`, which keep their dumped or mocked stand-ins in a build without the FFI.

**Keymap and menus** (free functions, pure, any thread). Actions are named by text (`newTerminal`, `terminal.3`).
`keymapEntries() -> [KeymapEntryRecord]` (the table, in order), `keymapSystemKeys()`, `keymapMenus(state:) ->
[MenuRecord]` and `keymapMenusDump(state:)` (core's text dump), `keymapHiddenActions(featuresOff:flagsOff:)`;
`keySpecFromGlyphs/Parse/Glyphs/Text/SameKey`, `keyUses`, `recordShortcut(mods:key:)`, `shortcutConflict(key:own:)`
and the routes `keyActionFor(key:scope:)`, `keyTerminalYields`, `keyTerminalAction`, `keyListAction`,
`keyStudioAction`. `KeySpecRecord.mods` is a mask: control 1, option 2, shift 4, command 8. A menu item is one flat
`MenuItemRecord` (`kind`, `command`, `title`, `key`, `checked`, `enabled`, `items` for a submenu). A `MenuStateRecord`
action name the table does not know is dropped. The window's own facts (which toggles are off, the terminals, the
profiles) are still the app's to say: core has no model of them.

**Notifications** (`Notifier` object, one per app, internal mutex, no thread of its own). Sans-IO: nothing reads a
clock or posts a banner. `observe(tabs:settings:appActive:timeSensitive:nowMs:wallMs:) -> [NotifyActionRecord]` takes
one `TabFactsRecord` per terminal of every window (`sinceMs`, a failure's `atMs` on the monotonic clock;
`foreground.startedUnixMs` on the wall clock) and returns, in order, `post(NoticeRecord)`, `retract(id)`, `bounce`
and `say(text, urgent)`. The rules remembered across calls: each tab's last look (a tab seen for the first time counts
as idle), the burst (3 banners in 10 s become one summary and the singles are retracted; needs input is never
folded), the bell (once a minute per terminal), a reporting program's asks (once in 5 s). A tab may carry its own
profile's `NotifySettingsRecord`. `respond(response:) -> ResponseOutcomeRecord` turns a click into `open`,
`showChanges`, `reply(tab, text)` or `activate` and names the banner to take down; `dismiss(tab:)`, `testNotice(...)`,
`finishedCommand(tab:)`; free functions `notifyDockBadge`, `notifyDefaultSettings`, `notifyLevel`. Call `observe`
after an event that changed a terminal (a hook, a mark, a bell, focus), never on a timer.

**Folder picker** (`FolderPicker` object). `providerCount()`/`decoratorCount()`/`decoratorKinds(index:)`;
`ask(index:query:listener:) -> PickerJob` and `decorate(index:keys:listener:) -> PickerJob` start a job on a Rust
thread under core's gate (4 readers, 8 threads with the stuck ones, a job over 1.5 s stops counting); the
`PickerListener` hears `answered` or `decorated` **exactly once per job**, from that thread with no lock held, also
when the job was cancelled (`PickerJob.cancel()`: the answer is then empty or partial and the caller drops it).
Marks are byte ranges of the label (`ByteRangeRecord`). `recordVisit(path:nowSecs:)`, `forget(path:)`,
`recentsJson()` (the host stores it; `FolderPicker(recentsJson:recentTitle:)` reads it back). Every disk look goes
through core's `looks`, so a hung mount is skipped.

**Palette search** (free functions, pure). `paletteRank(query:candidates:) -> [PaletteRankedRecord]` (best first,
ties in order; ranges are byte ranges of the title), `paletteFuzzy`, `paletteParse`, `paletteScopes`,
`paletteStepScope`, `paletteGroupMax`, `paletteGroupOrder`.

**Words** (free functions, pure). `textIds()` (every message id, in `Key` order), `textTable(tag:) -> [String]?` (the
same length: each message's text, English's where a language lacks it, `""` for a message with variables, which
Rust formats), `textLanguages()`, `textNegotiate(requested:)`, `textSetLanguage(tag:)` (the words Rust itself writes),
`textHash()` (FNV-1a of the id list, for a generated key list to check itself against). One fetch per language,
then a lookup in Swift.

## The hook socket and the control line: `HookServer`

What the GPUI app's `app/control/` and hook handling do, over a `Workspace`, behind one object. Without it no
coding agent reaches the app: the installed hook scripts and the `Wormhole`/`wh` command line talk to a socket, and
the socket's server is this.

**Start.** `HookServer(workspace:usage:access:listener:dataDir:) throws` takes the data folder's single-instance lock
and serves its socket as core names and protects it (`ipc::acquire_in`): the path from the data folder
(`<temp>/wormhole-<uid>-<hash of the folder>.sock`, `socketPath()`; `dataDir` `nil` is the process's own, which the
terminals' `WORMHOLE_SOCKET` names too), a `0600` file, a peer check on the user's uid, and the kernel lock a second
instance cannot take. Errors (`HookError`): `NotIsolated` (a folder not named `Wormhole-swift*`), `AlreadyServed`
(another process holds the lock; it was asked to come forward, so this one is the second launch and should quit),
`failed`. `stop()` removes the socket, gives up the lock and closes the streams.

**What it does with what arrives.**
- An agent's hook (`WORMHOLE_BIN hook …`) is a status event for its terminal (`WORMHOLE_TAB_ID` = the `Opened` id);
  its meaning is core's `StatusMachine`. Plan usage reports (`Message::Usage`) go to `UsageService`. A status
  line's `Gauge` (model, context window) is dropped: a row has no field for it yet.
- A control request is answered with core's parsing, limits, budgets and reply formats, after the gate (below). Verbs
  served: `list`, `status`, `doctor`, `notify`, `send-text`, `send-keys`, `mouse`, `read`, `blocks`, `wait`, `run`,
  `events` (topics `status title folder bell blocks terminals splits output`; `windows` is never published: one window), `new-terminal`,
  `split` (right and down), `focus`, `rename`, `close`. Not served, each answered `unsupported` with words that say
  so: `new-window` (one window), `new-terminal --worktree`, `split` to the left or up, `welcome`. `split --size` is
  accepted and ignored (the reply carries no `size`).
- **The gate** is `agent_access/gate.rs` over core's `decide`/`resolve`/`Scopes`, with `AccessDesk` holding the
  approvals and the questions: Automation flag off, `control_write` decides as in the release; mode `off` refuses what
  needs approval; `open` lets it in; `ask` identifies the program behind the `wh` from the system (process chain,
  signature), asks the person through `AccessDesk.nextQuestion` (a request that said it waits holds its
  connection, one that cannot is refused with `approval_needed` and the question stays) and then asks about each
  terminal the program did not open. What a request opens is the program's own. Not ported: the question that
  asks before a mode loosened outside Wormhole takes effect (`change.rs`), and the "who is driving" chip.
- Every terminal started with `Terminal.startForTab(tab:…)` gets `WORMHOLE_TAB_ID`, `WORMHOLE_SOCKET`,
  `WORMHOLE_BIN` (core's `terminal::process`). `WORMHOLE_BIN` is the app's own executable, so its `main` must run
  `cliExitCode(arguments:) -> Int32?` first (`hook`, `statusline`, the control verbs; `nil` for a launch).

**Threads.** The socket thread (core) parses and writes; one dispatch thread (`wormhole-hook-dispatch`) answers in
order, never waits for a person (a question goes to the desk) and never panics out (a failed handler answers
`failed`). `read`/`blocks` run on the socket thread, `wait`/`run`/`events` on threads of core's. A thread of the
server's own (`wormhole-hook-tap`) compares what `list` would say now with what it said last when the model moved and
somebody listens, and publishes the status, title, folder, terminal and split events; output and bells come from the
terminals' own events through the workspace's tap. With nobody subscribed that is one atomic load. No lock of the
model is held while the bus or Swift is called.

**What Swift does.** `HookListener.wake()` (a Rust thread, no lock held, once until drained) then
`drainEvents() -> [HookEvent]`: `activate` (a second launch, `focus --activate`), `notice(terminal, title, body)` (show
it by the notification rules, `wh notify`), `closeAsked(terminal)` (ask the person; call `closeQuestionOver()` when
answered: only one is asked at a time; nothing closes unless they say yes). A `new-terminal` or `split` is not a
`HookEvent`: it is the workspace's own `Opened` in `Workspace.drain()`, started like any terminal the person opens
(`Terminal.startForTab`, `register_terminal`); a `--folder` replaces the model's folder in that event. A command the
request asked for is typed once the shell shows a prompt. Revoke an approval with `HookServer.revokeApproval(id:)`
(it ends the streams the approval let in), and call `settingsChanged()` after a change of the agent-access mode.
Swift side: `HookBridge` (`HookBridge.start(workspace:usage:access:)` and `HookBridge.exitCodeIfCommandLine()`).

**Core changes this needed** (additive): `ipc::acquire_in` and `ipc::socket_path_in` are public (a server on a folder
the process was not started with, and tests); `WorkspaceModel::splits_overview()`.

## Scrollback: find, command blocks, minimap (more `Terminal` methods)

Additive methods on `Terminal` (no version bump); code in `ffi/src/{find,blocks,minimap}.rs`. Swift sees them
through `FrameSource.scrollback` (`TerminalFinding`, `TerminalBlocks`, `TerminalMinimap` in TerminalCore).
Lines are grid lines (0 the top of the live screen, history above it negative); rows are viewport rows of the
last `frame()`; read the frame first.

```swift
// Find (wormhole_term::find). Matches stay in Rust, in grid coordinates, as of the search.
func findSearch(text:, matchCase:, wholeWord:, regex:) -> FindOutcome  // BLOCKING: copies the grid's text under the
                                          // engine's lock, searches the copy; call off the main thread. count, capped
                                          // (10,000), invalid (bad regex), nearest (the match to start from)
func findSetCurrent(index: UInt32?)      // the match painted apart
func findReveal(index: UInt32)           // sets it current and scrolls it into view (the next frame is complete)
func findSpans() -> [FoundSpan]          // matches on the viewport rows of the last frame {row, first, last, current};
                                          // shifted by the history growth since the search, dropped when the cells changed
func findTicks() -> [Double]             // 0 top of the scrollback ... 1 bottom of the screen
func findHasNewOutput() -> Bool          // the program printed since the search
func findClear()                         // forgets results and drops a search still running
```
A search that finishes after a newer one began reports its outcome but does not replace the results.

```swift
// Command blocks (OSC 133): wormhole_term blocks/marks/command_to_prompt.
func blocksAvailable() -> Bool;  func shellAtPrompt() -> Bool
func blockSpans() -> [BlockSpanRecord]   // viewport rows per block of the last frame {id, firstRow, endRow, starts, status}
func blockById(id:), blockAtLine(line:), blockNewestRan() -> BlockRecord?   // {id, start, outputStart, end, command, status}
func promptLine(from:, up:) -> Int32?;  func displayOffset() -> UInt32;  func bottommostLine() -> Int32
func scrollLineToTop(line:);  func selectLines(first:, last:);  func linesText(first:, last:) -> String
func blockPutAtPrompt(id:, palette: [UInt32]) -> PutResult   // ^U + the command as a paste, never a Return; refuses
                                          // concealed / same-colour / invisible text and multi-line without bracketed paste.
                                          // `palette`: the theme's colours by slot (0xRRGGBB, 263 entries) for the colour check
// BlockStatus: prompt | running | finished(code: Int32?, tookMs: UInt64?)
```

```swift
// Minimap (wormhole_term::minimap): lines are read under the lock only when the output or the view changed.
func minimapGeometry(fits:) -> MinimapGeometryRecord   // {total, screen, fits, viewTop, columns, stamp}; cheap
func minimapLines() -> [[MinimapRunRecord]]            // call again only when `stamp` moved; colours/flags as the frame's
func scrollToDisplayOffset(offset:)                    // the view `offset` lines into the history (0: live)
```
Threads: every call is a short lock except `findSearch`. Hostile input: the pattern is compiled with core's size
limit, matches are capped, a command is typed only when every cell shows what it holds.
## Persistent terminals: `HostedTerminals` and a `Terminal`'s host

Terminals that outlive the app, as the GPUI app's "Persistent terminals" flag does (`repo-docs/docs/session-host.md`).
Core is called, never re-done: the host program's place (`ipc::host_binary`), the hosts folder's scan and the launch
plan (`ipc::hosts`: `forget_gone_hosts`, `scan`, `LaunchHosts`, `adoptable`), what a failed attach means
(`AttachFailure`), the PTY's hosted constructors (`terminal::{spawn_hosted, attach_hosted}`) and `tabs.host_id` of the
profile's database (`Store::save_session_host`). Additive: no existing signature changed (`Terminal` gained methods).

| Object / record | What it is |
|---|---|
| `HostedTerminals` | `new(dataDir?, hostBinary?, window)`: refuses outside a `Wormhole-swift*` folder. `enabled()` (core's flag rules over the folder's `config.json`), `program()` (`HostingProgram`: `found(path)`, `disposableCopy`, `missing`), `planAtLaunch()`, `adoptableOrphans()`, `start(what, listener)`, `attach(hostId, what, visible, listener)`, `finishRestore()`, `finish(endHosts)`. |
| `HostedStart` | `tab`, `program`, `args`, `cwd?`, `cols`, `rows`, `cellWidth`, `cellHeight`: the same facts `Terminal.startForTab` takes. |
| `LaunchPlan` | `saved` (the last run's terminals in the sidebar's order, each `SavedTerminal { hostId, running, cwd?, shell? }`: `running` attach, otherwise its host is gone and a new terminal starts in the folder; their text comes in `text`, for `HostedStart.savedText`), `orphans` (live hosts no saved terminal of any profile names). |
| `Terminal` (new methods) | `hostId() -> String?`, `hosting() -> TerminalHosting` (`local`, `persistent`, `detached`, `lost`), `detach() -> Bool`, `setVisible(Bool)`. Everything else (frames, keys, resize, scroll, find, interaction) is unchanged: a hosted terminal's engine is a replica fed from the host. |
| `HostingError` | `notIsolated`, `noProgram` (no host program: stay in the app), `start` (host or program did not start: the caller may start locally and say so once), `gone` (attach: the host is not there: start a new terminal), `unavailable(reason, kind, transient)` (it runs and cannot be shown: keep the terminal's place and say why; `transient` = try again), `store`. |

**The host program.** Core looks beside the app's own executable and nowhere else (a disk image or translocated copy is
`disposableCopy`: no host there). `hostBinary` names another executable (tests, development); it is an argument and not
`WORMHOLE_HOST_BIN` because the release library ignores that variable. The Swift app passes `WORMHOLE_HOST_BIN` through
as that argument. A host signed by `cargo build` takes every peer of the same user (core's peer policy).

**Saved rows.** Each started or attached terminal is saved at once under `window` in the default profile's database
(`tabs.host_id`, the column the Session Hosts window and the launch plan both read, so a named host is never an orphan).
Closing an attached terminal ends its program and forgets its row; `detach()` (or `finish(false)`) leaves both. The
workspace gives new ids every run, so after the restore `finishRestore()` forgets the rows the last run left under old ids.
A window saver that comes later writes the same `SessionRecord`s with `host_id` from `Terminal.hostId`.

**The flow.** Launch: `enabled()` and `program()`; if both allow, `planAtLaunch()` (once; fast), then per `saved` entry
`Workspace.apply(newTerminal)` and, on the `Opened` id, `attach(...)` for a `running` one (`gone` becomes a new terminal,
`unavailable` shows the reason and keeps the row) or a new hosted terminal in its folder, and, off the main thread,
`adoptableOrphans()` (up to half a second per silent host) attached the same way (the GPUI app puts them in a Recovered
group). Then `finishRestore()`. New terminals: `start(...)`, falling back to `Terminal.startForTab` on `start`/`noProgram`.
Quit: `finish(endHosts)`, with `endHosts` from the quit question's answer (`QuitFacts.endsHosts`): true closes every terminal
(each host ends, no row names one), false lets go of each (the hosts and their programs run on). Call it from
`applicationShouldTerminate`'s decision and `applicationWillTerminate`.

**Threads.** `start`, `attach`, `adoptableOrphans`, `endHost` and `saveTexts` block (a host starts in about 15 ms; a stuck one
takes seconds). The Swift backend calls them from a detached task with the pane blank (`HostConnect`), as the GPUI app does:
the terminal's view gets a `UniFFIFrameSource(waiting:)` that draws nothing until `fill(terminal)`, and a start or attach that
finishes after its terminal was closed, started again or the app quit is closed or let go of. The host's events reach Swift
through the terminal's own `TerminalListener`, as a local terminal's do.

### Round P additions (all additive)

- **Restoring what a gone host showed.** `SavedTerminal` gained `forgetText` and `text` (the saved screen, only for a terminal whose
  host is gone); `HostedStart` gained `oldHost` and `savedText`, which only `start` reads. `start` draws, above the new shell, the
  checkpoint of `oldHost` when it reads (after a restart of the Mac), else `savedText` (core's `terminal::restored`, shared with the
  GPUI app), then a divider that says when. `saveTexts()` keeps the screens of the terminals the desk made (the app calls it each
  minute and on a quit that leaves hosts running); it needs the profile's "restore terminal text" and core's sealed store.
- **Restart restore** (`FeatureFlag::RestoreAfterRestart`, with persistent terminals and "restore terminal text"): `start` and `attach`
  ask the host to keep checkpoints, sealed with the key core derives from the host id (`hosting/checkpoints.rs`); a terminal restored
  from a checkpoint has its new host write its own at once, and the old one is removed when the new one reads back. A closed terminal
  removes its checkpoint (now and 5 s later); `planAtLaunch` sweeps old ones in the background (a restore under way is spared).
- **Host problems.** `HostingError.Unavailable` gained `kind: HostUnavailableKind` (core's `UnavailableKind`); `Terminal.hostNotResponding()`
  (seconds, or nil) with `TermEvent.HostResponsiveness` as the cue to ask again; `HostedTerminals.endHost(hostId, hard)` (nil, or why it
  failed); `attach` for a host not in the plan answers as core classifies it (gone, or unavailable).
- **Workspace.** `Action.RecoverGroup { members }` puts adopted terminals in core's Recovered group (`TermGroup.recovered`);
  `Workspace.shown()` (the terminals on screen: a host feeds those and leaves the others headless) and `cwdOf(id)`.
  `HostedTerminals.adopt(tab, terminal)` takes the tab id now.
- **The Keychain.** The crate builds core with its `dev-key` feature: the saved text's key lives in a file beside the text in the
  isolated data folder, never in the login Keychain (this crate runs only in `Wormhole-swift*` folders).

## Platform and settings gaps (round N)

Additive to the hook socket and settings sections above; nothing existing changed meaning (`HookEvent` lost `Eq`,
since `gauge` carries a float).

- **`split` on all four sides.** `left`/`up` (and `right`/`down`) make the terminal standing alone, then
  `Action::JoinSplit` joins it to the target's pane on a drop zone's side, as the GPUI app's `control_split` does
  (`control/joined.rs`); no room refuses and leaves no trace for Swift. `--size` is still not applied.
- **`new-terminal --worktree <branch>`** answers `Asked` after core's `worktree::plan` and queues
  `HookEvent::WorktreeAsked(folder, branch)`: Swift puts the new-worktree sheet up with the branch filled in. Trust is
  the sheet's backend to decide (git knows none); a `--command` with it is refused. `new-window` stays `unsupported`
  until the window host exists.
- **`HookEvent::Gauge(terminal, agent, model, context)`** from the status line (`Message::Gauge`), once per change
  per terminal; Swift keeps the latest (`GaugeBoard`) and shows it while that agent is in the terminal.
- **Command line tool** (`cli_tool.rs`): `cliToolStatus(binaryPath)`, `cliToolInstall`, `cliToolUninstall`,
  `cliToolPathNote(folder)` (starts the login shell: off the main thread) and `cliToolKeepCurrent(binaryPath)`,
  which the app calls at launch before its first terminal (`<data dir>/bin/wh`, first on every terminal's PATH).
  Folders are core's `Targets::from_env`; the Swift app sets `WORMHOLE_CLI_DIR=<data dir>/cli-bin` unless given.
- **Settings:** `ProjectService.settingsOverrides(folder) -> ProjectSettingsRecord?` (`json` only while the
  project's Settings switch is on and it is trusted) feeds `SettingsHandle.setProjectOverrides`;
  `AccessDesk.approvals()` lists stored and held approvals, `revokeStored(id)` / `HookServer.revokeStoredApproval(id)`
  take one back; `resetTips()`, `dismissedTipsCount()`, `defaultTerminalScriptTypes()`, `defaultTerminalFailed(error)`,
  `profileDeleteQuestion(profile)`, `profileCopyOption(profile)`.
## Several windows: `Windows`, window records, terminals moving between windows

Additive, contract version stays 2. One `Workspace` per window, as in the GPUI app (`AppState` + one `Workspace` each);
what they share and what is saved of them is one object, `Windows` (`windows/`, over core's `Store`).

```swift
// the registry (fails with FfiError.NotIsolated unless WORMHOLE_DATA_DIR is the experiment's)
Windows()                                    // opens the profile's database; ids start above everything saved
func saved() -> [SavedWindow]                // to open at launch, the window that was in front last LAST: SavedWindow(id, frame: FrameRecord?, terminals)
func allocWindowId() -> Int64                // an id for a window that starts empty (first launch) or is given terminals
func plan(from: Workspace?, folder: String?) -> PlannedWindow   // window + first terminal ids, known before anything is on screen
func save(window: Int64, workspace: Workspace, frame: FrameRecord?) -> Bool   // cheap when nothing changed; keeps the Notes tab's rows
func saveFront(window: Int64)                // which window was in front, for the next launch
func forget(window: Int64)                   // the person closed it: it and what it held are gone (a quit saves, never forgets)
// a window's workspace
Workspace.inWindow(windows:, window:, config:, listener:)   // ids from the registry's counter: no two windows share one; the control line finds its terminals
func restore() -> [WorkspaceEvent]           // fill an EMPTY workspace from its saved or planned record: `opened(id, cwd)` for every terminal (start its program there)
func takeOut(ids: [Int64]) -> TakenOut       // TakenOut(departure: Departure?, events): terminals leave RUNNING; here they are closed as a close does (selection by the window's rules, a window left with none starts one)
func receive(departure: Departure) -> [WorkspaceEvent]   // they join the end of the list under their ids; `adopted(id)`; once per departure
// frames (core `view::layout::frame`: y down, primary display first, plain numbers)
fitFrame(frame:, displays:) -> FrameRecord   // a window still reachable stays; one whose display went, or that is off every display, is put on its own (else the primary) display
cascadeFrame(from:, display:) -> FrameRecord // a step (26 pt) down and right, or the display's corner when that would run off
```

- **What a record holds** (`WorkspaceModel::window_record`, core `workspace/saved.rs`): terminals in the window's order with
  id, name, pin, "hide from agents", "forget text", folder and split, the splits with their pane layouts, the sidebar's
  groups/folds/order, the columns and the panel's tab, the selected place; `frame` is the shell's own JSON
  (`FrameRecord` through core's `WindowFrame`, the GPUI app's own shape). The layout JSON is the GPUI app's (`SavedLayout`).
  Not kept: what ran in a terminal (a restored terminal is a fresh shell in its folder), notes (the Notes tab's own rows,
  untouched by `save`), the terminals' saved text.
- **Ids.** Terminal and split ids share one counter across all windows (`IdCounter`), window ids another; both start above
  anything in the database (`Store::next_ids`). A terminal keeps its id when it moves, so notes and agent hooks follow it.
- **Moving.** `takeOut` unregisters the `Terminal`s before the close is told (the model's `Closed` ends a registered program), so the
  caller must also let go of the old view's source WITHOUT closing it (`UniFFIFrameSource.detach()`), keep each terminal's
  `TerminalRelay` (the view's wake) and hand both to the other window; `receive` re-registers the terminals and feeds them to
  the new window's model. Dropping a `Departure` that nobody received ends the programs. Save the receiving window before the
  giving one (the store moves a session row to the window that saves it first). A moved terminal's status resets until its
  next hook, and it joins the end of the new list outside any group.
- **Events.** `WorkspaceEvent.adopted(id)`: a terminal arrived running, make a view, start nothing. `HookEvent.newWindow(window,
  terminal, folder, activate)`: see below.
- **The control line over windows.** `Workspace.inWindow` makes every window's terminals reachable by the server that was
  started over any one of them: hooks (`WORMHOLE_TAB_ID`), `read`, `send-text`, `send-keys`, `mouse`, `run`, `wait`, `notify`
  and the hidden-from-agents rule use the window that holds the terminal. `new-window [--folder] [--command] [--activate]` is
  served (it was `unsupported`): the server plans the window (`Windows.plan`, the window's panels without its groups, a
  folder from the request or the selected terminal's), answers `{window, terminal, folder}`, and queues `HookEvent.newWindow`;
  Swift makes a workspace with `inWindow(windows, window, …)`, calls `restore()` and starts the program; `command` is typed
  once the shell is ready (within 3 s the terminal must be in some window). Every verb reaches every window (see "Round R: window"): `list`, `status`, `events`, `focus`, `rename`, `close`, `split`, `new-terminal`.
- **With persistent terminals.** `HostedTerminals(window:)` is per window and saves its rows under that window's id;
  `Windows.save` writes each terminal's `host_id` from `Terminal.hostId` (a terminal not registered yet keeps the host its saved
  row names). `SavedTerminal.terminal` is the id the row was saved under: a window restored from its record opens that terminal under
  the same id and attaches to its own host (`HostedStart`/`attach`), else starts it again. The launch plan counts other windows'
  hosts as named, so no window takes another's for an orphan; one window should ask `adoptableOrphans`. `HostedTerminals.adopt(terminal:)`
  hands a terminal that moved to the desk of the window that holds it, which lets go of it at the quit. Save the windows before
  `finish(endHosts:)` (a save names hosts; End All forgets them).
- **Threads.** `Windows` takes its store's mutex per call (microseconds); `save` reads the workspace's model with `peek_model`
  (no tap wake). Everything is meant for the main thread; none calls Swift.
- **Core changes this needed** (additive): `workspace::{IdCounter, SavedLayout, Departing}`, `ModelConfig.ids`,
  `WorkspaceModel::{window_record, restore, take_out, receive}`, `Event::Adopted`, `view::layout::frame`.

## Font picker rules (`fonts.rs`)

Additive, free functions (no object, no listener, no thread of their own; each call is microseconds except the folder scan,
which reads font files: call it off the main thread). They carry the rules of `theme::fonts`; the installed families and
their traits stay CoreText's on the Swift side, as `fonts.rs` and `platform/macos/fonts.rs` of the GPUI app have them.

- `fontChoices(installed, bundled, custom, monospace?, query, monospaceOnly) -> [FontChoiceRecord{origin, name}]`:
  core's `grouped_font_choices` (hidden families out, every word of the query, prefix matches first, each family once,
  groups bundled, custom, installed). `monospace` is nil while the installed families are being classified: nothing is
  filtered then; it must include the bundled and custom monospace families.
- `fontCustomScan(dir) -> CustomFontsRecord{files, families, skipped}`: core's capped `custom::scan` of the fonts folder
  (`SettingsPaths.fontsDir`). A file that is not a font is only named in `skipped`; a missing folder is empty. The caller
  registers `files` with CoreText for the process (`CTFontManagerRegisterFontURLs`, scope process), as GPUI does.
- `fontBundledFamily() -> String`: `JetBrains Mono`. A custom family of that name replaces it (list it once, as custom).
- Swift: `SettingsBridge.RustFontRules` implements `FontPickerModel.FontRules`; `RustSettingsHost.fontRules` hands it out.
  `LocalFontRules` is the same rules in Swift for a build without the core, and a test keeps the two equal.
## Settings panels: skills, MCP, feature flags, Extensions, Project folders (round P)

Additive; nothing existing changed meaning. Code: `ffi/src/settings/{skills,flags}.rs`, `settings/probes/`,
`files_panel/project/settings_page.rs`.

- **Skills and MCP.** `settingsSkillRows() -> [SettingsSkillRow]` (the agents that load skills and are on the PATH, or
  have it already: `id` is the target's word, `state`, the catalogs' `detail`, `shown` the folder), 
  `settingsSetSkill(word:install:)` (writes only under `WORMHOLE_AGENT_HOME`, as the hooks do) and
  `settingsMcpCommand()` (the `claude mcp add …` line to copy; nothing edits another app's MCP settings).
- **Feature flags and the nav.** On `SettingsHandle`: `featureFlags() -> [SettingsFlagRow]` (key, title, description,
  `on`, `default`), `setFeatureFlag(key:on:)` (the default takes the entry out), `toggleFlagsUnlocked() -> Bool` (the
  unlisted ⌃⌥⌘F) and `visibleSections() -> [String]` (core's `SectionId::is_visible`: the Preview pages follow their
  flags and the build; section ids are the schema's).
- **Extensions: `ProbesDesk(authenticator:)`** (isolated only). `list()` (reads and tidies: off the main thread) ->
  `ProbeListRecord` (rows with `id`, `state` = `on|off|changed|unsupported|invalid`, the catalogs' words, `note`,
  `canApprove`, `canForget`; `keyProblem`; `tooMany`), `sheet(id:)` reads the Probe once and keeps that request,
  `needsTouchId(id:granted:)` (core's `Request::confirmation` for the permissions left on), `approve(id:granted:)`
  -> `ProbeApprovalOutcome` (`approved|cancelled|failed(message)`; it calls `OwnerAuthenticator` only when core says Touch
  ID, blocks while the owner answers, and records exactly the bytes the sheet showed) and `turnOff(id:)` (also forgets).
  Words a Probe's author wrote pass `reveal`. **`wormhole-core`'s `dev-key` feature is on for the FFI**: approvals (and
  the other sealed stores) keep their keys in files of the experiment's own data folder, never in the login Keychain;
  drop the feature in `ffi/Cargo.toml` when the Swift app ships.
- **Project folders** (more `ProjectService` methods, the same hub as the Project tab): `projectDefaults()`,
  `setProjectDefault(feature:on:)`, `clearFeatureChoice(root:feature:)` (with the existing `setFeature` for a choice of
  on/off), `trustedProjects()`, `knownProjects(folders:)` (the project of each open terminal's folder, then the trusted
  ones), `projectPage(root:) -> ProjectPageRecord?` (trust, its sentence, the commands while untrusted, a row per
  feature with `choice`/`defaultOn`, problems) and `missingFolderNote(root:)`; `approve`/`revoke` are the existing ones.
## A program's clipboard (OSC 52): more `Terminal` methods (`clipboard.rs`)

Core's rules, Swift's pasteboard. Core's `ReadGate` decides (one question at a time, only where it can be seen);
the object holds the engine's reply slot until every path out has answered it, with the text or with nothing.

- `set_clipboard_rules(read: ClipboardReadRule, write_allowed: bool)`: Settings' `clipboard_read` (`ask`, `allow`,
  `deny`) and `clipboard_write`. Ask and write-allowed until said.
- `set_on_screen(bool)`: the terminal is shown in the window in front. A read that would be asked about is refused while
  it is false (GPUI's `in_front`); false until Swift says.
- Events (`TermEvent`, added at the end): `ClipboardWrite { text }` (a program set the clipboard and the setting allows
  it: Swift writes the pasteboard), `ClipboardRead` (let through: `clipboard_give(text)` answers the oldest one),
  `ClipboardAsk` (the person decides: `clipboard_answer(choice, text)`, `text` read only for a yes) and
  `ClipboardAskClosed` (the question timed out after 30 s and counted as Deny).
- `foreground_program() -> Option<String>`: the program in front as the sidebar names it, cleaned and cut, who the
  question names; `None` when the shell holds the terminal.
- `select_lines(first, last)` (blocks) now also brings the first line into view, as GPUI's Select Output does.
## How long a terminal has waited (`workspace_waited.rs`)

`TerminalSummary.since` is when the terminal got into its state, in milliseconds of the workspace's clock
(`now_ms`). `Workspace.waited(sinceMs) -> Waited(words, nextInMs)` says how long ago that was as the core's status bar
does ("<1m", "4m", "1h 5m": `view::layout::status_bar::elapsed`) and in how many milliseconds the words next read
differently: arm one timer for it. Core change (additive): `TerminalSummary.since`.

## Settings pages: profile fields, import review, page sentences (round P)

Additive; `wormhole-ffi` `settings/`. Everything blocking: call off the main thread when a page reads files.

```
// Profile page (profile_edit.rs): the fields of a profile besides its list row, saved through core's ProfileList::edit.
record SettingsProfileForm { id, name, avatar, color /* GroupColor key */, startDir, folders: [String], env /* NAME=value lines */,
                             gitName, gitEmail, logins: [SettingsLoginField { agent, name, placeholder, dir }], startCaption }
func settingsProfileForm(id: String?) throws -> SettingsProfileForm   // nil: the profile Wormhole is in
func settingsProfileSave(form:) throws -> Bool                        // true: changed; core's sentence on refusal, profile unchanged

// Import page (import_review.rs): one terminal's proposal as rows, and taking the ticked ones.
protocol FontFamilyResolver { func family(postscriptName:) -> String? }    // Swift: CoreText; an unresolved font stays under "Doesn't carry over"
func settingsImportReview(home: String?, source: String, fonts:) -> SettingsImportReview?   // files, items {label, from, value, command?, defaultOn}, skipped {key, reason}
SettingsHandle.importApply(home:, source:, ticked: [UInt32], shown: UInt32, fonts:) throws -> SettingsImportDone {brought, summary}
   // re-reads the terminal's files; refuses when the proposal no longer has `shown` rows; palettes saved as theme files, then one settings update.
   // The write is core's own, so no wake follows: the caller reloads its values.

// Sentences that carry a number or a header (page_text.rs), in the catalogs' words and plurals.
func settingsUpdateUserAgent(version:, counting:, unstable:) -> String?    // core's counted_user_agent / plain_user_agent
func settingsOthersHead(count:), settingsImportSelectedLine(selected:, total:), settingsResetQuestion(count:), settingsPreviewNote(section:) -> String?
```

## Keyboard focus and Zen mode (`focus.rs`, round R)

Additive; pure functions over `wormhole_core::view::focus` and `config::presets`, no state, no threads. The shell holds the real focus and the
window's facts, asks, and acts on the answer (`WorkspaceModel.Focus.rules`, installed by `WorkspaceBridge.CoreWindowRules`).

```
enum FocusArea { TitleRow, Sidebar, Tabs, Terminal, PanelTabs, PanelContent, StatusBar }        // core's Area, F6's order
record FocusStop { area, index }      record AreaStops { area, count }                            // a ⇥ stop; how many an area has now
func focusStep(from: FocusArea?, delta: i32, shown: [FocusArea]) -> FocusArea                    // F6 / ⇧F6 skip what is not on screen
func focusTabStep(from: FocusStop?, delta: i32, stops: [AreaStops]) -> FocusStop                  // ⇥ / ⇧⇥, stop by stop
record EscapeState { terminal, popup, detail }   enum EscapeRung { ToShell, ClosePopup, CloseDetail, ToTerminal }
func focusEscape(state: EscapeState) -> EscapeRung                                                // the Esc ladder
record ListItem { level: u8, fold: bool? }   enum ListKey { Up, Down, Left, Right, First, Last }   enum ListMove { To{index}, Open{index}, Fold{index}, Stay }
func focusListWalk(items: [ListItem], at: u32?, key: ListKey) -> ListMove                        // arrows in a list
func focusTypeAhead(names: [String], at: u32?, typed: String) -> u32?                             // letters jump by name
func focusPressFoldsBand(space: bool) -> bool                                                     // Space folds a band, ↩ does not
enum WindowPreset { Zen, Balanced, Everything }   record PresetShown { titleRow: [String], statusBar, minimap, columnDividers, panelOpen }
func windowPreset(preset: WindowPreset) -> PresetShown                                            // Zen's layer is windowPreset(.zen)
```
## Round R: window (panes, title extras, the control line over windows)

Additive.

- **Panes are the core's.** `Workspace.panes(width, height) -> PanesSnapshot` now also has `unzoomed: [PaneRect]` (every pane at
  its place in the unzoomed layout, reading order: what a zoom hides and the header order). The Swift window keeps no pane tree: it
  sends `setPaneArea` (with the pane header's height in `WorkspaceConfig.paneHeaderHeight`, 34 as GPUI's) and draws the snapshot.
  New actions: `movePane(id, target, zone)` (a pane's header dropped on another pane's edge, inside its split; refused `unchanged`,
  `tooSmall`, `unknown`) and `focusDirection(direction)` (⌥⌘ arrows; refused `unchanged` at the area's edge). New query
  `Workspace.paneDropRefusal(dragged, target, zone) -> Refusal?` (pure: a member moves, any other terminal joins) for the drop hint.
  `Direction` is a core enum on the seam. Core: `Action::{MovePane, FocusDirection}`, `WorkspaceModel::pane_drop_refusal`.
- **Title extras.** `TitleSnapshot` has `ports: [UInt16]`, `pullRequest: PullRequest?` (`number, url, state, draft, checks, review`) and
  `pullRequestWords` (core's `PullRequest::words`). The `ports` and `pull_request` slots are `present` once found. Looked up by the
  workspace's pump (`workspace_extras.rs`), nothing in Swift: the ports from the shell's process tree for the selected terminal every
  5 s while the row has the part (no OS event exists for a listener); the pull request from `gh` per terminal branch (`git::pr`'s
  cache and deadlines), only while the `project_folders` flag is on and the repository's `.wormhole/` is approved in the data
  folder's default profile (`ProjectHub::trusted`'s rule). A change moves the chrome revision.
- **The control line over windows.** One publisher per window feeds the same bus (attached for every window that exists at
  `HookServer.start` and, through `Windows.on_register`, every one made later), so `events` hear every window. `list` and `status`
  cover all windows (`front` is the window the server started over); `focus`, `rename`, `split` and `close` act in the window that
  holds the terminal and answer with its id; `new-terminal --window <id>` (else the caller's own window, else the first) opens in
  that window. `HookEvent.front(terminal)` follows `focus --activate`: bring that terminal's window forward.

## Probes: decorations, fold markers, hook effects (round R)

The Probes that are on run in Rust, hosted by the hook server's core (`probes/`), and Swift draws what they say. Everything
that decides is core's or the terminal layer's, shared with the GPUI app: the rules and their spans
(`probe::runtime::{RuleSet, Decor}`), the hooks and their budgets (`probe::active::Dispatcher`), the bus feed
(`ipc::probe_feed`), the folds and their rules (`wormhole_term::folds`), the marker's words (`view::fold::marker_words`).
The `probes` flag, the `output_folds` flag and `never_hide_lines` are read with the settings: call
`HookServer.settingsChanged()` when any of them changes. The Settings window's approvals (`ProbesDesk.approve`, `turnOff`)
reload the Probes on their own; `HookServer.reloadProbes()` does it for a script or a test.

### Two more sections of the frame (additive to layout version 1)

After the cell extras, each when its header flag is set: bit 3 (value 8) decorations, bit 4 (value 16) fold markers. Like the
extras they are as complete as the rows the frame carries: a row the frame names and no record names has no mark. A change
of a row's marks that the engine did not damage (rules loaded, a hook tinted a block, the pointer revealed a span) makes the
frame a full one.

```
decorations: count, then per row with a mark
    row, the block's tone, number of spans m, then per span
        first column, last column, tone, k (0: the text stays; else the span's width), then k scalar values
fold markers: count, then per marker row
    row, fold id (low word, high word), lines hidden, flags (bit 0 open, bit 1 its block failed),
    then label, spoken, tooltip, each as a byte length (0xFFFFFFFF: none) and its UTF-8 bytes padded to a whole word
tone: 0 none, else 1 + the place in link, danger, warning, success, muted (`Tone::ALL`); the theme token is `Tone::token`
```

A span's tone paints the text, a block's tone tints the cell backgrounds (faint, `TONE_TINT` 0.16); a mask is what each cell shows
instead of its text (a shortened or redacted span: the real characters, and their zero-width marks, are never drawn). The
marker row's cells are blank: the host draws the marker over it (`label`, a chevron, danger when `failed`) as a real button
named by `spoken`, whose click or Return calls `Terminal.foldToggle`. The palette gets five tone slots (263 to 267).

### `Terminal`: folds, reveal, files (`probes/terminal.rs`)

```swift
func hasFolds() -> Bool
func foldToggle(id: UInt64) -> Bool        // the marker was clicked; whether the fold exists
func foldShowAll() -> UInt32               // Edit > Show Hidden Lines: the lines it showed (0: none was closed)
func foldShownWords(lines: UInt32) -> String   // "124 lines shown", for VoiceOver
func setReveal(row: UInt32?, col: UInt32?, alt: Bool) -> Bool   // pointer over a cell (nil: left); true: take a frame
func resolvePath(text: String) -> ResolvedFile   // `path[:line[:column]]` in the terminal's folder; off the main thread
```

`LinkHit` gains `file` (a rule named a file: `uri` is the path as written, not an address; `resolvePath` finds it) and
`readout` (what the chip says while Command is held: the rule's hover words, where it really goes, the Probe's name).
`link_at` asks the rules and, under folds, maps rows to grid lines through the last frame (`grid_point`).

What may be folded is never Swift's: the engine refuses the live screen, the live block, the alternate screen, a program
that has the mouse, a secret being typed and, while the terminal needs the person (an agent's question, a program's report),
the last two screens (they show open). The workspace tells each terminal's folds when its row needs you (`post`), a bell opens
the recent ones, and output while folds exist looks at echo.

### Hook effects: more `HookEvent` cases, one read

```swift
case probeNotice(probe: String, subtitle: String, title: String, body: String)   // subtitle: "<name> (extension)"
case probeOffer(terminal: Int64, words: String, text: String)   // a chip; a click calls resolvePath(text)
case probePaused(words: String)
case probesChanged        // rules, status item, tint or fold changed: take a frame of every terminal
func probeStatusItems() -> [ProbeStatusRecord]   // probe, item, text, token (design token or nil), tooltip
```

Nothing opens or runs by itself: an offer is a chip, a click opens the file, `run` is the dispatcher's under the `process` grant.
`front` in a hook's `match` is "the selected terminal of its window" here (the model knows no front window).

### The stale progress bar

A program's OSC 9;4 bar that stops being renewed goes after `PROGRESS_TIMEOUT` (15 s): the workspace arms one deadline in its
pump when a report appears (`Pump::progress_by`), the pump drops the bar when it falls due (or waits out what a renewal left) and
the row's state is told as every report is. No polling.

## Profiles: which one Wormhole is in (`profile.rs`, round R)

Additive, contract version stays 2. `profiles.json` (core's `ProfileList`, written by `settingsSwitchProfile`) says which profile
Wormhole is in; every object that keeps a profile's data reads it when it is made, so a switch moves them all and Swift
only hides and shows windows. A profile's folder is core's `DataRoot::profile` (Default's is the data folder itself).

```swift
activeProfile() throws -> ActiveProfile     // { id, dir }: the profile Wormhole is in and its folder (config.json, themes/, fonts/, wormhole.db)
Windows()                                    // now the ACTIVE profile's database; counters start past every profile's saved ids
func beside(profile: String) throws -> Windows   // another profile's windows open at once: own database and saved windows, SHARED id counters and live windows
```

- Made for the active profile (was: Default): `Windows()`, `SessionNotesStore()`, `UsageService`, `ProjectService`,
  `HostedTerminals(dataDir: nil)` (its registry and settings; the hosts folder stays the data folder's), `SettingsHandle.open(profile: nil)`.
  An explicit `dataDir` (tests) is still one profile of its own.
- **A switch** (the Swift app, `AppController.switchProfile`): save the windows leaving, `settingsSwitchProfile`, make the target's
  `Windows` with `beside` (once; kept), show its windows (restored from its database the first time, one new window when it has none)
  and hide the leaving ones (their terminals run on), make the settings model and theme folder again for the new profile.
- **Ids.** Terminal and window ids come from one counter for every profile open (`beside` shares it, `Windows()` starts past
  all profiles' databases), so an id is never taken twice and hooks and the control line find a terminal in any profile.
- Not done: the
  Access, Trust, Imports and Session Hosts desks still work on the data folder's default profile.

## The palette's actions, the shortcut sheet and pasting over a program's selection (round S, lane O; additive)

`keymap/lists.rs` (free functions; pure, no lock):

- `keymapPaletteActions(state: MenuStateRecord) -> PaletteActionsRecord`: core's `keymap::palette::actions` for the
  window `state` describes (the menus' facts: `off`, `disabled`, `hidden`, `split`, `debug`, `persistent`; the other
  fields are ignored): `actions` (`id`, `label`, `key`, `section` `Terminal`/`View`/`App`) in the order the palette lists
  them, and `suggested` (ids) for an empty query.
- `keymapSheet(hidden: [String], query: String) -> SheetRecord`: `sheet::sections_without(hidden)` through `sheet::filter`:
  `sections` (`title`, `rows`: `label`, `keys`, `range` (a run: first and last key), `labelHit` in bytes), `shown`, `total`
  and `exampleKey` (the key the footer's example filter finds, "⌘W"). `keymapSheetColumns(heights, columns)` is
  `sheet::columns` as half-open ranges of sections.

`Terminal` (`interaction/app_selection.rs`), over `wormhole_term::input::app_selection::AppSelection`, which the
terminal keeps itself (a left press and release reported to a program that tracks the mouse start it; a key sent or
`write` forgets it):

- `pasteClipboard(clipboard: String?)`: ⌘V. Over a selection the program drew on the cursor's rows it sends a Backspace
  first and pastes what the clipboard held before the program's copy; otherwise it is `paste`.
- `programCopied(text, clipboardBefore)`: a program's OSC 52 copy reached the clipboard, which held `clipboardBefore`;
  within a second of the program's own selection it is remembered as that selection's copy.

### A window in another profile, and tearing a terminal off (round S, lane W)

```swift
func rehome(window: Int64, workspace: Workspace, frame: FrameRecord?, to: Windows) -> Bool
    // "Move Window to": the record (terminals, notes, frame) is written into `to`'s database and removed from this one;
    // unlike `forget` the window stays among the live ones (its terminals run on). A profile moved to itself is true and nothing.
tearOffLanding(stack: [TearWindow], x:, y:, source: Int64, covered: Bool) -> TearLanding   // .stay | .newWindow | .into(window)
tearOffPlace(x:, y:, chip: TearRect, width:, height:, visible: TearRect) -> TearPlaced   // the same with the dragged chip (display space in, the window's own out)
tearOffGrowMs() -> UInt32                                                      // how long the window takes to grow from its chip (280)
tearOffGrow(chip: TearRect, width:, height:, radius:, elapsedMs: UInt32) -> TearGrow   // what shows of it then: rect, corners, content and face opacity, done
```

```swift
SessionNotesStore.forProfile(profile: String) throws -> SessionNotesStore   // a window's notes live in ITS profile's database
Windows.setKeepsChanges(keeps: Bool)   // `sidebar_changes_remember`: the files each terminal changed are saved with its window and restored
```

- A window moved to another profile (`rehome`) goes with its notes, tasks, flags, kept changed files and (when the target keeps text) its
  terminals' text, and is among the target's windows afterwards (hooks judge it by that profile). Ids are one counter for every profile
  (GPUI's rule), so only a database written before that can hold a window, terminal or split with the same id: that dormant window is
  renumbered from the shared counter (`windows/reissue.rs`); `rehome` is refused (false) only when it is open.
- `ProjectService.follow_profile(profile)`, `UsageService.follow_profile(profile)` (false for an unknown profile): the window's trust
  state and usage meters are those of the profile it was moved to.
- `insert_row_motion/insert_row_pose`, `fold_row_motion/fold_row_pose` (`row_motion.rs`): the core's sidebar pose tables by the settings'
  name (`motion.insert`, `motion.fold`): `RowMotionRecord { duration_ms, room_spring }`, `RowPoseRecord { opacity, dx, dy, clip }`.
- **Open Window in <profile>**: the shell asks `beside(profile)` for that profile's `Windows` once (kept) and plans, makes and
  saves the window there with the ordinary calls; Wormhole stays in the shown profile (the odd window, as the GPUI app's
  `open_window_in_profile`). Its settings are the shown profile's (the workspace reads the active profile).
- **Tear off** (`view::tear_off`): AppKit sends a drag's events to the window it began in, so the shell reads the app's windows
  front to back (`TearWindow`: its id when it shows this profile's terminals, else nil), the screens, and whether another
  application's window is on top (`covered`), and does what the core answers. All points are AppKit's global space (y up)
  except `tearOffPlace`'s, which are in one display's (from its top left, y down); the pointer lies at the new window's
  anchor (120, 90), kept inside the display's usable part.

- Not done: a window in a profile other than the shown one (menu "Open Window in" switches, then opens one), "Move Window to".
  Round S: the Trust window and, at launch, the Access desk follow the active profile; Imports apply through the settings handle (the
  active profile's); Session Hosts are the data folder's (core's `hosts_dir`), shared by every profile; the Access desk is not remade by a live switch.

## Settings consumers (`byte_size.rs`, `aux_backends/updater.rs`, workspace action, round S)

Additive, contract version stays 2. What the settings pages say reaches the windows:

```swift
byteSize(bytes: UInt64, binary: Bool) -> String      // core's git::byte_size: "48.2 KB" or "47.1 KiB" (size_units)
WorkspaceAction.setChangeRules(ChangeRules { plain: PlainChanges, shared: SharedChanges })
                                                      // sidebar_changes_plain / sidebar_changes_shared: the model makes the rows' ±N again
UpdateEngine.setAutoUpdate(on:) / setAnonymousCount(on:)
                                                      // auto_update: a check installs what it finds; anonymous_count: the check carries
                                                      // `Wormhole-Count` and the detailed user agent (first of a day / month, after the 24 h grace,
                                                      // which now starts when the engine is made)
```

- `setNumbersFollowList` now also raises `Event::Chrome` (the chrome snapshot says which way they count).
- The Access desk and the project Trust window are made for the active profile (`activeProfile().id`) when they are made. A live
  profile switch does not remake the desk the hook socket holds (the workspace's server cannot be stopped and started again in one
  run: "this workspace already has a server"), so after a switch the agent questions are still the launch profile's.

## Round S, terminal: live resize, opening a file at a line, find in folds (`live_resize.rs`, `open_file.rs`, `find.rs`)

- **Live resize** (core `view::motion::live_resize`; Swift's mirror is deleted): `resize_snapped(proposed, from, spare, cell, least) -> SizeRecord`
  (where a window dragged to `proposed` lands so the terminal's area keeps whole cells; the rule is `snapped`, new in core),
  `resize_settle(spare, cell) -> SizeRecord?` (the strip a window off the grid gives up, never a whole cell),
  `resize_settled_frame(window, strip, visible, held_left, held_top) -> WindowFrameRecord?` (`None` for a tiled or screen-filling
  window; AppKit's space, `y` is the bottom edge) and the object `LiveResizeGate` with `step(live, differs) -> ResizeStepRecord { apply, wake_in_ms }`
  (a size told during a drag at most every 100 ms, the last always). The gate reads its own clock. `TerminalCore.ResizeSteps` and `ResizeGate`
  call them once `UniFFISource.installResizeRules()` has run; without it nothing snaps and every size is told at once.
- **Open a file at a line**: `open_file_run(bundle, path, line, app_folder) -> bool`. The core picks the mechanism the app takes (`platform::open_file`:
  VS Code's URL, Zed's and Sublime's command line, JetBrains' `--line`, a plain open, Finder's reveal), this builds the process the GPUI app starts
  (`open -b`, `open -n -b … --args`, the app's own command line or the file alone when the app lacks it) and starts it, detached. Swift gives only
  `app_folder` (Launch Services) and never spawns anything. `TerminalProbes.openFile` is the seam.
- **Find and folds**: `Terminal.find_reveal` opens a fold that hides the match (as the GPUI find does, and it stays open); `Terminal.find_fold_words() -> [FoldWords { row, label, spoken }]`
  gives the markers of the last frame as they read while a search is open (how many matches are inside each; empty with no match or no marker).

## Round S wave 2, terminal: the cell (`cell.rs`)

- `terminal_cell(font_size, line_height, advance, ascent, descent) -> CellRecord { width, height, baseline, underline, strike }` (points; core
  `view::layout::cell::Cell`, which the GPUI app's `prepare.rs` uses too) and `terminal_grid_fit(cell, width, height, slack) -> GridFitRecord { cols, rows }`.
  Swift measures the font with CoreText and asks; `TerminalCore.CellRule` calls them once `UniFFISource.installCellRules()` has run.
- `cell_shapes(scalar, cell_width, height, phase, font_size, scale) -> CellShapesRecord? { shapes: [CellShapeRecord { rect, points, width }], alpha }`:
  what GPUI's painter draws for a box-drawing or block-element character (core `wormhole_term::glyphs`), in device pixels from the cell's left
  whole pixel; `phase` is how far the fractional cell starts past it. `None` for a character drawn from the font. Swift's own box drawing is deleted.
- `Terminal.prepare_frame() -> u64` and `Terminal.take_frame() -> bytes`: `frame()` in two calls, so a frame that changes nothing (no row,
  and the same header and sections as the last such one) crosses as the number 0 and allocates nothing on either side; `frame()` stays.

## Platform and notices (lane S)

- `HookEvent::Driven { terminal, typing, agent }`: a control request that drives a terminal (send-text, send-keys, mouse, run: typing; read,
  blocks, wait with a pattern: reading) was let in and answered ok, and the Notices flag is on: say it on the chip of the window that shows
  `terminal`. `agent` is the coding agent's name as the sentence has it ("Claude Code"), none for a script. Never for a refused request.
- `notice_reason(error) -> String`: what an error said as one short line (core's `view::notice::reason`), for the explanation behind a warning.
- `notice_data_folder_detail(profile, folder, error) -> String`: the data-folder warning's explanation in full.
- `Interaction::clear_history() -> bool` (was `()`): the history will come back at the next attach, because the terminal's session host is too
  old to forget it; the app says so on the chip.

## Round S2 (overlays): shells and a project's keys in the sheet

- `SettingsHandle.shell_choices() -> [ShellChoiceRecord]` (`key`, `name`, `command_line`, `program`, `args`, `is_default`): core's
  `terminal::shells::choices` of the effective settings, login shell first, each resolved to what starts it. The palette's and the menu's
  "New terminal: fish" rows; `WorkspaceAction.newTerminalRunning(program:arguments:)` starts that program (Swift side only: the core's
  `NewTerminal` opens the row, the bridge starts the program it was asked for).
- `keymap_sheet_matches(query, label, key) -> SheetMatchRecord?`: core's `sheet::matches` for a row the keymap does not hold (a trusted
  project's action and its key as glyphs): `None` when the query does not find it or `key` is no key; `label_hit` the bytes to mark.

## The terminal welcome (lane settings-b, wave 2)

The first run as a program in a terminal (`Wormhole welcome`, core's `onboarding::welcome`), behind the `terminal_welcome` flag.
- `cli_exit_code(["Wormhole","welcome","--",…])` runs core's program (`onboarding::welcome::run`); the app's executable is what a welcome
  terminal starts. `Terminal.start_for_welcome(tab, program, args, cwd, cols, rows, cell_w, cell_h, token, listener)` starts it in the app's
  own process (never a session host) with `WORMHOLE_WELCOME_TOKEN` in that terminal's environment alone.
- `HookServer.begin_welcome(tab, WelcomePlan { handover, first_run, animates, reduce_motion, import_home }, fonts) -> String?` opens the door
  (core's `Registry`) and starts the agent scan; the token goes to `start_for_welcome`. `welcome_playing(tab, playing)` starts/ends the
  animation clock behind the state's `wait_ms`; `set_notification_permission(word)` tells what macOS says of banners; `welcome_gone(tab) -> bool`
  shuts a closed terminal's door (a first run closed unfinished is remembered as skipped); `welcome_available() -> bool` is the flag.
- `Action::Welcome` is answered by the server (`state`, `set_look`, `set_notify`, `install_hooks`, `demo`, `import`, `finish`; budgets and the token
  rule are core's; `set_shown` is refused and `state.shown` is absent, so the program shows that page read-only). Settings go through core's
  `Config::update` of the window's profile; the open window sees them by its file watch.
- `HookEvent::WelcomeDemo { terminal, show }`, `WelcomeEnded { terminal }`, `WelcomeAskPermission`.
- The example band's rows are core's: `welcome_demo_title()`, `welcome_demo_label()`, `welcome_demo_rows(elapsed_ms, still)`, `welcome_demo_next_ms(...)`.

## Round S2 (panel B): worktree trust, logos, close stake, line to open

All additive; the core decides, Swift asks.

- `ProjectService.worktree_allowed(repo) -> String?` is `project::api::worktree_allowed`: a project at `repo` must be trusted with its
  Settings on (`ProjectHub::enabled`), judged on the files as they are now; no project, or the Project feature off in the shown profile
  (`FeatureMask`), no condition. The string is the refusal's sentence. `worktree_setup(repo) -> String?` is the trusted project's
  `worktree.setup` under the same rule (nil when refused). Swift's `PanelBackendsFactory` no longer decides trust.
- `UsageReportRecord` gains `name` (the card's name, `Agent::short_name`), `monogram` and `logo: AgentLogoRecord?` (the SVG path data
  and `even_odd` of the registry's bundled logo, 24 pt box). No colour travels: the panel draws it in its muted one, as `style::agent_logo`.
- `notes_close_stake(open_items) -> String?` is `view::notes::stake::discarded`: what the close question says of open notes and tasks
  ("notes and tasks are discarded."); Swift puts "Its" or "Their" before it.
- `GitDiffLine.open_line` is core's `lines_in_new_file` per hunk: the line of the new file a diff line's open icon opens the editor at
  (a removed line: the nearest).

## Round S3, engine: what a terminal runs, the engine's options and colours, window operations, saved text, notices

- **What a terminal runs** is decided in Rust (`launch.rs`), as the GPUI app's window does: `HostedStart` gains `shell: String?` (the
  key "New Terminal with ▸" chose, from `Workspace::launch_facts`), `profile: String?` (`None`: the one Wormhole is in) and `program`
  becomes optional: a program named there runs as it is (`--shell`, the welcome's tour, tests) unless `shell` names one; without it the
  shell is core `shells::for_terminal` (the terminal's own, a trusted project's (`ProjectHub::enabled`: project folders on, its Settings
  on, its files approved as they are now), `default_shell`, else the login shell with `-l`). The options carry the trusted project's
  `env`, then the profile's (`apply_profile`: environment, git identity, agent logins, `WORMHOLE_PROFILE`, start folder), Settings'
  `scrollback_lines`, `image_memory_mib` and the three `allow_*` switches (read at each start, so a change reaches the next terminal, as in
  the GPUI app), the theme's colours (below) and, for a restored terminal, its saved text drawn above the new shell. Core's
  `WORMHOLE_SHELL` replaces the program last, for test instances. `Terminal::start_planned(what, listener)` starts one in the app's
  process; `start_for_welcome(what, token, listener)` takes a `HostedStart` too; `HostedTerminals::start/attach` build the same
  options in the desk's profile. `Terminal.start*` with a program stay for tests and previews.
- `Action::NewTerminalWithShell(key)`: a new terminal that keeps its shell key; saved in the window's record (`SessionRecord.shell`) and
  restored with it. `Workspace::launch_facts(id) -> LaunchFacts { shell, saved_text, profile }`: ask as the terminal starts; the saved
  text is given once.
- **The theme's colours to the engine**: `set_terminal_colors(slots)` (free function), 259 `0xRRGGBB` (256 indexed, foreground,
  background, cursor) as drawn; the scheme is light by the background's luma (the GPUI palette's rule). Every live terminal is told at
  once (`set_default_colors`, `set_color_scheme`: OSC 4/10/11/12 queries, `CSI ? 996 n` and mode 2031 reports), later ones at spawn.
  Swift calls it whenever its palette changes. Telling the same colours again does nothing.
- **Frame layout 2**: after the fold markers, with header flag bit 5 (32), the colours programs set: a count, then `(slot, 0xRRGGBB)`
  word pairs, the whole table as it is now (count 0: all the theme's again). Sent when it changed and with every full frame while any is
  set; a frame without it leaves the table as it was. The renderer draws its palette with these over it: cells, the background behind the
  grid (OSC 11) and the cursor (OSC 12). Cells keep unresolved colours.
- **Window operations** (`window_ops.rs`): `TermEvent::WindowOp { op: WindowOpRecord }` (deiconify, iconify, move, raise, lower,
  resize in cells, maximise, full screen; only with `allow_window_ops`, the engine having taken the cells already). The sums are Rust's:
  `window_op_frame(op, place, pane) -> WindowFrameGoal?` (a move kept in the usable area, a resize to cells moved onto the screen, a
  one-way maximise) and `window_geometry(place, pane, iconified)`; rectangles in points from the top left of the window's screen. Swift
  carries the operation out, then after 400 ms calls `Terminal::sync_size()`; `Terminal::set_window(geometry)` whenever the window moves,
  resizes, minimises or the grid changes (not while a program's resize is pending): the reports (XTWINOPS 11, 13, 14, 15, 19) answer it.
- **Saved text** (`workspace_texts.rs`): `Workspace::save_texts()` saves the window's terminals in the app's process (hosted ones are their
  desk's), each minute and at quit, unless the profile keeps none or a row says not to; `Workspace::restore` keeps the record's texts for
  `launch_facts`. Erased at once: `Terminal.clear_history` (⌘K) and `WorkspaceEvent::SavedTextForgotten` (the FFI erases before Swift
  hears it); `Windows::erase_saved_text_if_off()` / `Workspace::erase_saved_text_if_off()` at window open and on a settings change;
  `HostedTerminals::stop_checkpoints_if_off()` stops the hosts' checkpoints and removes them when their settings went off.
- **Status and notices**: `Terminal.focus_changed` that sent a focus report feeds `StatusEvent::FocusReported` (a redraw answering it is
  not work). OSC 9/99/777 keep their words (`TerminalFacts.notice`, a serial); `wh notify` keeps its words without touching the status
  (`HookEvent::Notice` is gone: the notification rules tell it). A failure carries a serial, how long it ran and its last line.
  `TerminalSummary.notice` is the serial (a new notification moves the terminals' revision); `Workspace::notice_facts(now_ms) ->
  [NoticeFactsRecord]` gives the source (plain, agent, program), who (the agent's or the reporting program's name), `reported`, the notice,
  the failure (`at_ms` on the caller's monotonic clock) and the foreground command with its Unix start, for `TabFactsRecord`.
  `Workspace::attached_again(id)` after a host attached at launch brings back a "Done" its record said nobody had seen.

