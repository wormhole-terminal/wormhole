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
func revisions() -> Revisions                      // sidebar / layout / chrome counters
func selected() -> Int64?
func dropPlan(dragged: Int64, over: Over) -> DropPlan?   // pure: the line a drag would drop on
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
- `SidebarRow(id, name, number: UInt32?, keycap: String?, selected, pinned, state: RowState, agent: AgentInfo?, message, branch, folder, unread, bell, failureCode: Int32?, changes: UInt32?, notes, secondLine: SecondLine, split: SplitInfo, position, of, spoken, repo: String?, dirty, ahead: UInt32, behind: UInt32, repoFiles: UInt32?)`
  (the last five are additive, still contract version 2):
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
- `SidebarRow(id, name, number: UInt32?, keycap: String?, selected, pinned, state: RowState, agent: AgentInfo?, message, branch, folder, unread, bell, failureCode: Int32?, changes: UInt32?, notes, secondLine: SecondLine, split: SplitInfo, position, of, spoken, repo: String?, dirty, ahead: UInt32, behind: UInt32, repoFiles: UInt32?)`
  (the last five are additive, still contract version 2):
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

## The object: `SettingsHandle` (settings)

One profile's settings as the Settings window holds them, over core's `Config` (`config.json`: merged,
sanitised, written in place with comments kept, never over a file that does not read). Keys are
`config.json`'s (a path for one inside an object: `motion.insert`), values are JSON text; no Swift type
mirrors `Settings`. The schema the rows are drawn from is a free function. Code: `apple/ffi/src/settings/`.
Errors are `SettingsError` (`notIsolated`, `invalid(reason)`, `profile(reason)`, `agents(reason)`; `reason`
is core's sentence).

```swift
static func open(profile: String?, listener: SettingsListener) throws -> SettingsHandle   // nil: Default; isolated only; may create config.json
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
func commitDetails(folder, sha) -> GitCommitDetails                // message cleaned by core, files vs first parent
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

- **Diff = unified text.** Core parses once (caps, binary, LFS, too-large rules) and `patch.rs` writes the
  hunks back as unified text for Swift's `UnifiedDiffParser`: hunks are core's own lines, the header is rebuilt
  (modes are written as the usual ones). Whole files are not read for highlighting (Swift's lexer colours the
  text). `GitDiff` cases: `text`, `binary(patch, sizes)`, `lfs(summary)`, `tooLarge(bytes, title, detail)` (ask
  again with `loadAnyway`), `submodule(label, note, ahead, behind)`: the last three have no lines, and the words
  are core's (`diff_text`, `Submodule::label`).
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
