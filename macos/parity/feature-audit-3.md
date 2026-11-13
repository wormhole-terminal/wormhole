# Audit 3: is every row written? (2026-11-13)

After waves 4 and 5 (code written without compiling or running anything) `built.csv` had every row at 1. Seven readers
then went through the GPUI app's source row by row, and through the ledger itself, and counted a behaviour as written
only when the Swift and bridge code that does it was read and followed to where the app reaches it. Nothing was built or
run for this audit: it says what is written, not what compiles or works. Line numbers are of the branch `exp/swift-s5`.

**Written and reachable: 88.6 %** by the tracker's weights (32 of 107 rows with no gap found). The per-row numbers are
`written.csv` (steps of 5; 100 only where a gap was looked for and none found); `built.py` prints the figure.

| Area | Share of the app | Written | Rows with no gap |
|---|---|---|---|
| crosscut | 21.5 % | 86.6 % | 0 of 15 |
| panel | 15.4 % | 87.8 % | 0 of 9 |
| terminal | 13.4 % | 97.1 % | 24 of 31 |
| studio | 9.6 % | 89.2 % | 0 of 3 |
| window | 9.2 % | 88.8 % | 3 of 12 |
| sidebar | 7.9 % | 88.9 % | 3 of 10 |
| aux | 7.8 % | 84.7 % | 0 of 7 |
| settings | 7.3 % | 85.5 % | 0 of 7 |
| overlays | 4.3 % | 88.3 % | 0 of 5 |
| platform | 3.6 % | 84.8 % | 2 of 8 |

Not in that figure, because no row names them (the coverage part, last): running as the product (the app refuses any
bundle id and data folder but the experiment's; no packaging or signing step), log files and the panic record, the
Settings Components page, the data folder's files made owner-only at launch.

## Terminal (T01 to T31)

Read only, from the tree at the branch `exp/swift-s5`. Nothing was built or run. GPUI paths are under
`crates/wormhole/src/`, Swift paths under `macos/swift/Sources/`, bridge paths under `crates/wormhole-ffi/src/`.

```
id,written
T01,95
T02,100
T03,100
T04,100
T05,100
T06,100
T07,95
T08,100
T09,100
T10,100
T11,100
T12,100
T13,100
T14,100
T15,100
T16,90
T17,100
T18,100
T19,95
T20,95
T21,100
T22,100
T23,100
T24,100
T25,95
T26,100
T27,100
T28,100
T29,95
T30,100
T31,100
```

Weighted by the ledger's weights this is about 97 % of the area. The area differs from what the two earlier audits
found: its rows were last touched in wave 3 (partly driven live), not in the uncompiled waves 4 and 5, and nearly every
rule is the core's or the bridge's own Rust, called from thin Swift. The gaps below are small ones.

#### Rows below 100

##### T01 Grid drawing (95)
- The edge cells' background colour runs on into the pane's padding on all four sides, so a full-screen program with a coloured background has no theme-coloured frame around it (`terminal_view/render/paint.rs:160-176`, `prepare.rs:277` `reach`; double-size rows too, `sized::backgrounds`) → Swift: missing: the grid is drawn inside a viewport inset by the padding and the padding is "left clear" in the clear colour (the theme's or OSC 11's background) (`TerminalMetal/MetalRenderer.swift:381-387`; `RowBuilder.flushBackground` extends only a sized row to `gridRight`, never past the grid).

##### T07 Keyboard (95)
- Which ⌘ keys a focused terminal takes itself is looked up in the core's keymap table (`keymap::terminal_action`, then the same key with ⌘ alone unless it is an app key: `terminal_view/input.rs:91-116`) → Swift: partial, re-implemented: a fixed list in Swift (c, v, a, k, key codes 51/123/124) instead of the table (`TerminalMetal/Pointer/TerminalShortcuts.swift:13-31`); the app-key test does use the keymap (`Wormhole/Look/TerminalLook.swift:95`). Same result with today's table; a key added to or changed in the table's Terminal/Edit scope will not follow.
- Not judged (needs a keyboard): Control and Option-as-Meta chords on a non-Latin layout. Swift names the key from `charactersIgnoringModifiers` (`TerminalMetal/Keyboard/KeyMap.swift:27-33`); GPUI's keystroke key comes from gpui-pre's own layout handling, which I did not read.

##### T16 Pane bar, exited state, banners, clipboard prompt (90)
- The exited bar names the program the terminal ran: `"{shell} exited"` with `shell = wormhole_core::terminal::shell_name(&options)` (`terminal_view/hosted.rs:535-543`, `mod.rs:281`) → Swift: partial: the bar gets one name per window, `hub.shell`, which is `--shell`'s last path part or the literal "zsh" (`Wormhole/TerminalFactory.swift:28`, `Wormhole/AppController+Windows.swift:59`, `WindowUI/Overlay/TerminalPanes.swift:61`). A terminal that ran `default_shell`, a profile's or project's shell, or one opened with "New terminal: fish" says "zsh exited". The bridge already keeps the right name per terminal (`crates/wormhole-ffi/src/terminal.rs:403`) and the status bar uses it (`WindowUI/WindowBody.swift:161`); the exited bar does not.
- A notice said in the bar (chip off) is kept behind a question or a host problem and shows when that goes, until its 4 s are up (`terminal_view/pane_bar.rs:112-128`, `:176-186`) → Swift: partial: dropped when any bar is up (`WindowUI/Overlay/TerminalOverlay.swift:86-87` `guard bars.bar == nil`).
- The bar's notice is said in the terminal it is about (`self.say_in_bar`, `pane_bar.rs:104`, `blocks.rs` `say_refusal`) → Swift: partial: the window's route says it in the selected terminal's bar (`WindowUI/Hub/WindowHub+BarNotice.swift:9-12`); a program's copy goes to its own terminal directly (`TerminalOverlay+Clipboard.swift:19`), so only the routed kinds can land on another pane.

##### T19 Agent status per terminal (95)
- The status rules are GPUI's own code in `terminal_view/status.rs`, `reported.rs`, `settle.rs` (output to busy unless it answers typing, a resize or a focus report; OSC 133 marks; the bell once a minute; hooks, `answered`, `refresh_agent`; acknowledge) → Swift: a second implementation in the core, `crates/wormhole-core/src/workspace/status/` fed by `crates/wormhole-ffi/src/workspace_feed.rs` and `workspace_pump.rs`. Not a missing behaviour: every event GPUI handles has a `StatusEvent`, and the constants I compared are the same (`BELL_COUNTS_EVERY` 60 s, `TITLE_EVERY` 250 ms, `QUIET`). But it is the rule "re-implemented", and I did not diff it branch by branch; the GPUI app does not run this code, so its tests are the only check that the two agree.

##### T20 Persistent terminals (95)
- No missing behaviour found. Read and followed: start in a new host and attach off the main actor with the pane blank, the launch plan, orphans into the Recovered group, the fallback to the app with its once-per-run notice, "move to Applications", unavailable with one quiet retry after 1.5 s, not responding / Wait / End host, lost, detach or end at quit, headless when not shown (`WorkspaceBridge/RustWorkspaceBackend+Hosting.swift`, `+HostProblems.swift`, `crates/wormhole-ffi/src/hosting/desk.rs`, `checkpoints.rs`, against `terminal_view/hosted.rs`, `host_problem.rs`, `checkpoint_plan.rs`).
- Not traced to the end (so not 100): the confirm of a new host's first checkpoint before the old one is removed (`checkpoint_plan.rs` `CONFIRM_WITHIN`, `checkpoint_written`) on the bridge side beyond `Checkpoints::start`, and the app quitting while a start is still under way (`hosted.rs` `kept_past_quit`; Swift has `book.quitting` in `connected`, which detaches on the main actor after the start returns, where GPUI detaches on the background thread).

##### T25 Output folds (95)
- The wheel and a selection drag past the edge scroll by rows as shown, a closed fold counting as one row (`Pty::scroll_rows`: `terminal_view/input.rs:441`, `:506`; `crates/wormhole-term/src/handle/view.rs:52-61`) → Swift: partial: both scroll by grid lines, `pty.scroll(Scroll::Delta(lines))` (`crates/wormhole-ffi/src/interaction/pointer.rs` `scroll_wheel`, last branch; `crates/wormhole-ffi/src/terminal.rs:290` used by `TerminalMetal/Pointer/DragSelection.swift:13`). With a closed fold in view the scroll steps through lines that are not shown.
- ⌘↑/⌘↓ from a scrolled view starts at the line of the top row as drawn, which under folds is not "minus the offset" (`terminal_view/blocks.rs:109`) → Swift: partial: starts at `-displayOffset` (`TerminalUI/Blocks/BlockController.swift` `jumpPrompt`, the third `from` case). Differs only with a fold above the view's top.

##### T29 xterm window operations (95)
- A program sends its next operation or report at once after a resize; until the layout shows the asked size (or a second passes) the pane's size is the one asked for (`window_ops.rs:12-14`, `terminal_view/mod.rs` `size_asked`, `window_geometry.rs:31`, `:77`) → Swift: partial: each operation measures the pane from the grid as it is now (`Wormhole/Platform/WindowOps.swift` `carry` → `pane(of:in:)`); reports are held back for 400 ms (`pending`), but a second resize or a maximise sent right after the first is worked out from the old size.

#### Rows at 100 (what was checked)

- T02: ligatures off turns `calt`/`liga` off as `fonts::ligature_features` does, the fallback cascade, marks and joiners shaped as one cluster, the setting read live (`TerminalMetal/FontSet.swift:114-130`, `RowBuilder.swift`, `Wormhole/Look/FontSettingsMaker.swift`).
- T03: block/beam/underline, hollow without focus, blink only when asked, focused and Reduce Motion off, held on while typing, 530 ms, cursor-text colour, no cursor after exit, OSC 12 through the program colours (`CursorBlink.swift`, `RowBuilder.addCursorShape`, `TerminalScene.swift:254`, `MetalRenderer.swift:196`).
- T04: click/double/triple kinds, drag, autoscroll past the edge, ⌘A, copy with the chip, selection drawn over cells with the theme's selection text rule (`Pointer/TerminalPointer.swift`, `DragSelection.swift`, `RowMarks.swift`, `interaction/selection.rs`). Swift also has an Option rectangle, which GPUI's pointer does not start.
- T05: modes, pixels (1016), the DEC locator, Shift taking the pointer back, one report per new cell, only the middle of the extra buttons, arrow pointer while a program has the mouse, the program's own selection remembered (`interaction/pointer.rs`, `TerminalPointer.swift`, `TerminalLinks.swift` `resetCursorRects`).
- T06: fractions kept, at most 10 reports per event, sideways wheel, alternate-scroll arrows with application cursor keys, Shift to the history (`interaction/pointer.rs` `scroll_wheel`). The fold case is under T25.
- T08: ⌘-hover underline and hand, OSC 8 and text URLs, the scheme rules through `Openable`, no link in the padding, Open/Copy Link (`interaction/links.rs`, `TerminalLinks.swift`).
- T09: placements section (z below/above text, source rect, offsets, extents), placeholder tiles, textures kept under a budget with mips (`inline_images/section.rs`, `placeholders.rs`, `TerminalMetal/Images/`), draw order in `MetalRenderer.encode`.
- T10: runs, slider, click to centre, drag, fold ticks, takes the scroll bar's place; colours come from the drawn palette, which includes what a program set (`TerminalUI/Minimap/MinimapView.swift`, `MetalRenderer.swift:196`), so TO-VERIFY's "known unfinished" line no longer holds.
- T11: a real `NSScroller` (the owner's decision), hidden with no gutter while the minimap shows (`Scroller/TerminalScroller.swift:36`).
- T12: open with the selection's first line, ⌘E, ⌘G/⇧⌘G round the ends, options, count wording, invalid pattern, search again at most every 200 ms keeping the current match, matches inside a fold open it, fold counts (`TerminalUI/Find/FindModel.swift`, `find.rs`, `WindowUI/Overlay/ScreenSearching.swift`).
- T13: hover tint and readout, jumps with the VoiceOver words, target block, Select/Copy Output, Copy Command, Put at Prompt with the concealed/same-colour/invisible/lines refusals against the theme and program colours (`BlockController.swift`, `blocks.rs:172-200`).
- T14: sound once per glow, 150 ms glow in the theme's colour; the Dock bounce is off for the bell as in GPUI (`TerminalBell.swift`, `Settings/LookSettings.swift:65`).
- T15: items, order, separators, keys and enablement against `context_menu.rs`; the pane is selected first; a program with the mouse keeps the click unless Shift (`PlatformModel/Context/TerminalMenu.swift`, `Wormhole/AppController+Context.swift`, `Pointer/TerminalMenu.swift`).
- T17, T18: zoom per window, eased 120 ms and cut under Reduce Motion, new or moved-in terminals adopt it without easing, PTY told on arrival; the chip's rules (`noteSize`, 2 s, 700/150 ms) match `scaling/readout.rs` line for line (`MetalTerminalView+Zoom.swift`, `WindowHub+Zoom.swift`, `TerminalUI/Scaling/SizeReadout.swift`, `live_resize.rs`).
- T21: bracketed paste and control stripping in the core, files as escaped paths, paste over a program's own selection, drop tint, the drop takes the keys (`Keyboard/KeyDown.swift`, `interaction/pointer.rs` `paste`).
- T22: GPUI has neither (`actions/mod.rs:291` maps Print and ExportText to nothing); Swift has both (`Wormhole/Actions/ActionRouter+Output.swift`).
- T23, T24: tone, mask, block tint under the selection and find, reveal under the pointer, file links resolved link-safe off the main thread, the readout (`RowMarks.swift`, `MetalTerminalView+Probes.swift`, `WindowHub+Probes.swift`, `probes/terminal.rs`).
- T26: the one-shot deadline is the pump's (`workspace_pump.rs`).
- T27: `launch.rs` against `window/terminals.rs` and `TerminalView::new`: shell choice through `shells::for_terminal`, a trusted project's env and shell, the profile, worktree ports, `--shell`; every start (local, hosted, attach) takes it.
- T28: the five engine options from Settings, theme colours and scheme told to live terminals, per-window profile colours, program colours drawn including behind the padding (`launch.rs`, `engine_look.rs`, `frame_colors.rs`, `Look/TerminalLook.swift:85-91`).
- T30: saved each minute and at quit, skipped while a full-screen program hides the history (the row keeps its last text, `storage/store/scrollback.rs`), ⌘K erases at once (`interaction/pointer.rs` `clear_history` → `Touch::Cleared` → `workspace_texts.rs` `erase_saved_text`), the setting going off erases (`RustWorkspaceBackend+Hosting.swift` `startSaving`, `finishQuit`, `settingsChanged`). TO-VERIFY's "NOT BUILT: ⌘K" line is out of date.
- T31: the core's notice facts per terminal feed the core's rules; banners, clicks and replies are wired (`Wormhole/Notices/NoticeFeed.swift`, `workspace_notices.rs`).

#### Not covered by any row

- Typing a first command once the shell shows a prompt (`terminal_view/first_command.rs`; `wh new-terminal --command`): written in the bridge (`hook_server/control/open.rs:217-270`), named by no T row (perhaps a CLI row).
- Keys and pointer events from the control line (`terminal_view/control_input.rs`, `wh send-keys`, `wh mouse`): bridge `hook_server/control/input.rs`; no T row.
- The stream of bell, command and output events for `wh events` (`TerminalEvent::Bell`, `Command`, `Printed` in `status.rs`): bridge `hook_server/control/stream.rs`; no T row.
- "Hide from Agents" as a fact of the terminal (`status.rs` `agent_hidden`) and `grab_prompt` (what is typed in an agent's input box, `mod.rs:645`): no T row.
- The cell the welcome's ship docks on (`TerminalView::set_dock`): no T row.
- The session host binary beside the app (`ipc::host_binary`, `hosting/program.rs` `locate`): whether the Swift app's bundle carries `wormhole-host` is packaging, in no T row; T20 depends on it.
- Swift has what GPUI lacks and no row counts: input methods and dead keys (`Keyboard/InputClient.swift`), Print and Export, the Option rectangle selection, the terminal's accessibility node.

## Right panel (P01 to P09)

```
id,written
P01,85
P02,85
P03,85
P04,90
P05,75
P06,90
P07,90
P08,90
P09,85
```

Read only, nothing built or run. Paths: GPUI under `crates/wormhole/src/`, Swift under `macos/swift/Sources/`, bridge
under `crates/wormhole-ffi/src/`. "audit 2" is `macos/parity/feature-audit-2.md`. Most of audit 2's gaps are closed in
the code and reachable (per-terminal state of Changes / History / Files, the review comments per terminal, File
History switching the strip, the one cover, CRLF lines, the picture decoder, the trust window's held approval, the
window's own profile for trust and pins, the notes.md merge saved at once, the submodule card, the wrapped ref chips,
the age timer, the palette's commit, tree-sitter colours from the bridge). What follows is what is still open, and
what audit 2 did not see.

#### P01 Panel shell (85)

All of these were in audit 2 and are unchanged in the source:
- A tab's action is bound only while the tab is offered; ⌃⌘4 without an agent (or a tab whose feature is off) does nothing, "Show Usage" is hidden, "Show Project" disabled (window/feature_host.rs:288-296; features/usage/def.rs:36) → Swift: partial: the request is turned into Changes and the panel opens (WindowUI/Hub/WindowHub.swift:100-102, whose comment says "as in the GPUI app" but GPUI drops it); the menu hides by `flag` only, not by the keymap's `feature` tag (WindowUI/Menu/MenuStateMaker.swift:84). Only `newNote` / `stashPrompt` are guarded (Wormhole/Actions/ActionRouter+View.swift:34-35).
- Under Zen, showing a tab switches it without revealing the panel (window/layout.rs:63-73) → Swift: missing: `.showPanelTab` goes to the core whatever Zen says (WindowHub.swift:103).
- A tab on show that is no longer offered is left on the window's tick, panel open or not (window/panel.rs:31-34, tick.rs:80) → Swift: partial: only from `RightPanel`'s `.onChange(of: hub.panelTabs)` (WindowUI/RightPanel.swift:39), which exists only while the panel is on screen and never fires for a restored layout: the Usage body with no tab marked.
- The panel's title-row button, when it closes the panel, gives the terminal the keys (window/layout.rs:93-100) → Swift: partial: the key and menu do (ActionRouter+View.swift:15-20), the button only sends `.togglePanel` (WindowUI/WindowControls.swift:35).
- Tab tooltips name their key ("Changes ⌃⌘1"), as do the stash button and the Notes "+" (window/chrome.rs:278; features/notes/view.rs:31-33, 427) → Swift: missing: bare words (WindowUI/PanelTabBar.swift:94; PanelUI/Notes/StashPromptButton.swift:26).
- VoiceOver: tab role in a tab list, and each tab's own words with counts ("Changes, 3 changed files", "Notes, 2 open next steps, 1 note") (window/chrome.rs:279-286; core view/spoken.rs `panel_tab`) → Swift: partial: a button, the bare title, value "2 of 6, 3" (PanelTabBar.swift:30-33, 51-54).
New:
- Content slides in from the side of the tab that was left, by the strip's order (window/panel.rs, tab order of `features::TABLE`) → Swift: partial: the direction is computed from `PanelTab.allCases`, the enum's order (notes, changes, history, …), not the bar's (changes, history, notes, …): Changes → Notes slides in from the wrong side (WindowUI/RightPanel.swift:52-56; WorkspaceModel/Model.swift:24).

#### P02 Changes (85)

- Every path the list hands on is under the repository's root (`status.root`): open in editor, Reveal, Copy Path, "is it on disk", the submodule's terminal, the stamp that re-reads an open diff (features/git/changes/list.rs:62, 84, 162; core git/status.rs:52 runs at the root, so paths are root-relative) → Swift: partial: works only while the terminal sits in the repository's top folder. `panelRoot` is `controller.folder`, the terminal's cwd (PanelUI/Shell/PanelContentView.swift:28, PanelCoverView.swift:28, PanelCoverTitle.swift:48), and `ChangesStore.stamp` resolves against `folder` too (PanelUI/Stores/ChangesStore.swift:163-166); the FFI's `GitStatus` carries no root (ffi git/records.rs:73-86). In `repo/crates/x`: Copy Path copies `repo/crates/x/<root-relative path>` (Changes/PathMenu.swift:45), Open in and Reveal are greyed because `exists` is false (PathMenu.swift:27-31), the row's open icon hands the editor a relative path (Changes/FileRowView.swift:49), a submodule's "open a terminal" opens the wrong folder (Changes/SubmoduleRowView.swift:34), and an open diff is not read again when its file is edited with unchanged counts (stamp nil). `PanelController` already knows the root (`roots`, PanelController.swift:86-93) and gives it only to Files and Project.
- The title row's branch chip shows the Changes tab and opens "compare with" (features/git/changes/def.rs:108-117 `branch_chip`; window/panel.rs:113) → Swift: unreachable: the chip calls `actions.pickBranch` (PanesUI/Title/BranchCrumb.swift:19 → WindowUI/Hub/WindowHub+Title.swift:24), and `HubActions.pickBranch` is the empty default nobody assigns (WindowHub.swift:148); `RevPicker` can only be opened by its own click (`@State open`, PanelUI/Shell/RevPicker.swift:25). (Shared with W02.)
- The compare branch is the terminal's and survives a `cd` (window/repo/state.rs:39-41; only `compare_with`, poll.rs:66-68, changes it) → Swift: partial: kept per terminal, but a folder change in the same terminal sets `compare = nil` (ChangesStore.swift:91).
- A path that leaves the repository (a planted link) is refused before open or reveal (features/git/open_file.rs head; `files::resolve_within`) → Swift: partial: `PathMenu.resolved` and the row's icon fall back to the unresolved path when the core refuses (`?? absolute(path)`, PathMenu.swift:49; `?? file.path`, FileRowView.swift:49), so a refused path is handed on anyway.
- Stale tab words from audit 2 still open: none further; nested repository row, picker hints, Copy error chip, ⌘-click on a submodule, the Finder case of the open icon, the refresh on show and File History are in the code.

#### P03 Diff (85)

- The same root error as P02 in the diff: the header's open button, the per-line open icon, "Reveal" on the too-large card and LFS note resolve `file.path` against the cwd (PanelUI/Diff/DiffHeader.swift:75; Diff/DiffView.swift:128; Diff/DiffNote.swift:82) → Swift: partial: in a subfolder of the repository `RepoFiles.resolve` answers nil, so the header button and the line icons are gone and Reveal does nothing; GPUI uses `view.root()` (features/git/diff/mod.rs:159).
- The comment form is a dialog titled "Comment on <place>" with "Add Comment" / "Save" (features/git/diff/review.rs:92-178) → Swift: partial: a popover on the line (Diff/CommentEditor.swift; the lane lists it); the review rules are a Swift copy of core's `git::review` (PanelModel/Review/Review.swift) that differs in small ways: the quote is cut at 160 grapheme clusters and trimmed of spaces only, core cuts at 160 `char`s and trims all white space.
- Loading says "Loading diff…" (features/git/diff/render.rs:432) → Swift: "Reading the diff…" (Diff/DiffView.swift:164); wording only.
- A change of `tab_width` rebuilds the open diff's rows (features/git/changes/mod.rs `sync_diff`, rows' `BuildKey`) → Swift: partial: the widest line is measured once with the tab width at read time (PanelUI/Stores/DiffSession.swift:87-93); a later change redraws the tabs but keeps the old sideways extent until the diff is read again.
- Syntax colours: the bridge's own engine (ffi syntax/, git/colors.rs → GitBridge/GitDiffFile.swift:35) is wired and the factory installs `CoreSyntaxHighlighter` (Wormhole/PanelBackendsFactory.swift:44); differences from GPUI Kit's queries are the owner's decision, not counted. No parse cache (GPUI keeps 32): not counted.
- Closed since audit 2 and read: wrap-around of ‹ ›, "File N of M" at any width with the commit's hash, the submodule badge and card, "Renamed from", re-read when the base moves, rows cut once (`DiffBuilt`), close giving the terminal the keys (`closeCover`).

#### P04 History (90)

- Paths of a commit's files are under the repository root → Swift: partial, the P02 root error again: in a subfolder the file menu's Copy Path is wrong and Open / Reveal are greyed (PanelUI/Changes/PathMenu.swift:27-31, 45), and the row's open icon, shown because the FFI's `present_files` does use the root (ffi git/asked.rs:33-36), opens a bare relative path (Changes/FileRowView.swift:49).
- ⌘C on a commit says "Copied" on the notice chip (features/git/history/copy.rs:33-43; keys.rs) → Swift: partial: the hash goes to the pasteboard through `onCopyCommand` with no notice (PanelUI/Shell/SelectableListSplit.swift:51); the menu and button copies do post it.
- `gh` and the worktree store under the window's own profile: `approved` now takes the window's profile (ffi workspace_extras.rs `profile_of`); read, not followed further.
- Checked and found written: per-terminal rev / filter / open commit, details re-read when refs move, `paths` in the refresh guard, the palette's commit (`reveal`, `is_on`), age timer on core's `next_age_change`, wrapping refs and parents, `present_files`, picker words and hints, Esc only closing the picker, the details menu on ↩, PR checks notification (`tell_checks_settled`).

#### P05 Worktrees (75)

All from audit 2, none taken up (the lanes' own "not built" list says so, and the source agrees):
- `new-terminal --worktree <b> --command <c>` runs `setup && command` (features/git/worktree/ask.rs:97-132, 343-360) → Swift: missing: refused as Unsupported (ffi hook_server/control/worktree.rs:22-28).
- The command line answers Refused for an untrusted project and Failed when a dialog is already up (ask.rs:120-125) → Swift: missing: always `asked: true` (control/worktree.rs:44-51); a second request resets the sheet that is up (PanelUI/Stores/WorktreeStore.swift:80-93).
- Trust is asked of the repository (the main working tree), and not at all with project folders off (ask.rs:85-86, 272; features/project/api.rs:90-106) → Swift: partial: asked of the terminal's folder (GitBridge/RealWorktreeBackend.swift:29, 41), so from inside a linked worktree the wrong `.wormhole/` (or none) decides.
- Closing a group looks at every terminal of the window (features/git/worktree/close.rs:61-73) → Swift: partial: built from `sections…rows` (Wormhole/AppController+WorktreeClose.swift:21), so a folded group's terminals are not seen, and "inside the worktree" is a string prefix (`hasPrefix`, line 27: `…/fix-login2` counts as inside `…/fix-login`).
- Worktrees (and so the ports of a terminal started in one) are remembered per profile (app_state/profiles.rs:230-236) → Swift: partial: `Store::open()`, the Default profile's database, in `load`, `save` and `env_for` (ffi git/worktrees.rs, about lines 174-201).
- A trust refusal at Create keeps the dialog up with the reason (ask.rs:268-278) → Swift: partial: the sheet closes and the reason is an alert titled "Couldn't create the worktree" (WorktreeStore.swift:112-120).
- The setup line goes into the terminal the window opened for the worktree (ask.rs:355-360) → Swift: partial: pasted into `.current` after two awaits, straight after `newTerminalIn` (Wormhole/PanelBackendsFactory.swift:66-71): whichever terminal is selected then, and nothing waits for the new terminal's view to exist. Needs a run to know.
- Written and read: the sheet, the plan following the typing, `worktree_base` and clone build folders from Settings (Wormhole/Settings/PanelSettings.swift:26), ports in `spawn_options`, the close question for one terminal, removal.

#### P06 Notes and tasks (90)

- The draft step and the fold of "done" belong to the terminal (features/notes/def.rs:86 `selected`) → Swift: partial: `drafting` / `draftAfter` are the window's one `NotesStore`'s and are not cleared when another terminal is selected (PanelUI/Stores/NotesStore.swift:20-21): an open draft row shows under the next terminal's steps.
- A merge is applied to what the terminal holds now (features/notes/api.rs:54-98, on the UI thread) → Swift: partial: `sync()` hands the model to an `await` and then replaces the tab's model with the answer (Wormhole/Panel/NotesCoordinator.swift:118-127); a tick or a new step made during the await (no editor open, so not "writing") is overwritten by the merge of the older model.
- Stash prompt and "Path copied"-style notes: `stashPrompt` only speaks to VoiceOver (NotesStore.swift:118); GPUI posts no chip either: not a gap.
- Checked and found written: merge saved at once, ⇧⌘E into the notes (`focusIn`, WindowUI/Focus/WindowFocus.swift:149), the draft step's keys and actions, Send focusing the terminal and Return 120 ms later, the selection before the input box, the feature-off guards and the sync gated on Notes and Project, the wrapped-lines preview through core's `note_cap`, `note_body`, the keymap's item keys, "being written" from the editors.

#### P07 Files (90)

- ↩ on the path head opens the path menu under it (features/files/keys.rs:63-69 `press_row` `Line::Head`) → Swift: missing: `press` looks the row up among the entries and returns for the head (PanelUI/Stores/FilesStore+Walk.swift:22-25); the menu is a SwiftUI `Menu` only a click opens (Files/FilesPathButton.swift:14). The lane's note "return opens nothing there in GPUI's code either" is wrong.
- A terminal whose own folder is not UTF-8 (paths are `PathBuf` throughout) → Swift: missing: the root comes from the workspace as text (the lane says so).
- Files' path actions are right (resolved inside the root by the core first, FilesStore+Open.swift), unlike the git tabs'.
- Checked and found written: tree of the repository root, per-terminal tree and chosen folder, one cover, CRLF and last-line cutting, the picture decoder off the main thread with the too-large sentence, fixed line numbers, pins per profile, the crumb as a button with the path card, resolve-within before any hand-over and no default-app fallback (WindowUI/SystemPanelHost.swift:79-84), rename's busy guard and catalog sentences, the icon theme (`FileIconSource.shared`), no reads while hidden, stamps, non-UTF-8 names by key.

#### P08 Project (90)

- "Open terminals" skips a terminal whose own (custom) name is the layout's (window/feature_terminals.rs:100-126) → Swift: partial: matched by the shown title (Wormhole/Panel/PanelEffects.swift:63-65), so a terminal whose automatic title equals the name blocks the layout's terminal, and one renamed by the person but showing another title is opened twice. Also every layout terminal is selected as it is made (`make` reads `hub.state.selected`), `focus` or not (PanelEffects.swift:44-50, 67-73).
- An action's key is the window's wherever the focus is, and lets the key through once the project is not trusted (features/project/actions.rs:56-70; app/handlers.rs:258-275) → Swift: partial: claimed from the focused terminal and by a menu key equivalent; a text field takes the key first, and between a revoke and the next read the key is swallowed (the lane says both).
- Per-profile colour, hub problems in the no-project card, `.andromeda` head, the Project switch honoured in the FFI, the theme overlay (`ThemeStore.projectTheme`, Wormhole/Settings/ProjectSettingsFollower.swift:57-68), the band's folder for colour and Create, Copy into a new note showing Notes, the held approval in the Project commands window (AuxBridge/TrustBridge.swift:43-54): read and found written.

#### P09 Usage (85)

- The words redraw at each word's own boundary: every second in a limit's last minute ("resets 45s"), each report's minute for "Updated Nm ago", the reset itself (features/usage/def.rs:42; core agents/usage `next_change`) → Swift: partial: `TimelineView(.everyMinute)` (PanelUI/Usage/UsageView.swift:33): no countdown in seconds, each word up to 59 s late.
- Reports are kept per profile and the windows draw the shown profile's (features/usage/state.rs:26-51, 85-95) → Swift: partial: one shared service bound to the first window's profile; "Move Window to profile" moves it for every window (the lane says so).
- One service for every window and the hook socket: written (Wormhole/PanelBackendsFactory.swift:33, 51-53).

#### Not covered by any row

- The title row's branch chip opening "compare with" sits between W02 (the chip) and P02 (the picker); neither row's notes name it, and it does nothing (see P02).
- Settings → Changes & diffs draws a live sample diff from the diff view's own rows (features/git/diff/preview.rs); P03 does not name it and I did not look for it in the Swift Settings (the settings rows' auditor would).
- `window/repo/attribution.rs` and `remembered.rs` (a row counts the files its own terminal changed; kept across restarts, `sidebar_changes`, `sidebar_changes_remember`) are under `window/repo/` in my area but belong to the sidebar rows; not checked here.

#### Could not judge from reading

- Whether the wave 4 and 5 code compiles (written blind); I only checked that the FFI functions the Swift side names exist.
- Timing: the worktree setup paste and a layout's `run` written right after a terminal is made; the Notes merge race.
- Whether the bridge's grammar list covers every language GPUI Kit colours.

## Sidebar and window (S01 to S10, W01 to W12)

```
id,written
S01,95
S02,95
S03,90
S04,100
S05,100
S06,100
S07,95
S08,75
S09,90
S10,85
W01,85
W02,80
W03,85
W04,100
W05,90
W06,95
W07,100
W08,95
W09,85
W10,100
W11,90
W12,85
```

Weighted by the ledger's weights: sidebar 88.9 %, window 88.8 %, both together 88.8 % (not 100).
Read only; nothing was built or run. Paths: GPUI under `crates/wormhole/src/`, Swift under `macos/swift/Sources/`.
Most rules of both areas are the shared core's (`wormhole-core/src/workspace`, reached through `wormhole-ffi`), so the
gaps are in what the Swift views are given and what they reach, not in the list's or the panes' logic.

#### Gaps per row

##### S01 Sidebar list (95)
- Coming into the list by keys (F6, Tab) with the selected terminal folded away puts the keys on its group's head (sidebar/keys.rs:73-90 `enter`) → Swift: partial: the cursor is set to the selected terminal's row, which the walk does not hold, so the first arrow starts from nowhere (SidebarUI/SidebarView.swift:127).
- Not a behaviour gap, will not compile as written: `RenameField.font` has a default value that reads the instance's `tokens` (SidebarUI/Row/RenameField.swift:16); both the row's and the group head's rename go through it.
- Checked and found written: row menu item by item against `row_menu.rs` (same 12 items, checks, greyed states, Show beside by room, fenced groups greyed), close x through the question, tooltips of a cut name/second line, lock and worktree marks with two-line tips, 600 ms flash, ring while a menu is open, notes and ±N (terminal or repository scope, once per repository), Space/Return/arrows/type-ahead/Ctrl-Return, status dot off.

##### S02 Inline rename (95)
- Same `RenameField` default-argument fault as above (Row/RenameField.swift:16): the only path to a rename.
- Otherwise written: whole name selected, Return and click elsewhere keep, Esc drops, keys back to the terminal (TerminalRowView.swift `rename`, GroupHeadView.swift `name`). Selecting all is an async `NSApp.sendAction(selectAll)`; an emptied name is sent to the core as "" (core resets it): could not be judged without running.

##### S03 Groups (90)
- The folder question has a checkbox "Only terminals in this folder" and, when terminals would leave, a second detail text with the count; "Not Now" and closing the sheet are the same answer (window/group_folder.rs:110-150, 155-185) → Swift: partial: an alert with Use Folder / Not Now / Cancel, no checkbox, no leaving count, Cancel opens no terminal (SidebarUI/Group/GroupFolderQuestion.swift:11-29).
- The question is asked wherever a terminal is made in a group (`new_session_in_group`, group_folder.rs:92) → Swift: partial: asked from the head's +, the popover and the group menu; not from the tab bar's +, an empty pinned set opened from the switcher or by Ctrl-Opt-1..9 (WindowUI/Hub/WindowHub+Sidebar.swift:89, 126 send `.newTerminalInGroup` directly).
- Checked and found written: head (dot, pin, folder line, folded tiles loudest first, count, alert dot, + under the pointer and on an empty pinned group), popover (name field, no-colour ring, nine swatches, picker, folder lines, five actions, keys back to the terminal), native menu with swatch row / Other colour… / No colour / folder lines, in-place rename, Ctrl-Return menu, custom colour wash, title tint.

##### S04, S05, S06 (100)
- S04: density read from `density` (SidebarSettingsMaker.swift), one-line rows in compact and dense, the group head's folder line rule, cards on/off.
- S05: pin item and mark, numbers are the core's (frozen under "Stay with the terminal" in `workspace/model.rs:387-409`, set through `setNumbersFollowList`), Cmd held alone 250 ms shows keycaps on rows and on tabs, any key or a resigned window hides them, `sidebar_cmd_hints` off hides them.
- S06: no sidebar search in either app; Cmd-F from the list opens the terminal's Find.

##### S07 Drag and drop (95)
- With the Motion feature on, the part of a row under the pointer is held by the line's edge the pointer came from (`Part::at_from`, sidebar/drop.rs:197-200) → Swift: missing: the part is the pointer's share of the row's height only (SidebarUI/Drag/DropLines.swift `over(at:)`).
- Checked and found written: row and group drags (own gesture), the core's `dropPlan`/`groupDropPlan` for every landing, the line / card / row marks, a row dragged onto a pane edge (hint zones, refusal by room), a pane header dragged onto the list (system drag: join, move, leave the split below the rows, not on its own split's row), Esc, tear-off (flag, not in full screen, never the last terminal into a window of its own, into another window of the same profile, chip outside the window, growth from the chip). The system drag of a pane header and the outside-window chip need a real pointer.

##### S08 Sidebar motion (75)
- While a row is dragged the lines it passes step aside on a spring and open a gap where it would land, the dragged row leaves the list, and no drop line is drawn then (sidebar/motion/room.rs:1-14, list.rs:83-85) → Swift: missing: the list never changes during a drag; a line is always drawn and the rows only move after the drop (SidebarUI/Drag/SidebarView+Drag.swift `dragLayer`, no code for the room).
- A row coming or going plays the chosen `motion.insert` pose wherever it stands, and a closed terminal's row stays drawn while it leaves (motion/insert.rs, motion/leaving.rs:1-9) → Swift: partial: only rows in no group have the pose transition (Group/SidebarSectionView.swift:21); a row inside a group's card (GroupRows.swift `row(item)`) comes and goes with no pose.
- Without the Motion feature every motion is Classic (`motion_in_force`, wormhole-core/src/config/settings/mod.rs:464-467; off in a release build, features.rs:249) → Swift: differs: only `motion.column` is held to Classic (Wormhole/Settings/LookSettings.swift:43-45); `motion.status`, `motion.insert`, `motion.fold` are applied whatever the flag says, and where a value is absent Swift's own defaults are badge-pop, from-side and tuck (SidebarUI/SidebarAppearance.swift:66, Settings/SidebarSettingsMaker.swift:20-22), so a release build moves where GPUI cuts.
- A split just made draws its bracket down the two rows while they settle (motion/join.rs:1-5, row/face.rs:300-306 `draw_bracket`) → Swift: missing: the split line appears at once (Row/SplitLine.swift); the join's row tint and the chip snapping onto the row are written (LiftedPicture).
- The status bar's counts roll when they change (window/status_rolls.rs) → Swift: missing: a fade (WindowUI/StatusBar.swift:102).
- Checked and found written: fold Tuck/Drawer per row from the core's tables, folded summary waiting for the rows, Flare, Nudge, Wash, LineRoll, crossfade, badge pop, lifted row picture, Reduce Motion cutting each.

##### S09 Terminals as tabs (90)
- The +, an empty pinned set and Ctrl-Opt-N on one ask the group's folder question before the terminal opens (sidebar/tab_bar.rs:142-150, window/sidebar_host.rs:505, group_folder.rs:92) → Swift: missing on these three paths (WindowHub+Sidebar.swift:89, 126); see S03.
- Which set alerts (the dot on the closed switcher, the card's dot) is decided by the tile's urgency, needs-you or failed, as the row shows it (tab_bar.rs:139 `others_need_you`, switcher.rs:205-210, amber or red) → Swift: differs: decided from `activity` (`needsInput || failed`), always amber, and the sets, stepping, numbering and search are a Swift copy of core's `sidebar::tabs` (PanesModel/Tabs/TabSets.swift, TabBarModel.swift:49); the copy matches the core's rules today, the dot's colour and source do not.
- The switcher's search rows carry the terminal's tile or the group's colour dot (switcher.rs:300-312) → Swift: missing: text only (PanesUI/Tabs/GroupSwitcherView.swift `hitRow`).
- A click on a tab gives that terminal the keyboard (`Activate`, sidebar_host.rs:486) → Swift: could not judge: the tab sends `.select` only, where the sidebar's row also calls `focusTerminal` (WindowHub+Sidebar.swift:87 against SidebarUI/Keys/SidebarView+Keys.swift:60).
- Checked and found written: bar shown by the core's rule, tabs with the row's own tile, x, Cmd keycaps, context menu (the row's), right-click of + with shells, profile chip, switcher (search, cards with wash, tiles loudest first, ring, + on an empty pinned card, popover on the name, native group menu, New group, Show all groups in the sidebar saved to `terminals_in`, key hints from the keymap), last-selected terminal per set, folded groups keep their tabs, VoiceOver labels from the core.

##### S10 Sidebar extras (85)
- The no-agents note is raised when the first run ends on a machine with no agent, by the terminal welcome and by the intro window alike (onboarding/mod.rs:465 `primer`, window/welcome_host.rs:232) → Swift: partial: raised only when the terminal welcome ends (Wormhole/Hooks/HookHost+Welcome.swift:43); the intro window's ending never sets `hub.agentsTip`.
- Checked and found written: the note with x, "Tip closed" with Don't show again (persisted through the FFI), Coding agents settings, hidden once a terminal runs an agent; New terminal with shells on a right-click of the + (sidebar title row and tab bar); lock and worktree marks.

##### W01 Splits and panes (85)
- A split opening (Cmd-D, Shift-Cmd-D) is played from the layout before it: the new pane slides in on a spring over the split pane's room, the old terminal keeps its grid's size until the end and is resized once (window/panes/opening.rs:1-19) → Swift: missing: the new layout is animated as a whole (`.animation(Motion.layout, value: panes)`, PanesUI/Panes/PaneArea.swift:49) with an opacity-and-scale transition; nothing holds the old terminal's size.
- A pane's header shows the terminal's own tile: the agent's logo with its live status (render.rs:247 `terminal_tile(agent, told, status)`) → Swift: partial: the plain terminal symbol with a state dot, never the agent's logo (PanesUI/Panes/PaneHeader.swift:24, TerminalTile.swift); `PaneInfo.agent` is passed and not drawn.
- The pane menu's items and the Zoom and Close tooltips carry the keys the keymap has now (panes/mod.rs:376 `keymap::key(action)`, render.rs:203, 210) → Swift: partial: fixed strings Cmd-D, Shift-Cmd-D, Shift-Cmd-Return, Cmd-W (PanesUI/Panes/PaneMenuItems.swift:11-22, PaneHeader.swift:56, 70): a changed key is not shown.
- Checked and found written: the core's rects and dividers, header (name, folder cut from the front, buttons giving way from the left, primary line or outline), unfocused pane dimmed, zoom, focus by direction, divider drag with grab offset, hover delay, double-click to equalize, Esc restoring, drop zones with the hint text and dashed alternatives, refusal by room, View > Layout presets, Move out of split, close through the question, exited bar.

##### W02 Title row (80)
- After the name the row says how the terminal is doing with its time: "working · 4m", "done · 3m", "failed · exit N", redrawn by the clock (window/title/breadcrumb.rs:457-473 `crumb_words`) → Swift: partial: the bare word, no time, no exit code, "finished" where GPUI says "done" (WindowUI/Title/TitleFactsMaker.swift:17, 38-48).
- A terminal hidden from agents wears a lock after its name with "Hidden from agents" (breadcrumb.rs:340-345, mod.rs:79) → Swift: unreachable: `TerminalCrumb` draws it from `facts.hiddenFromAgents`, which `TitleFactsMaker.facts` never sets (TitleFactsMaker.swift:13-19; PanesUI/Title/TerminalCrumb.swift:23).
- In a submodule the branch chip is followed by whose it is, "in <parent>" (breadcrumb.rs:55, 244-296) → Swift: unreachable: `BranchCrumb` draws `facts.superproject`, never set (TitleFactsMaker.swift; PanesUI/Title/BranchCrumb.swift:36).
- A detached HEAD shows the amber "detached" chip (`branch: Some(None)`, title/mod.rs:70) → Swift: unreachable: `branch: terminal.branch.map { .some($0) }` can never be `.some(nil)` (TitleFactsMaker.swift:18), so a detached repository shows no chip (or whatever string the core put in `branch`).
- The terminal crumb's tile is the tool's own icon with the row's live status (breadcrumb.rs:515-545) → Swift: partial: the plain terminal tile with a dot (TerminalCrumb.swift:15).
- Files dropped on the title row go to the terminal under it (title/mod.rs:212-216, 409-413) → Swift: missing (no drop target on the row; only the Metal view takes files).
- Insert into terminal is off only for an exited terminal (path_card.rs:207) → Swift: differs: off when there is no folder (`canInsert: folder != nil`, WindowUI/Hub/WindowHub+Title.swift:33), so it stays on for an exited one.
- Checked and found written: parts arranged by the core, fit and elision, folder crumb (click reveals, hover card with three buttons and "Copied", Finder-style path menu), branch chip to the compare picker, branch status to Changes, gauge, port chips with "+n", pull request chip, Open in (icon, chevron menu, missing app tooltip), split buttons, the folded "…" menu, the row's own menu (parts, reset, customize, off in an odd window and under Zen), cover title.

##### W03 Status bar (85)
- View > Show/Hide Status Bar writes `status_bar` to config.json: one switch for every window, kept across launches, the same switch as Settings (app/handlers.rs:125-129) → Swift: partial: the action flips the app's look in memory only (`look.statusBar.toggle()`, Wormhole/Actions/ActionRouter+View.swift:21); nothing writes `status_bar`, so it is lost at relaunch, Settings does not follow, and a window of another profile (its own look) does not change. The minimap toggle beside it does write its setting.
- The state item of a failed terminal says "failed · exit N" (breadcrumb.rs:466-469, used by status_bar.rs:267) → Swift: partial: "failed · 2m" (WindowUI/StatusBar.swift:108-114; the lane's own note: the model carries no exit code).
- The attention button's tooltip ends with the key the jump has now (status_bar.rs:478-481) → Swift: partial: a fixed "(⌃⌘↓)" (WindowUI/StatusSummary.swift:65).
- The attention button shows the terminal's tile, the agent's logo (status_bar.rs:42-46 `face`) → Swift: partial: a status dot (StatusBar.swift:49).
- The counting, the longest-waiting rule, the width classes and the tooltip are a Swift copy of core's `layout::status_bar` (StatusSummary.swift against wormhole-core/src/view/layout/status_bar.rs:129-200): the same rules as read today.
- Checked and found written: counts over every visible window with a click going there, the profile tag, hosting item, Probe items, program and size, words dropping by width, key walk.

##### W04 Columns (100)
- Checked: the core fits the columns (`edgeDrag`, `dragEdge`, `resetEdge`), handle hover delay and active colour, readout beside the pointer with "Hide"/"Close", double-click resets, Esc restores, the panel waiting for room, tabs taking the sidebar's place, Zen hiding the panel without changing what is wanted, slides from the core's column pose with Reduce Motion, widths remembered for new windows in the FFI registry. The three slide variants were read, not traced frame by frame.

##### W05 Multi-window and restore (90)
- Cmd-N opens a window in the profile of the window in front, an odd window too (window/windows.rs:61-64 `new_window_in`: `self.profile`) → Swift: differs: `openNewWindow` plans in the app's `windowsHost`, the shown profile, from the front window's workspace (Wormhole/AppController+WindowOpening.swift:28-31), so Cmd-N in a window of profile B opens a window of profile A.
- Checked and found written: saved windows restored in order with the front one last, frames fitted to the displays that are there, cascade, minimised and zoomed and full-screen state, saving 600 ms after a change and at quit, group to a new window, terminals handed over running, closing the last window quits.

##### W06 Profiles (95)
- Left by the lane and still so: a moved window's hosted desk and the Access/Trust/Imports desks stay on the profile they were made for (TO-VERIFY "Lane window" W06; Wormhole/Settings/ProfileChipBinding.swift `move` follows workspace, notes, project, usage and files only).
- Checked and found written: chip (two profiles or more), Switch Wormhole to, Open a window in, Move this window to (not to its own), New profile, Manage profiles, Settings; switching parks and brings back windows; deleting a profile ends its windows.

##### W07 Zen mode (100)
- Checked: toggle from the menu, the key and the palette; the Zen layer (title row, no status bar, minimap or dividers) over values that are never saved; the panel hidden and its button leaving Zen; the right-panel, status-bar and minimap toggles doing nothing meanwhile; every profile's windows; VoiceOver's announcement; the settings note with Leave Zen mode. The presets are the first run's walk (another lane's rows).

##### W08 Close and quit (95)
- Closing a group names a worktree as shared when a terminal outside the group is in it (features/git/worktree/close.rs, every session of the window) → Swift: partial: "outside" is read from the sidebar's listed rows, so a terminal in a folded group is not seen (Wormhole/AppController+WorktreeClose.swift:23).
- Checked and found written (the second audit's ten gaps): the core's `asksToClose` over every terminal the window holds, an idle agent asks, programs named, one question at a time, the red button's dot, an unavailable host counted, End All ending every live host (`hosts::end_all_live`, ffi hosting/desk.rs:280), SIGTERM quitting saved, x / menu / palette through the question.

##### W09 Window chrome (85)
- The window is named after the selected terminal, kept current outside a draw: what the Window menu, Cmd-` and Mission Control list (window/windows.rs:335-362 `sync_title_soon`, `sync_window_title`) → Swift: missing: the title is set once to "Wormhole (Swift experiment)" plus the window's id and never again (Wormhole/AppController+WindowOpening.swift:53, WindowUI/WorkspaceWindowController.swift:35).
- A terminal window opens and closes with the system's document animation when the Motion feature is on, cut under Reduce Motion, default for a torn-off window (wormhole-core/src/view/motion/window_animation.rs, window/scaling.rs:103-110) → Swift: missing (no `animationBehavior` anywhere in Sources).
- Checked and found written: traffic lights placed, collapse button and panel toggle at fixed spots (moved to the edge in full screen), double-click by the system's preference, minimise, resize in steps of a cell with the strip given up after a drag, `smooth_resize`.

##### W10 Glass, blur, translucency (100)
- Checked: opacity clamped 0.3 to 1, blur only while translucent, the material following the theme, glass by `liquid_glass` and the Mac and not under Reduce Transparency or Increase Contrast (core `glass_on`), glass per card and tab, per-profile look. How it looks cannot be judged from reading.

##### W11 Probe hook effects (90)
- The Probes that are on are read again about once a minute, the backstop for edits behind the app's back (first audit gap 43) → Swift: unreachable: the FFI exports `reload_probes` (wormhole-ffi/src/probes/server.rs:37) and nothing in Swift calls it; a reload happens only on an approval change.
- Checked and found written: status items in the bar with the Probe's name in tooltip and spoken words, notify, the offer chip opening the file in the terminal's folder, the paused chip.

##### W12 A window of another profile (85)
- Cmd-N from such a window opens a window of the shown profile (see W05).
- View > Show Status Bar from such a window changes the app's look, not its own (see W03).
- The Notes preview reads the app-wide type scale (`TextRole.body.font`, PanelUI/Notes/NotePreview.swift:43, 49), so a B window's note text is in A's interface font; the lane's claim that no window module reads the static scales holds elsewhere (`Radius.row` is a constant).
- A terminal of a B window has A's colours for colour queries until its view dresses it, and a B terminal with no view hears A's theme changes (the lane's own "not written", TO-VERIFY wave 5).
- Checked and found written: a look, theme store and terminal look per profile (`ProfileLook`, `dressForProfile`), following that profile's config.json, re-dressing on a move, the 3 pt edge, the status bar's tag, the title-row menu off, panel switches written to the window's own profile.

#### Not covered by any row

- The window's name following the selected terminal (Window menu, Cmd-`, Mission Control): listed under W09 above, named by no row.
- The system open/close animation of terminal windows (Motion feature, Reduce Motion): under W09 above, named by no row.
- The split's opening motion (`window/panes/opening.rs`, 232 lines): S08 names the sidebar's motions only, W01 names no motion.
- The status bar's rolling counts (`window/status_rolls.rs`) and the panel's tab-switch slide (`window/panel_switch.rs`): no row names them.
- `motion_in_force`: the rule that without the Motion feature everything is Classic. No row holds the Swift app to it.
- Files dropped on the title row (`title/mod.rs:409`): W02's name does not list it.
- The tab strip's left/right keys walk `state.sections` rows (WindowUI/Focus/WindowFocus+Keys.swift `walkTerminals`), which skips terminals of folded groups, where GPUI walks the tabs shown: a keyboard row's matter (X04), seen while reading.

#### What reading could not settle

Whether a click on a tab leaves the keys in the right terminal; the system drag of a pane header onto the list; whether SwiftUI plays the per-row poses at all (the lane itself saw rows dropped once); every popover and native menu's survival when another opens; anything about how it looks. None of this code was compiled: `RenameField` is one place that cannot compile as written, and there will be more.

## Settings and Theme Studio

```
id,written
ST01,85
ST02,80
ST09,95
ST03,80
STX1,95
STX2,90
STX3,90
SD01,90
SD02,90
SD03,85
```

Paths: GPUI under `crates/wormhole/src/`, Swift under `macos/swift/Sources/`, bridge under `crates/wormhole-ffi/src/`.
Read only; nothing was built or run. Line numbers are of the tree at `exp/swift-s5` on 2026-11-13.

#### How the settings keys were followed (the 40+ key check)

All 96 `config.json` keys of the settings table (`wormhole-core/src/config/settings_index/table/*.rs`) were searched for in
the Swift sources outside SettingsUI/SettingsModel and in the bridge, and every terminal, appearance and sessions key was
followed to the code that acts on it. Result: **no key is saved and never read.** Where each lands:

- Appearance: `theme`, `theme_light`, `theme_mode`, `theme_primary`, `primary_color`, `branch_color`, `branch_color_custom`,
  `borders`, `settings_text_size`, `ui_font_family`, `ui_font_size`, `settings_translucent`, `studio_translucent`,
  `hosts_translucent`, `opacity`, `blur` → `Wormhole/Settings/LookSettings.swift:19` (`themeKeys`) → `ThemeSync.apply`;
  `opacity`, `blur`, `liquid_glass`, `status_bar`, `smooth_resize`, `icon_animations`, `column_dividers` →
  `LookSettings.swift:36-44`; `title_row` → `HubSettings.applyTitleRow`; `icon_theme` → `SettingsApplier.applyIcons`;
  `hidden_themes` → `SettingsUI/Rows/ThemeCards.swift:48`.
- Fonts: `font_family`, `font_size`, `line_height`, `font_ligatures` → `SettingsApplier.swift:66`; `code_font_family`,
  `code_font_size` → `LookSettings.applyCodeFont`.
- Terminal: `scrollback_lines`, `image_memory_mib` (as `image_memory_bytes()`), `allow_screen_checksum`, `allow_window_ops`,
  `allow_title_report`, `default_shell`, `shells` → `wormhole-ffi/src/launch.rs:48-52` when a terminal starts (GPUI reads
  them at the same moment, `terminal_view/mod.rs:284-288`: no gap); `restore_terminal_text` → `SettingsApplier.swift:70`;
  `minimap`, `split_gaps` → `LookSettings.swift:41,43`; `never_hide_lines` → `SettingsApplier.swift:80` (hook server);
  `option_as_meta`, `bell_sound`, `bell_visual`, `clipboard_read`, `clipboard_write` → `LookSettings.swift:63-80`;
  `open_in` → `HubSettings.applyOpenIn`; `worktree_base`, `worktree_clone_build_folders` → `PanelSettings.swift:31`.
- Sessions and Git: `terminals_in`, `density`, `show_groups_as_cards`, `fade_group_colors`, `show_group_folders`,
  `sidebar_cmd_hints`, `sidebar_changes`, `sidebar_changes_scope`, `sidebar_status_dot`, `sidebar_notes` →
  `LookSettings.swift:49-52` → `SidebarSettingsMaker`; `sidebar_numbers`, `sidebar_changes_plain`, `sidebar_changes_shared` →
  `HubSettings.applySidebarRules`; `sidebar_changes_remember` → `SettingsApplier.swift:95`; `changes_tree`,
  `group_untracked`, `diff_split`, `diff_wrap`, `tab_width`, `size_units` → `PanelSettings.swift:22-29`.
- The rest: `motion.*` → `LookSettings.swift:46-52`; `notify_*`, `dock_bounce` → `NoticeSettingsMaker` and
  `wormhole-ffi/src/notify/records.rs`; `dock_badge`, `menu_bar*` → `AmbientController+Settings.swift`; `hotkey`,
  `hotkey_action`, `jump_waiting_key` → `SettingsApplier.swift:77-78`; `auto_update`, `update_channel` →
  `SettingsApplier.swift:91-94`; `agent_access`, `control_write`, `feature_flags` → `SettingsApplier.swift:80`;
  `show_config_keys` → `SettingsUI/Rows/EntryRowView.swift:61`.

The gaps are therefore in the pages (controls, notes, states), not in dead keys. Two exceptions, where a key can be
stored but the page cannot set it or reads it by another rule, are listed under ST02 (`branch_color_custom`) and ST03
(the `notifications` master switch).

#### ST01 Settings window shell (85)

- Nothing found: a card with the search icon, "Nothing matches …", "Some settings live only in config.json", and the two buttons Open config.json and Clear search (`settings_window/window.rs:565-612`) → Swift: missing: the results page only changes its intro line to "No results" (`SettingsUI/Pages/ResultsPage.swift:33-35`); no card, no buttons.
- Picking a section while a search runs shows that section with the matches still marked, until the search changes (`settings_window/mod.rs:123-124`, `window.rs:234-262`) → Swift: partial: the nav click and "Open <section>" clear the query (`SettingsUI/Nav/SettingsNav.swift:38-42`, `ResultsPage.swift:20-23`), so the page opens without marks; only a single result chosen with Return is marked (`SettingsModel.jump`).
- The nav marks Coding agents while an installed agent's hooks are only partly there (`sections/agents.rs:132`) → Swift: missing: `SettingsNav.badge` (`SettingsNav.swift:52-55`) knows only "Preview" and search counts.
- A row a project sets is drawn as set by the project and its swatch rows/controls are held back (`project_sets(...)` in `sections/appearance.rs:97,128,207`) → Swift: partial: the row's description becomes the project's note (`EntryRowView.swift:41-44`), the control stays live and writes the person's value as before.
- Checked and present: sections and nav groups from core's dump, search with hit counts and dimmed sections, Return opens the first hit, Changed page with core's `changed_rows`, Copy as JSON, Reset All with question, per-row reset arrow and menu (Copy key / Copy as JSON / Reset), config keys under ⌥ or hover, Esc ladder, F6, ⌃⌥⌘F, reveal from the palette (`AppController+Context.swift:37`), window sizes and no full screen, the unreadable-config banners, the preview-flag note, the Zen note.

#### ST02 Pages Profile, Terminal, Appearance, Fonts, Sessions, Terminal list, Git (80)

- Branch colour → Custom: a swatch row of the theme's ANSI colours and "+" for any colour, writing `branch_color_custom`; "Pick a colour below · the theme's is …" until one is picked (`sections/appearance.rs:65-76,113-142`) → Swift: missing: the dump carries `customKey` (`wormhole-ffi/src/settings_dump/controls.rs:95`), `ControlSpec` decodes it, and `EntryControl.swift:20` drops it (`case .segmented(let options, let fallback, _)`). Custom can be chosen and no colour can be picked; `branch_color_custom` is only ever read (`LookSettings.swift:20`).
- Blur off below 95 % opacity: a warning under the row with a "Turn on blur" button (`appearance.rs:195-214`) → Swift: missing (no row has anything under it; `SettingRow.swift` has no `below`).
- Opacity slider switched off, with a note saying why, while Reduce Transparency or Increase Contrast makes every window opaque (`appearance.rs:171-192`) → Swift: missing: `SettingSlider` is always live (`EntryControl.swift:30-33`); the window itself does stay opaque (`DesignSystem/SecondaryWindow.swift:40`).
- Liquid Glass: a "Beta" chip after the label and a note when accessibility holds the glass back (`appearance.rs:79-81,217-228`) → Swift: missing (no chip, no note).
- Primary colour: the description says which is in use ("The theme's · #hex", "Yours · #hex", "Pick…"), and with the theme's own switched off a row of the theme's ANSI colours plus "+" (`appearance.rs:84-110`) → Swift: partial: a switch and the system colour well (`SettingsUI/Rows/PrimaryColorControl.swift`); no description of what is in use, no theme swatches.
- Terminal → Terminal font: "JetBrains Mono 14 pt · line height 1.30 · ligatures on" under the row (`sections/terminal.rs:176-188`) → Swift: missing: `EntryRowView.caption` (`EntryRowView.swift:48-58`) has no case for `terminal-font`; the row shows only the link to Fonts.
- Default shell: each choice says what it runs ("Login shell: /bin/zsh", a profile's command line), and a default that names nothing any more is listed as "Not found: the login shell runs" (`terminal.rs:293-345`) → Swift: partial: names only (`wormhole-ffi/src/settings/host.rs:52-58` drops the command line on purpose); a vanished default is shown as its raw key with no explanation (`OptionPicker.swift:19-20`).
- Open folders in: an app that is gone is listed as "Not installed: the title row opens …" (`terminal.rs:255-267`) → Swift: partial: shown as the bare bundle id, no note (`SettingsUI/Rows/OpenInControl.swift:22`).
- Default terminal: a failed "Make default" says why on the page; a build that cannot be the default says so in the tooltip (`terminal.rs:140-172`) → Swift: partial: the three states and the disabled button are there (`EntryControl.swift:59-66`); the failure goes to `host.problem` (shown), the "only the app bundle" tooltip is missing.
- Fonts → Custom fonts: "2 families found: Berkeley Mono, Monaspace Neon" or "None yet" (`sections/fonts.rs:57-64`) → Swift: missing: `InfoValue` returns an empty string for anything but the version (`SettingsUI/Rows/InfoValue.swift:12`).
- Fonts specimen: tabs Terminal | Code | Interface, each at its slot's family and size, and a facts line (family, size, line height, ligatures, theme) (`fonts.rs:72-188`) → Swift: partial: one scene in the terminal font only, no tabs, no facts line (`SettingsUI/Preview/FontsScene.swift`); a font hovered in an open picker does show.
- Title row parts: the flexible space says what it does, and "Terminal status" says the status bar shows it instead while the bar is on (`sections/title_row.rs:80-84`) → Swift: missing (`SettingsUI/Rows/PartsList.swift:36-42` has no note). Also: where a part switched on goes is re-implemented in Swift (`PartsList.swift:66-78`) instead of core's `Settings::show_title_part`; same rule as read, but a second copy.
- Scrollback and image memory: a value set by hand in config.json is listed with its proper label ("75,000 lines") (`terminal.rs:190-212`) → Swift: partial: shown as the raw number (`OptionPicker.swift:20`).
- Checked and present: every row of the seven pages has a control (dump test), the visible-when rules of the sidebar changes rows, Option as Meta, clipboard rules, bell, Shells list under its row, worktree caption, the profile page (name, letter, colour, start folder, folders, environment, git identity, logins, delete with question, New profile sheet), Sessions and Git rows and their scenes, the three translucent checkboxes, title row drag and VoiceOver move actions, the Motion page and its four scenes, the Zen note.

#### ST09 Font picker (95)

- The Fonts page's specimen follows the slot being picked (GPUI switches the specimen to that slot's size and line height, `sections/fonts.rs:79-90`) → Swift: partial: the scene takes the hovered family and that slot's size but always the terminal's line height and has no slot tabs (`SettingsUI/Preview/FontsScene.swift:14-17`). Everything else of the row was found: search, own faces, Monospace | All, bundled/custom/installed captions from core's `font_choices`, recents, the inherit row, the "Bundled" tag, the monospace tag, counts and loading lines, arrows / Page / Return / Esc-clears-first, VoiceOver announcements, the picker's own preview with coverage.

#### ST03 Pages Notifications, Agents, Menu bar, Import, Updates, Advanced, Preview (80)

- Notifications' kinds are read and written through the master switch: with `notifications` off every kind reads off, and turning one on turns the master on (`sections/notifications.rs:101-110`, core `Settings::change_notify_kind` / `notify_kind_on`, `wormhole-core/src/config/settings/mod.rs:505-520`) → Swift: missing in Settings: the kinds are plain switches over their own keys (`settings_dump/controls.rs:36` → `EntryControl.swift:17`) and a write is `settings_index::set`, which does not touch `notifications` (`write.rs:17-26`). With a config where the master is off (the GPUI app writes that when the last kind is turned off) the Swift page shows kinds as on, nothing is delivered (`notify/records.rs:165` carries the master), and no switch on the page turns it back on. Only the terminal welcome uses the rule (`hook_server/welcome/ops.rs:128-131`).
- Notification permission row: what macOS allows, state by state (asking, not asked yet, allowed with or without Time Sensitive, off in System Settings, outside the app bundle), with "Allow notifications" when it was never asked and "Open System Settings" otherwise (`notifications.rs:112-184`) → Swift: partial: one static "Open System Settings" button, no state, no way to ask from the page (`SettingsUI/Panels/PanelControl.swift:32`).
- Test row: says what came of the test (waiting, sent, denied, unanswered, refused with the error) and asks macOS first when it was never asked (`notifications.rs:186-243`) → Swift: partial: a "Send test" button that posts and says nothing (`PanelControl.swift:33`, `PlatformModel/Notify/NoticeCoordinator.swift:62`).
- "A terminal needs input": a "Time Sensitive" chip and a description that says whether this build can deliver it so (`notifications.rs:39-42,245-254`) → Swift: missing.
- Agent access → approved programs: a tick per capability (Read, Type, Run, Open), changed at once and kept with the approval (`sections/agent_access.rs:59-98`) → Swift: missing: the capabilities are a line of text (`SettingsUI/Panels/ApprovalList.swift:31-34`); no setter exists in the bridge or the host. Revoke is there.
- Coding agents: each profile's login of an agent as its own line (its folder, the hooks' state there, Install / Repair / Remove for that location) (`sections/agents.rs:72-88,360-430`) → Swift: missing: `AgentRecord` and `wormhole-ffi/src/settings/agents.rs` carry no logins; hooks can only be written to the default location.
- Coding agents: "Looking for agents…" until the first read (`agents.rs:167-176`) → Swift: missing: the list starts empty and so shows "No coding agent on this Mac's PATH" until the background read lands (`AgentList.swift:21-23`, `RustSettingsHost.swift:150-155`).
- Advanced → Reset tips: the tooltip says whether a tip was closed for good (`sections/advanced.rs:56-70`) → Swift: partial: disabled when none, no tooltip (`EntryControl.swift:51-52`).
- Checked and present: Updates in full (status by phase, Check now, Install, Skip, Restart now / Don't restart with the question, download meter, security line with advisories, release notes, channel note, the privacy block with core's user agent), Import (cards, review rows with ticks, Scan again, Import selected), Menu bar, Hotkey with recorder, conflicts and the registration problem, Advanced's folders, Show config keys, Reset with question, Developer Tools, the command line tool row with its PATH verdict and Copy line, How status works, Rescan PATH.

#### STX1 Extensions page (95)

- While a Probe is being read for its sheet, turned off or approved, the page holds that row's work (`pending`, `sections/extensions.rs:58`) → Swift: partial: the sheet is locked while busy (`ProbesModel.Open.busy`), the list's switch is not: flipping a second switch while the first read runs starts a second one (`SettingsModel/ProbesModel.swift:31-41`). Everything else was found: list with five states and the unsealed word, identity and description through core's reveal, too-many and key-problem banners, the empty card with Show folder, Forget, Approve again, the approval sheet (permissions with switches, Touch ID tag, program class notes and command lines, combinations, "since" block, fingerprint line, click or Touch ID, cancel/failed leave it up), refresh on open, on the button and when the window comes forward, tidying after a full read.

#### STX2 Project folders page (90)

- The selected project's files and trust are read again about once a second while the page shows (`sections/projects.rs:44-46`) → Swift: missing: read on open and after the page's own buttons only (`SettingsModel/ProjectsModel.swift:20-25`, `ProjectsPanel.swift:36`); a project trusted or changed from the Project tab, or edited on disk, leaves the page stale.
- Return in the project search takes the first match (`projects.rs:120-127`) → Swift: missing: the search is a separate text field that only narrows the pop-up (`SettingsUI/Projects/ProjectsPanel.swift:22-24`).
- Checked and present: defaults with switches, trusted list with Settings and Revoke, the no-approval banner, one project's trust badge and detail, commands, Trust with Touch ID / Revoke trust, Default | On | Off per feature, problems, Open in a new terminal, Show in Finder, the missing-folder banner.

#### STX3 Agents' skill and MCP rows, How status works, Feature flags, unlock key (90)

- The flag "Settings components" opens the Components page (`config/features.rs:83-86`, `SectionId::Components`) → Swift: unreachable: the dump leaves the section out (`wormhole-ffi/src/settings_dump/mod.rs:53-55`), so the switch is listed on Feature flags and does nothing.
- Skill rows: "Looking…" and the agents' marks as on the hooks list (`sections/agent_skills.rs:81-100`) → Swift: partial: a status dot instead of the agent's tile, and the empty note before the first read (`SkillList.swift:42-44`).
- Checked and present: skill states (current, outdated, foreign with its tooltip, covered, not installed) and their buttons, the per-row failure, the `claude mcp add` line with Copy, How status works, the flags list with changed dot and reset, nav gating by `shownSections`, ⌃⌥⌘F, the page leaving when locked.

#### SD01 Theme Studio: token editor, list, draft, save, export, versions, contrast (90)

- The Studio reads and writes the themes folder of the profile Wormhole is in (`studio/themes.rs:134` `active_profile::paths().themes_dir()`) → Swift: partial: `StudioLauncher` is made once with the data folder and uses `<data folder>/themes` (`Wormhole/Platform/StudioLauncher.swift:26`, `AppController.swift:80`), which is the Default profile's. The live theme reads the active profile's folder (`AppController+Look.swift:28`). In any other profile the Studio lists, saves and trashes the Default profile's themes, and a saved theme's pick names a file the window's loader does not find.
- Expression field completions offer token names and the functions (`mix(`, `readable(` …) from core's `completions` (`studio/completion.rs:1-6`) → Swift: partial: token names only (`StudioModel/Document/StudioDocument+List.swift:77-79`), matched by Swift's own rule (`StudioModel/Tokens/NameCompletion.swift`), not core's.
- Low-contrast fixes come from core: they follow tokens that merely name a failing one and spot cycles → Swift: re-implemented and simpler, as its own header says (`StudioModel/Contrast/ContrastFix.swift:1-4`): the pairs are a copied table (`ContrastPair.swift`), the maths is Swift's, the fix is always `readable(current, backgrounds, min)` on the text token.
- Metric editor: a preview of what the metric shapes and how many things use it (`studio/token_editor.rs:274-355`) → Swift: missing: stepper, slider, Follow and a help line only (`StudioUI/Editor/MetricEditor.swift`; its header comment promises the preview).
- Reading a theme file and keeping saved versions go through core (`storage::files` caps and link rules, `theme::saved_versions`) → Swift: re-implemented: `DiskThemeFolder.readCapped` and `VersionPolicy` are Swift copies of the cap, the link refusal, the version file names and the keep policy (`StudioModel/Document/DiskThemeFolder.swift:25`, `VersionPolicy.swift`). Same numbers as read; two places to keep in step.
- The appearance chip ("both" / "light") sits on each token's row in a pair and a click there moves the value (`studio/origin_tag.rs:48-52`) → Swift: partial: the chip is in the editor's header for the selected token only (`StudioUI/Editor/EditorHeader.swift:33-50`).
- Checked and present: opening on the theme in use (own file in place, built-in as "Untitled (from X)", a trusted project's theme), draft resolved by core on every edit and previewed in every window, refused values kept out with the reason and "Did you mean", token list with filter, All / Yours / Base / Low contrast, folding bands, origin words and tips, colour picker with one undo step per drag, rule, Used by, contrast pairs, the low-contrast card with fix and Show, undo/redo with named steps in the Edit menu and title row, Revert to Saved, Save (⌘S), first-save name, name taken → Replace, name is a version, changed on disk → Overwrite, newer file never written, conversion of the older format, picks following a save and a rename, Duplicate (⇧⌘S), Export JSON with the remembered folder, Copy JSON, Use in this project, the notice with Reveal, Use / Use with Match system, the theme menu (your themes, project theme, New, New From with families, Rename, Move to Trash with its sheet, Add Light/Dark Appearance, Update from with its sheet and three states, Versions with preview and Restore), Light | Dark, the newer and Preview chips, the unsaved question on close and on opening another theme.

#### SD02 Component gallery (90)

- Status change caught part-way per motion variant and counts caught half turned (`studio/gallery/tiles.rs`) → Swift: partial: played on a press instead; the lane says the views cannot be held at a time (`StudioUI/Gallery/TilesSpecimen.swift`).
- "Used by" and the outline for tooltip and dialog (`TOKEN_USES`) → Swift: missing by construction: both are the system's (an owner decision), so `tooltip.*` is outlined only on the readouts and no specimen draws a themed dialog; the gallery says so in "System controls".
- The uses table is core's `TOKEN_USES`, and a test holds every use to a mark in the gallery (`studio/gallery/mod.rs:370-392`) → Swift: re-implemented: `GalleryUses.swift` is generated from the Swift views' own `.marked(id)` (`macos/scripts/gen-gallery-uses.py`), so "Used by" names what the Swift views draw, not what core lists; the two can differ per token.
- A page that builds only the pieces near the viewport (`studio/gallery/layout.rs`) → Swift: not there (every specimen is built in a plain `VStack`, `GalleryView.swift:25-34`); a speed matter, named because the GPUI gallery measured 24 ms a wheel tick without it.
- Checked and present: 32 specimens against GPUI's 21 (each GPUI specimen has a Swift one, several split), the area filter narrowing the gallery and the token list, the "n uses of token" line, the dashed outline through `tokenMarks`, swatches that select their token (shape, terminal, code, as in GPUI: shape, terminal, chrome), the window's own specimens handed in by the app (`WindowSpecimens.view`), specimens not answering clicks.

#### SD03 Theme library: list, load, choose, persist (85)

- The mode decides the shelves: one shelf under Dark or Light, two under Match system (`settings_window/sections/theme_picker/mod.rs:304-311`) → Swift: partial: both shelves always (`SettingsUI/Rows/ThemeCards.swift:42`).
- A theme made with a newer Wormhole carries a warning mark on its card ("n values were skipped") (`theme_picker/card.rs:204-214`) → Swift: missing on the card (the Studio's title row has it).
- A trusted project's theme in effect: a notice over the cards naming the project, with "Project settings" (`sections/appearance.rs:300-320`) → Swift: missing: the cards show the person's picks as chosen while the window shows the project's theme.
- Hiding a theme is core's rule over the card's value (`Settings::set_theme_hidden`, `config/theme_setting.rs:82-94`) → Swift: re-implemented over the theme's display name (`ThemeCards.swift:63-68`): for a theme of the person's own the value is its id or file, so a hide written by one app need not match in the other.
- "No themes match the filter" and the filter's count in its placeholder (`theme_picker/mod.rs:105-111,522`) → Swift: missing (empty shelves, plain placeholder).
- Arrow keys walk the cards of a shelf (`theme_picker/mod.rs:250-281`) → Swift: missing: no key handling in `ThemeCards.swift` / `ThemeCard.swift` (Tab reaches them as buttons at best).
- Hidden themes list says Dark or Light beside each name (`theme_picker/mod.rs:556-562`) → Swift: missing.
- Checked and present: bundled themes and the person's own on the shelves, a card per version named by family or version, the pick leading, Show all / Show fewer, filter by name or family, pick written by core's rule through the host, Hide theme, Hidden themes with Show, Import theme… with core's check and the failure line, Move to Trash… with the plan's question, the Theme Studio button, user theme files followed when they change (`ThemeStore.refreshIfThemeFilesChanged`).

#### Not covered by any row

- Settings → Components (`SectionId::Components`, `settings_window/sections/components/`, behind the `components` flag): the page of every settings control with sample state. No row names it; Swift has no page and still lists its flag.
- The devctl commands of the two windows (`settings_window/devctl.rs`, `studio/devctl.rs`: `settings-section`, `settings-search`, `settings-select`, `settings-record`, `studio-set`, `studio-filter`, `studio-scroll`, `import-open`, `import-apply` …). The Swift app has its own control folder (`AppController+ControlSettings.swift`, `+ControlStudio.swift`); whether each GPUI command has a counterpart was not compared and no row of mine asks for it.
- Settings opened from other places with a section named (the status item, the palette's sections, links such as "Project settings" on a theme notice): `SettingsWindowController.show(section:)` exists; the callers were not all followed.

## Secondary windows, overlays, platform

Read from the tree at the branch `exp/swift-s5`; nothing was built or run. Paths: GPUI under `crates/wormhole/src/`
(core under `crates/wormhole-core/src/`), Swift under `macos/swift/Sources/`, bridge under `crates/wormhole-ffi/src/`.

```
id,written
A01,80
A02,95
A03,90
A04,90
A05,85
A06,95
A07,90
O01,90
O02,90
O03,85
O04,85
O05,90
PL01,80
PL02,85
PL03,75
PL04,80
PL05,95
PL06,95
PL07,100
PL09,100
```

Weighted by the ledger's weights (18,092 for these 20 rows): about 86 % written and reachable, against 100 % in `built.csv`.

#### The keymap, action by action (menus, palette, sheet, keys)

Method: every `row(...)` of `keys/keymap/table.rs` against `Wormhole/Actions/ActionRouter*.swift`, `PlatformUI/Menus/StandardActions.swift`,
the views named in `ActionRouter.viewed`, and how a key reaches the router at all.

- The menu bar's layout, titles, keys and dynamic items are core's (`keymap::menus::menus` through `PlatformBridge/CoreMenus.swift`), the
  palette's actions and the sheet's rows are core's too (`keymap/lists.rs`). So where an item is listed is the same as GPUI **given the same
  `MenuState`**. The state is where they differ (below).
- Every action of the table has a handler or an AppKit selector, except `help` (the system's Help search in both apps: not a gap). No action
  with an empty handler was found. Handlers that do less than GPUI's are listed under PL01 and PL02.
- **Keys: the Swift app binds a key only as the key equivalent of a menu item** (`PlatformUI/Menus/NSMenuFactory.swift:66-75`, one key per item),
  plus the terminal's own ⌘ keys (`TerminalMetal/Pointer/TerminalShortcuts.swift`) and a project action's key. GPUI binds every key of every live
  entry (`actions/mod.rs` `bindings`, `app/handlers.rs`). The only code that maps "any key of the table" to an action is the test driver
  (`Wormhole/AppController+ControlKeys.swift:118-128` `menuAction`), which a person never reaches. So these keys do nothing in the real app:
  - `Group(1…9)`, ⌃⌥1 to ⌃⌥9: in no menu (the Window menu lists only terminals) → no key. Handler exists (`ActionRouter+Terminals.swift:32`), reachable only from the test driver.
  - `PlayIntroEmerge … PlayIntroVoyage` ⌃⌥⌘1 to 7 and `PlayWelcomeCursor/Portal/Logo` ⌃⌥⌘8, 9, 0: in no menu → no key (palette rows work).
  - Second keys: `NewTerminal` ⇧⌘N, `NextTerminal` ⇧⌘], `PreviousTerminal` ⇧⌘[, `Bigger` ⌘= (the menu item holds only the first key; ⌘= works only if AppKit maps it to ⌘+ by itself).
- Menu state (`window/menu_state.rs`, `window/feature_host.rs:309`, `features/usage/def.rs:46`, `features/project/def.rs:94`) against
  `WindowUI/Menu/MenuStateMaker.swift`:
  - Actions of a feature that is switched off (`feature("history" | "notes" | "project" | "files")`: Show History, Show Notes, New Note, Stash Prompt, Show Project, Show Files) are hidden in GPUI's menus, palette and sheet (`config/feature_table.rs:167` `hidden_actions`). Swift hides by feature **flag** only (`MenuStateMaker.swift:83`); `KeymapEntry.feature` is read by nothing. They stay listed and enabled in all three places.
  - Show Usage is hidden while no agent is on screen (`features/usage/def.rs:46`) → Swift: always listed.
  - Show Project is greyed without a folder or with project folders off (`features/project/def.rs:101`) → Swift: never greyed.
  - Undo/Redo titles of a window with its own history ("Undo Change “primary”", `menu_state.rs:36-52`, the Theme Studio) → Swift: `MenuState.undo/redo` are never set (no writer found).
  - The same three differences reach the palette (`Wormhole/Platform/PaletteSourcing.swift:77` `actionFacts` reads this state) and the sheet (`hiddenActions`).

#### A01 Onboarding window, intro, welcome (80)

- The agents step shows the project folder's trust card (Trust with Touch ID…, Later, "Waiting for approval…") for the folder picked in step 1, else the front terminal's project (`onboarding/mod.rs:201-212` `trust_root`, `window/onboarding_host.rs:99-104`, `agents_step.rs:159-230`) → Swift: unreachable in the real app: `AuxBackends.onboarding` builds `FFIOnboardingBackend` without `project:` (`Wormhole/Aux/AuxBackends.swift:150-163`), so `trustDesk` and `trustCard` are nil (`AuxBridge/OnboardingBridge.swift:46-47`) and `trust(_:)` returns at its guard. The card and view exist (`AuxWindows/Onboarding/TrustCardView.swift`) but only the sample backend ever supplies one; the card is also fixed at init, not after the folder pick.
- The folder step offers the projects opened recently under "Recents" (`window/onboarding_host.rs:36-44` `recent`, `onboarding/mod.rs:331`) → Swift: missing: `recents:` is not passed (default `{ [] }`, `OnboardingBridge.swift:30`), so `places()` never has a recent one.
- After Start on the primer of a first run the sidebar keeps the "No coding agents yet" note (`onboarding/mod.rs:433` `agents_note = Shown`) → Swift: partial: the note exists (`SidebarUI/Tip/NoAgentsTip.swift`) but only the terminal welcome sets it (`Wormhole/Hooks/HookHost+Welcome.swift:43`); the intro window's finish (`OnboardingWindow.swift:22-25`, `AuxBackends.swift:161`) never sets `hub.agentsTip`.
- A first run that picked a folder replaces the untouched first terminal by one in the folder, only if it is still the window's only one and not already there (`window/onboarding_host.rs:136-145`) → Swift: partial: a new terminal is opened in the folder (`AuxBackends.swift:161-163`), the first one is never closed and "already there" is not checked.
- A file dropped on the folder step picks the folder it is in (`onboarding/mod.rs:366-378`) → Swift: partial: only a dropped directory is taken (`AuxWindows/Onboarding/FolderStep.swift:42-44`).
- The primer's words and its "Jot a note" key follow whether Notes is on in the window's profile (`agents_step.rs:248`, `Facts::notes_on`) → Swift: missing (`AuxWindows/Onboarding/PrimerStep.swift` has no such fact).
- The steps' flow is core's `onboarding::Onboarding` → Swift: re-implemented (`AuxModel/Onboarding/OnboardingFlow.swift`), same paths as far as read; the intro is a window of its own, not the main window's page (looks, not counted).
- Help ▸ Welcome while the page is already up replays it (`onboarding_host.rs:62-66`) → Swift: opens a second `OnboardingWindow` over the same key (`Wormhole/Aux/AuxController.swift:96-100`); the earlier session's state is replaced, not replayed.
- Checked and found: welcome page, the three steps and the primer, keys (←, →, ⏎, Esc, R, ⌘O, digits of the walk), replay, the Look step's theme, colour, font, ligatures, import offer and "How much to show" walk through the FFI, hooks install per agent, the Metal intro's seven styles and three hand-overs, the terminal welcome program with its token, demo band, panel, permission and end (`ffi/hook_server/welcome`, `HookHost+Welcome.swift`).

#### A02 About and Licenses (95)

- About shows the copyright of `packaging/app.env` (`about/mod.rs:42`) → Swift: partial: `AppInfo.from(bundle:)` reads `NSHumanReadableCopyright` (`AuxModel/Licenses/AppInfo.swift:29`), which the Swift app's linked `Wormhole/Info.plist` does not have (the packaging plist does), so the panel has no copyright line.
- Licenses' search is core's `Licenses::search` → Swift: `LicenseList.search` (a port; same behaviour as far as read). List, selection kept while listed, detail, repository link, code-font texts, counts, ↑↓: present.

#### A03 Update (90)

- Check for Updates… (menu, status item) opens Settings ▸ Updates and starts a manual check (`app/handlers.rs:63-68`, `status_item/mod.rs:321-324`) → Swift: partial: opens a separate Updates window and starts no check (`Wormhole/Actions/ActionRouter+App.swift:26`, `AuxWindows/Update/UpdateView.swift:30`: only follows the phase); the person must press Check now.
- A development build says "This is a development build; it doesn't update itself." after a check (`update/mod.rs:307-311`) → Swift: says there is no feed (`AuxBackends.swift:117`): same meaning, other words.
- Checked and found in the bridge and the follower: baked feed, key, team and fallback for a release build, channel from Settings and the switch's call-off or "stays installed" note, the schedule (launch, six hours), quiet automatic checks, skip (never a security release), auto-install and not twice a failed version, download progress, the restart question with Restart When Idle and its 30 s idle rule, the ready chip that follows the front window and stays closed per version, the replaced-bundle watch (`ffi/aux_backends/updater.rs`, `Wormhole/Aux/UpdateFollower.swift`, `AuxBridge/UpdateBridge.swift`).

#### A04 Session Hosts window (90)

- Which terminals hold a host is read from every session of every window (`hosts_window/gather.rs:35-44`, `host_attachments`) → Swift: partial: `hostAttachments()` walks `state.sections.flatMap(\.rows)` only (`Wormhole/Aux/AuxBackends.swift:179`), so a hosted terminal in a folded group or filtered out of the list shows as detached.
- An unavailable host's card carries the error's detail → Swift: `unavailableDetail: nil` always (`AuxBackends.swift:185`).
- Checked and found: header with tally, folded cards, copy one and copy all with "Copied", End with its question, Forget, the folder watch with a slow poll behind it.

#### A05 Agent access sheet (85)

- The question about a terminal names its row title and folder; the program's facts end with "Runs in <terminal>" (`agent_access/sheet.rs` `terminal_display`, `facts_block`, `terminal_of`) → Swift: unreachable: the bridge takes both from a `Resolver` (`AuxBridge/AccessBridge.swift:13-23`, 69, 73) and the app builds the backend with the default one, whose closures return nil (`Wormhole/Aux/AuxBackends.swift:66`, `AuxBridge/AuxBridge.swift:27`). The terminal question shows the terminal's number and no folder; "Runs in" never shows.
- When the mode changes while questions wait: to Off (or the Automation flag off) every waiting question is refused with `access_off` and the streams started under approvals end; to Allowed while open every waiting question is let in at once (`agent_access/ask.rs:414-470` `settle_mode`, `refuse_waiting_questions`, `allow_waiting_questions`, `end_streams_under_approvals`) → Swift/bridge: missing: `Core::settings_changed` (`ffi/hook_server/core.rs:257-266`) only drops streams that "Allowed while open" let read; waiting questions stay up until answered or the 60 s timeout, and streams under approvals survive the switch to Off.
- "For this session" is offered only when the program has a process to tie it to (`sheet.rs` `buttons`: `resolved.process.is_some()`; Scripts has none) → Swift: differs: offered when the request has a program (`AuxModel/Access/AccessDisplay.swift:55`), which Scripts has ("Asked by"); the record carries no process fact, so Scripts gets a button GPUI does not show (the bridge then treats it as until quit).
- Checked and found in the bridge: the gate (mode, need, own reads, identity, cooling, scope of terminals, opened terminals owned), session, until-quit and stored approvals, the sweep of ended programs, warnings (unusual signer, unsigned, ad hoc, changed since allowed, root), one sheet at a time on a window of the question's profile, Esc and Return as Don't Allow, withdraw when the window goes, Settings' list and revoke.

#### A06 Trust window (95)

- Opened for a project that is no longer open it says so ("This project is no longer open in a terminal.", `features/project/trust_window.rs:98`) → Swift: with no project root it shows the sample backend's data (`AuxBackends.swift:53-62` falls to `SampleTrustBackend()`); reachable only if `onShowProjectCommands` fires without a project.
- Checked and found: status badge and summary per state, the lines with what runs set apart, Trust with Touch ID / password, "Waiting for approval…", the Project tab following an approval.

#### A07 Agent access: held-back mode, timeout, profile (90)

- A widened mode is noticed on the app's tick, so the keep-it question appears soon after the edit (`ask.rs:414-430`) → bridge: noticed when a request reads the settings (`ffi/hook_server/core.rs:46-65`); the question appears with the first request. The old mode stays in force either way.
- The mode's effect on waiting questions and streams: see A05 (`settle_mode` not ported).
- Checked and found: `modes.rs` reconcile (widening from outside held back, narrowing and Settings' own edits at once), Keep / Don't Keep writing the old mode back, 60 s timeouts for both kinds of question (`access/mod.rs:94-125`), the sheet on a window of the request's profile (`AuxController.swift:79-82`).

#### O01 Command palette (90)

- Commits are the log of HEAD of the palette's terminal, read when it opens, and only with the History feature on (`command_palette/mod.rs:304-324`) → Swift: differs: the History tab's own model (`PaletteSourcing.swift:22-26`), so they follow whatever rev and path filter History shows, and are listed with History off.
- The hidden and greyed actions: the three menu-state differences above (feature-off actions, Show Usage, Show Project) show in the Actions scope.
- Recent folders are the profile's list in its database (`app_state/recent_projects.rs`) → Swift: one app-wide `recent-projects.json` (`Wormhole/Menus/RecentProjects.swift:16`), not per profile and not the GPUI app's list.
- Checked and found: scopes and prefixes, Tab, Esc ladder, ⌫ in an empty narrowed field, ↩, ⌘↩ (terminal, folder, commit), ⌘C on a setting with its chip, ⌘⌫ asking before closing, terminals of every window with folded groups, project actions, shells, profile rows, recents.

#### O02 Shortcut sheet (90)

- Rows of actions whose feature is off are left out (`hidden`) → Swift: listed (the sheet's `hidden` comes from the same menu state, which never hides by feature).
- Checked and found: core's sections and filter, the project's keys block, footer with the hotkey, Esc clears then closes whatever has the keyboard, not under the first-run page, the recorded jump key.

#### O03 Folder picker (85)

- The picker's model and keys are core's `view/picker` (`model.rs`, `keys.rs`, `ghost.rs`) → Swift: re-implemented (`PlatformModel/Picker/PickerModel.swift`, `PickerKeys.swift`, `PickerMerge.swift`); the rules read the same (↑↓, page keys, ↩, ⇥, → at the end, ← at the start, ⌘⌫ forget, ghost) but nothing ties them to core's.
- Recent folders are kept per profile (`app_state::recent_folders`, the window's profile) → Swift: one list in the default store (`ffi/folder_picker/mod.rs`, `Wormhole/Platform/FolderPicking.swift:31`).
- Every terminal's folder change is a visit → Swift: partial: `follow()` watches `app.hub.state.terminals` (`FolderPicking.swift:96`): the front window's listed rows only; other windows and folded groups are not recorded.
- Checked and found: the flag gate, only over the terminal that has the keys, the cursor anchor, paste at the cursor, closing when another terminal is selected, core's providers and badges, forget.

#### O04 Context menus (85)

Checked by search that the second audit's missing pieces now have code and a caller (native group menu with the dot row and ⌃↩, switcher card menu, "New terminal with" on +, title strip ⌃↩, commit details menu, File History gate, greyed splits, `allowed` groups, Settings row menu); not re-compared item by item.
- Group head ▸ Other colour… opens the system colour picker (`sidebar/group_popover.rs:220-229`) → Swift: now `GroupColourPanel` (`Wormhole/Menus/GroupMenuKeyboard.swift:74`); TO-VERIFY still calls it unbuilt: the two disagree, the code is there.
- The native-menu catcher takes right-click and ⌃-click only when AppKit's current event is one (`DesignSystem/NativeContextMenu.swift`): whether a plain click, drag and hover still reach the views under it cannot be judged from reading.
- The pane header's keys are written in the view, not read from the keymap (second audit; a recorded key would not show).

#### O05 Notice chip (90)

- The chip's rules (which notice wins, timing, buttons, placement, reasons) are core's `view/notice` (`machine.rs`, `place.rs`, `buttons.rs`, `reason.rs`) → Swift: re-implemented (`TerminalUI/Notice/NoticeRules.swift`, `NoticeTiming.swift`, `Wormhole/Notices/NoticeReason.swift`).
- `HistoryKeptByHost`: the poster is wired (`WindowUI/Overlay/TerminalOverlay+Clipboard.swift:23`), the lane says the host-side seam that fires it is not built.
- Checked and found a poster for every kind of `Copied`, `Driven`, `Failed` and `Warning` of core's list (host fallback, move to Applications, data folder, update ready, probe offer and paused included).

#### PL01 Menu bar (80)

- The key gaps and the menu-state gaps of the keymap section above (Group 1 to 9, the intro and welcome trial keys, second keys; feature-off actions, Show Usage, Show Project, Undo/Redo titles).
- View ▸ Show / Hide Status Bar (⌘/) writes the `status_bar` setting (`app/handlers.rs:125-129`) → Swift: partial: flips the window look in memory only (`Wormhole/Actions/ActionRouter+View.swift:24`, `WindowUI/WindowLook.swift:27-30`); nothing writes the setting, so it is back at the next launch and Settings' switch does not follow (Show / Hide Minimap, next to it, does write its setting).
- Go to Waiting / Go to Finished Terminal act across every window: the one that waited longest (or finished last), then the next on each press, and bring its window forward (`window/ambient.rs:223-249`, core `notify::sections`, `next_waiting`) → Swift: differs: the next one after the selected terminal in the front window's own list order (`WindowUI/Hub/WindowHub+Menu.swift:28-34`); a terminal waiting in another window is never reached.
- Open Recent's list: app-wide file, not the profile's (see O01).

#### PL02 Hotkeys (85)

- The jump keys' behaviour: see PL01 (front window only, list order).
- ⇧⌘N, ⇧⌘], ⇧⌘[ and the ⌃⌥ digit keys: see the keymap section.
- Checked and found: the global hotkey registered from the setting, paused while recording, its error under the row, quick panel or show/hide, the recorder's verdicts from core, the recorded jump key rebinding the menu and the sheet.

#### PL03 Notifications (75)

- "Needs input" goes out Time Sensitive when the build is entitled and the person allows it (`notifications/mod.rs:216-219`) → Swift: unreachable: `coordinator.look` and `sendTest` are always called with the default `timeSensitive: false` (`Wormhole/Notices/NoticeFeed.swift:52`, `PlatformModel/Notify/NoticeCoordinator.swift:41, 62`); `SystemNotifier.deliversTimeSensitive` has no caller.
- A terminal's banner is taken down when the person looks at it (`Event::Seen`: selected, in the key window, the app active) → Swift: differs: after every look the front window's selected terminal is dismissed unconditionally (`NoticeFeed.swift:53`), so a banner for the selected terminal posted while Wormhole is in the background is retracted in the same pass.
- An event is announced by the settings of the profile its terminal is in (`notifications/mod.rs:191-196`) → Swift: the shown profile's (`app.settingsNotice`).
- Send test never ends silent: asks macOS first when not asked, says when notifications are off, shows the centre's refusal (`notifications/test_banner.rs`) → Swift: missing: the button posts and shows nothing back (`Wormhole/Settings/SettingsPlatform.swift:71`; no answer state in SettingsUI).
- A dev build without a bundle posts through the platform's script → Swift: the log only (a bare executable; dev only).
- Checked and found: core's rules through the FFI (kinds, bursts, quiet when in front), a thread per terminal, sound, Open / Reply… / Show Changes and what each does, the Dock bounce, VoiceOver's words, a Probe's banner.

#### PL04 Status item and menu (80)

- Every command but the hotkey and Quit brings Wormhole and its front window forward first (`status_item/mod.rs:271-274`) → Swift: partial: New Terminal, Jump to Next Waiting and Run action never activate the app, and a terminal's row calls `makeKeyAndOrderFront` without `NSApp.activate` (`Wormhole/Ambient/AppController+Ambient.swift:10-23`, `AppController+Windows.swift:142-146`): chosen from another app, the change happens behind it.
- Show Wormhole and Jump to Next Waiting show their keys (`menubar/mod.rs` `hotkey`, `jump_key`) → Swift: `pushStatus` never sets `snapshot.hotkey` / `jumpKey` (`Wormhole/Ambient/AmbientController.swift:62-69`).
- Projects ▸ lists the actions of every open project of the shown profile (`menubar/model/mod.rs:198-219`) → Swift: only the front terminal's project (`AmbientController.swift:74-77`).
- Jump to Next Waiting: front window only (see PL01). Check for Updates…: no check started (see A03).

#### PL05 Quick panel (95)

- Its Do section and Reply reach the same paths as the status item; the project's actions are the front terminal's only (see PL04). Rows, sections, filter, ↑↓, ↩, ⌘↩ Reply in place, Esc ladder, closing on losing the keyboard, the pointer's screen, second press shows the app: present.

#### PL06 Dock (95)

- Dock ▸ New Terminal brings the app forward (`status_item/mod.rs:237`, `run_command`) → Swift: `hub.send(.newTerminal)` without activating (`AppController+Ambient.swift:11`). Badge modes, menu sections, one informational bounce: present.

#### Rows at 100

- PL07: the document types and usage strings of `Wormhole/Info.plist` are identical to `packaging/Info.plist`'s; `application(_:open:)` keeps requests until the windows exist and hands them to core (`Wormhole/SystemOpen.swift`, `ffi/hook_server/control/open.rs`); Settings makes and reads the default-terminal role by core's types (`Wormhole/Platform/DefaultTerminal.swift`, `SettingsPlatform.swift:26, 76`).
- PL09: SIGTERM is caught and quits unattended with the windows saved (`AppController+WindowEnding.swift:48-54`, called from `AppController.swift:137`); a second launch's Activate opens a window when none is visible (`SystemOpen.swift` `activateFromSecondLaunch`, from `HookHost.swift:69`).

#### Not covered by any row

- Which keys are bound at all (the "every key of every live entry" rule of `actions/mod.rs`): PL01 counts menu items, PL02 names the hotkey and jump keys; the unbound second keys and menu-less keys above belong to no row's wording.
- The recent projects list as per-profile data (`app_state/recent_projects.rs`: Open Recent, the status item, the welcome's places, Settings' project list).
- Show Logs in Finder, the Services submenu, Wormhole Help's system search: present in Swift, named by no row.
- The Swift bundle's Info.plist beyond PL07 (copyright, minimum system version, icon name, category): only document types and usage strings are a row.

## Crosscut (X01 to X15)

```
id,written
X01,90
X02,95
X03,85
X04,90
X05,90
X06,95
X07,85
X08,90
X09,90
X10,85
X11,95
X12,75
X13,95
X14,90
X15,90
```

Weighted by the ledger's weights this is about 87 % of the area (X01 and X12 carry 69 % of it).

How deep each row was read (so the numbers are not taken for more than they are): the three full checks
(X12 commands, X08 verbs, X07 words) were done whole. X13, X14, X15, X08, X04 were followed from the bridge to
the call site. X01, X02, X03, X05, X09, X10, X11 were checked against the first audit's gap list plus spot
checks of the GPUI components; X06 was not re-read beyond the mode switch. No row is at 100: for none of them
did I look everywhere. Paths: GPUI relative to `crates/wormhole/src/`, Swift to `macos/swift/Sources/`, bridge to
`crates/wormhole-ffi/src/`.

#### X01 Design language (90)

- The status bar's counts turn over digit by digit when they change (`window/status_rolls.rs:1-25`, `style/rolling_number.rs`) → Swift: missing in the bar: `countLabel` is a plain `Text("\(count.number)")` (`WindowUI/StatusBar.swift:125-134`); the roll exists only in the sidebar's second line (`SidebarUI/Row/SecondLineView.swift`).
- One tile and one field component shared by every screen (`style/tile/`, `style/text_field.rs`) → Swift: partial: two tiles (`SidebarUI` StatusTile, `PanesUI` TerminalTile) and two fields (PanelField, PaletteField) with different key sets, as the row's own note says; a behaviour difference between them was not hunted.
- Checked and found: all 112 colour tokens of core's catalog have a generated accessor (`DesignSystem/Theme/Tokens1.swift`, `Tokens2.swift`; the 8 `metrics.*` are in `ThemeMetrics.swift`); `settings_text_size` scales Settings and the Studio (`SettingsUI/SettingsView.swift:44`, `ThemeSync.swift:18`); icon motion, slider, checkbox, stepper, info button, status shift, tile orbit, window sketch, size readout, tint line all exist and are used.
- Not judged: whether each of GPUI's ~80 `style/` files has every state (hover, pressed, disabled, focused) in its Swift counterpart. That is the bulk of this row's weight and needs a component-by-component read.

#### X02 Theme application (95)

- Checked: the engine gets the theme's colours and the colour scheme (`engine_look.rs:79-80`), which the first audit found missing. Tokens resolve through the core (`UniFFISource/FfiThemeResolver.swift:13`).
- The project theme is one app-wide value (`DesignSystem/Theme/ThemeStore.swift:66`, set from the front terminal by `Wormhole/Settings/ProjectSettingsFollower.swift:57-68`) → with two windows in different projects, both take the front one's theme. Whether GPUI overlays per window was not established (`settings/mod.rs:398` sets one store value too), so this is a question, not a counted gap.

#### X03 Accessibility (85)

- A right-panel tab says its feature's own words: "Changes, 3 changed files", "Notes, 2 open next steps, 1 note", "Changes, no changes" (`crates/wormhole-core/src/view/spoken.rs:88-108`, used at `window/chrome.rs`) → Swift: partial, re-implemented differently: label is the bare title, value is "2 of 6, 3" (`WindowUI/PanelTabBar.swift:32-33, 51-54`); core's `panel_tab` is not exported by the bridge. Audit 2 named this; still open.
- Status bar items have spoken sentences: "Claude working, 4 minutes", "2 terminals working", the attention item, the counted items (`spoken.rs:201-256`) → Swift: partial: only the profile and size items have a label (`WindowUI/StatusBar.swift:50, 88`); the attention button and the count buttons are read as their visible text (a dot and a bare number when words are off).
- A section band says "Next steps, none open", "Notes, no notes" for a zero count (`spoken.rs:131-142`) → Swift: partial: `CardBand` takes a `spoken` string from each caller (`PanelUI/Shell/CardBand.swift:49`); Changes builds its own "N files" (`PanelUI/Changes/ChangesListView.swift:49`); the zero-count wording was not found.
- Settings "Changed" page copy button announcing "Copied" → Swift: missing (TO-VERIFY.md line 65 says so itself).
- Cannot be judged by reading: every SwiftUI control's label. `a11y` dumps only AppKit's part of the tree (DRIVING.md), so nothing has ever shown the labels a person hears.

#### X04 Keyboard navigation (90)

- Esc's "popup" rung: a menu, popover or rename field closes first and the keys return to its row (`crates/wormhole-core/src/view/focus/escape.rs:10-11, 22`) → Swift: partial: `popup: false` is always passed (`WindowUI/Focus/WindowFocus.swift:206`); a text field's Esc is left to the field (`:203`), so the rename closes, but nothing puts the keyboard back on the row.
- Checked: F6 order, Tab stops and the ladder run core's rules through the bridge (`focus.rs`, `WorkspaceBridge/CoreFocusRules.swift`); the panel lists that GPUI walks (Notes, Changes, History, Files) each have a walker (`PanelUI/Stores/*+Walk.swift`), one Tab stop per section (`PanelUI/Controller/PanelKeyboard.swift:26-28`); Project and Usage have no list in GPUI either (`features/project/def.rs:45`, `usage/def.rs:40`), so the row's "arrows inside Project/Usage" note is not a gap.
- Not judged: `window/focus/bars.rs` (title row and tab bar stops) line by line, and the driver limit that keys reach only the first window (TO-VERIFY.md 212) hides whether a second window's focus works.

#### X05 Reduce Motion, Reduce Transparency, Increase Contrast (90)

- Checked only that the first audit's two gaps now have code (`DesignSystem/ContrastOutline.swift`, `WindowUI/TitleTintLine.swift`, `Segmented.swift`). TO-VERIFY.md 304 says the segmented test edit never compiled.
- GPUI's `scroller overlay|legacy` behaviour (thin thumbs that fade or a gutter) is the system's scroll bar in Swift: decided, not a gap.
- Not judged: each Reduce Motion consumer against GPUI's list.

#### X06 Light, dark, system (95)

- Only the switch was read (`Wormhole/AppController+Look.swift:41-43`). The first audit's "`setTheme` moves the mode" was not re-checked.

#### X07 Languages (85)

`python3 -I macos/scripts/words.py` exits 1: "16 literals, 0 unknown ids, 0 variable mismatches; 16 new" (the allowlist is empty, so TO-VERIFY.md 67 and 72, "words.py is clean", are not true now).

- The terminal's context menu is hard-coded English: Open Link, Copy Link, Copy, Paste, Select All, Find…, Copy Command, Copy Output, Put Command at Prompt, Fold Output, Split Right, Split Down, Clear Buffer (`PlatformModel/Context/TerminalMenu.swift:44-66`) → not through `Words.text`.
- The checker skips every file whose path matches `Control|Options|Sample|Mock…` (`macos/scripts/words.py` `SKIP_PATH`), which also hides every `AppController+*.swift`, every `*Controller.swift` and `*Control.swift`. Read by hand, those hold interface text:
  - the close questions: "Close", "Close Group", "Close this window?", "Close Window" (`Wormhole/AppController+Quit.swift:113, 165, 181, 185`);
  - "End the Host", "Leave It Running" through `String(localized:)`, which reads no catalog of ours (`AppController+Quit.swift:131-132`);
  - "Close and Remove Worktree(s)" (`Wormhole/AppController+WorktreeClose.swift:36, 46`);
  - "Hide sidebar", "Show sidebar", "Hide right panel", "Show right panel" with their tooltips (`WindowUI/WindowControls.swift:25, 32, 44, 48`);
  - the open panel's "Open" (`Wormhole/Ambient/AppController+Ambient.swift:38`).
- Smaller: "esc" (`SettingsUI/Pages/ResultsPage.swift:40`), VoiceOver "changed" (`StudioUI/List/BandView.swift:70`), the placeholder "fix/login" (`PanelUI/Worktree/WorktreeSheets.swift:39`, arguably sample text).
- 30 view files read (PanelUI Shell, Changes, Diff, Stores, Worktree, Usage; WindowUI StatusBar, PanelTabBar, WindowControls; SettingsUI Preview and Pages; SidebarUI Samples): everything else a person reads goes through a module helper that ends in `Words.text`. 261 `// text:` marks and 30 `// text-file:` marks were sampled by their reasons; none read as hidden interface text.
- Parity itself: the language is chosen as GPUI does (system list, `WORMHOLE_LANGUAGE`, `language` on the control folder; `WordsBridge/WordTable.swift`, `Wormhole/Platform/PlatformSetup.swift:29`). Only English ships, so every item above shows only under `en-XA` today.

#### X08 CLI and control socket (90)

Every verb and flag of `the private docs' cli.md` against the Swift app's side. The parser, `help`, `skill`, `doctor`'s client half, `mcp`, `sessions`, `probe`, `welcome` are core's and run in the Swift binary (`hook_server/mod.rs:212-234`). All 19 `Action`s are dispatched (`hook_server/control/mod.rs:97-133`), the same set as `app/control/mod.rs:155-182`. Differences:

- `split --size <percent>`: GPUI gives the new pane that share and answers the share it got (`app/control/structure.rs:112-118`) → Swift: partial: the value is checked and then dropped; the pane gets the default share and the reply's `size` is always absent (`hook_server/control/open.rs:144, 185`).
- `new-terminal --worktree <branch> --command <cmd>`: GPUI's dialog opens the terminal running the command once confirmed (`features/git/worktree/ask.rs:93-126`) → Swift: refused with `unsupported` (`hook_server/control/worktree.rs:22-28`; the row's note says so).
- `new-terminal` without `--window` opens in the front window (`app/control/mod.rs:274-277`, cli.md line 58) → Swift: differs: the caller's own window, else the first window, never "the front one" (`hook_server/control/open.rs:56-59`).
- `new-window` is "like the front one" (cli.md 480) → Swift: "like the first one (its panels, no groups)" (`hook_server/control/structure.rs:40, 59`); and it answers `unsupported` when the workspace is not one of several windows (`:53-57`), a case GPUI does not have.
- `split` says "no room" before the budget is spent (`app/control/structure.rs:100-105`) → Swift: the budget is spent first, then the refusal (`open.rs:149-163`).
- `main.swift:13` hands the command line to core only when the first argument has no leading dash, so `Wormhole --json list` or `--text` first would start the app; not checked whether core's parser accepts a leading flag at all.
- Not judged: `events`, `read`, `blocks`, `wait`, `run`, `mouse`, `send-keys` flag by flag (their rules are core's; the Swift-side files were only grepped), and the row's own "run shows output only the first time, blocks lists nothing" in the marking-shell fixture, which is untraced and could be a real fault.

#### X09 Events not polling (90)

- Checked: no repeating timer drives the window; the fonts folder is now in the watch (`settings/watch.rs:78-96`); the 60 s screen save and the 3 s look at a folder with no repository match GPUI's own slow paths.
- Notes sync runs on a repeating timer (`Wormhole/Panel/NotesCoordinator.swift:100`) → not compared with `features/project/notes_sync.rs`; `icon-themes/` is watched by neither app.
- Not judged: the bridge's pump and agent detection cadence.

#### X10 Performance guards (85)

- GPUI has seven counting guards (`terminal_view/render/guards.rs:129-235`: characters shaped once, rows built in buffers it has, half blocks without shaping, a word shaped once whatever its colour, unchanged screen, blink phase, no clock without blinking text) → Swift: partial: four in `Tests/TerminalCoreTests/FrameWorkGuardTests.swift:32-57` plus the renderer's counters in the same target; no guard for the blink phase or "no clock without blinking text" was found.
- `scripts/perf.sh` and `scripts/profile-app.sh` time a release build against the machine's stored baseline → Swift: partial: `macos/scripts/perf-compare.sh` compares with GPUI, `perf-profile` writes a report; nothing keeps a Swift baseline and fails on a regression.

#### X11 Activation and isolation (95)

- Checked: Settings, Studio and secondary windows go through `DesignSystem/TestInstance.swift:14`; the bell is gated (`TerminalMetal/TerminalBell.swift:26`).
- About still activates the app unconditionally (`AuxWindows/About/AboutPanel.swift:19`), and the group colour panel takes the front (`Wormhole/Menus/GroupColourPanel.swift:25`) → in a background instance these take focus (test instances only).

#### X12 Dev tooling (75)

Every command in the header of `devctl/mod.rs` against the Swift control line (the dispatchers in `Wormhole/AppController+Control*.swift`, `Driving/*`, DRIVING.md). Inspect mode, perf spans, detach/reattach, the agent simulations, menubar-icons and the whole `studio-*` family are now written. With no Swift counterpart at all:

- Terminal: `blocks` (the list), `block <n> <select|copy-output|copy-command|to-prompt>` (Swift's `block` only prints the target), `pane-bar` (read what the bar says), `images`, `paints`.
- Files dragged: `drag <x> <y> <path>` (the hover with its tint) and `drag-end`; only the drop exists (`drop-files`).
- Window: `scaling` with `settle` and `reveal-test` (Swift `geometry` prints a part), `layout <columns|rows|grid|single>`, `panes`, `zen`, `scroller`, `front`, `close-window`, `grow-hold`.
- Menus and popovers: `menu-native [press]`, `popover`, `popover-press`, `popover-close`, `popover-type`, `popover-key`, `popover-colour`, `popover-native`, `popover-real`, `path-menu`, `available <action>`.
- Folder picker: the whole `folder-picker …` family (Swift has `folders`, which only opens it).
- Sidebar: `new-shell`, `shells`, `agent-hide`, `group-ask`, `rename-edit`, `folder-color`, `stash`, `send`, `send-submit`, `new [dir]` (no folder argument).
- Profiles: `profiles`, `profile-open`, `profile-move`, `profile-delete`.
- Panel: `files`, `files-root`, `files-follow`, `files-menu`, `files-pin`, `files-rename`, `project-init`, `comment`, `compare`, `picker compare`, `commit-details`, `commit-more`, `note-seed`, `note-toggle`, `task-seed`, `task-done`.
- Settings: `import-scan`, `font-picker`, `settings-record`, `settings-reveal`, `settings-preview`, `settings-reset-confirm` (Swift `reset` resets), `settings-cli-tool`, `update-installed`.
- Secondary windows: `hosts-toggle`, `hosts-scroll`, `licenses-search`, `licenses-select`, `intro-frames`, `onboarding-tailor`, `welcome-probe`.
- Notices and notifications: `notice sticky`, `notice-click`, `notice-material`, `notify-sim`, `notify-permission`.
- Driving: `control [--from <tab>] <words…>` (a request as from inside a tab, let in whole), `delay <ms>` (Swift `after` schedules one command, it does not hold the batch), `sync <token>`, `a11y live`, `refresh` (GPUI's cache only).

Reached another way, so not counted: `resize` (`size`), `pane-split/close/zoom/focus` (`split`, `close`, `zoom`, `focus`), `split-join/leave` (`join`, `leave`), `row-drop` (`drop`, `plan`), `to-group` (`ingroup`), `sidebar-fold` (`fold`), `group-new` (`group` + `groupname`), `group <name> …` (`group-menu` + `menu-pick`), `panel-tab` (`tab`), `changes/diff/split` (`tab`, `diff`, `panel-option split`), `commit`, `commit-file`, `commit-copy` (`history`, `history-file`, `history-copy`), `clipboard` (`pasteboard`), `close-answer` (`question`), `jump` (`ambient-run jump-to-waiting`), `menubar | dock` (`ambient`), `notify-log` (`notice-log`), `quick-panel*` (`quick-*`), `hosts` (`hosts-list`), `trust-window`, `hosts-window`, the `*-shot` family (`window-shot <path> <title>`), `settings-click/key/scroll/a11y` (`pointer-window Settings` + `click`, `key`, `scroll`, `a11y Settings`), `settings-select` (`settings-pick`), `settings-resize` (`settings-size`), `profile-sheet` (`profile`), `probe-off` (`probe off`), `project-trust` (`project-approve`), `menu-new-window | menu-updates` (`action`), `diff-scroll-x`, `sidebar-scroll`, `panel-scroll` (`scroll x y dy dx`), `toggle-sidebar/panel` (`sidebar`, `panel`).

Other gaps in the row:

- `pointer` means something else: GPUI prints or sets the pointer's place in points (`pointer | pointer <x> <y> | pointer off`), Swift's is `pointer ROW COL` over a terminal cell (`AppController+Control.swift:49-54`).
- `close <n>` closes row n at once without the question; Swift's `close` takes no row and closes the selected terminal through `closeTerminal` (`AppController+ControlWindow.swift:23`).
- Keys injected through the control folder reach only the first window (TO-VERIFY.md 212), where GPUI gives each window its own `cmd<n>`/`out<n>.txt`.
- `a11y` shows AppKit's nodes only; GPUI's shows the whole tree VoiceOver gets.
- Wave 5's commands (inspect, detach/reattach, agents, the studio family) were written without compiling; nothing here was run.

#### X13 Fonts registered at launch, folder watched (95)

Checked: core decides the changes (`settings/fonts_follow.rs`), Swift registers at start and on each change (`SettingsBridge/FontFolder.swift:30-49`, called from `RustSettingsBackend.swift:26, 58`), a custom family replaces the bundled one, and the terminal, code and interface fonts re-resolve (`Wormhole/Settings/ProfileLook.swift:66-80`). Not read: `evidence/s3-themes-fonts/X13.md`.

#### X14 Theme files, project overlay, icon themes (90)

- Checked: the overlay is read and applied (`FilesBridge/Project/RustProjectBackend.swift:52`, `ProjectSettingsFollower.swift:63-68`, `ThemeStore+Resolve.swift:22-27`), icon themes are looked up by core with the flag rule (`Wormhole/Settings/SettingsApplier.swift:108-114`), import and trash of a theme file exist (`settings/theme_files.rs`).
- Not read to the end: version-1 files opened in place by the Studio (the first audit's gap; `settings/theme_picks.rs` and `StudioModel` were not followed), and the light/dark family of an icon theme.

#### X15 Legacy upgrades (90)

Checked: all three are written and called (`legacy.rs:26, 38`, `Wormhole/Platform/LegacyLaunch.swift`, from `AppController.swift:109` and `AppController+Open.swift:16`), the bundle id matches GPUI's (`platform/default_app.rs:47`). They never run today (`LegacyLaunch.applies` is false for `Wormhole-swift*`), so nothing has exercised them; the role take-over is skipped outside an `.app` bundle.

#### Not covered by any row

- Logging: GPUI writes a daily log to `<data dir>/logs/`, honours `WORMHOLE_LOG`, and writes a panic down (`app/logging.rs:1-7`). The bridge depends on `tracing` but installs no subscriber (`crates/wormhole-ffi/Cargo.toml:23`; no `tracing_subscriber`, `tracing_appender` or panic hook anywhere in `crates/wormhole-ffi`), so every `tracing::` line of the core is dropped. Help → Show Logs and the notice's "open logs" create and open an empty folder (`Wormhole/AppController+Open.swift:61-65`), and Settings still tells the person to set `WORMHOLE_LOG=debug`.
- The dev loop around the app: `scripts/dev.sh` (rebuild on change, last good instance kept), `scripts/devctl-instance.sh` (scratch HOME, agent home, `/bin/cat` shell in one call) and `tools/visual/` scenarios have no row; Swift has `visual-run.sh` and `visual-scenes.py`, no rebuild loop and no one-call isolated start.
- The structure gate (`scripts/structure.py`, the design and text rule tests in `crates/wormhole-rules`): Swift has `rules.py`, `layers.py`, `standards.py`, `words.py`; no row tracks that they hold, and `words.py` fails today.

## Coverage: what no row names

Tree: the branch `exp/swift-s5`. Paths are relative to it; GPUI = `crates/wormhole/src`, core =
`crates/wormhole-core/src`, bridge = `crates/wormhole-ffi/src`, Swift = `macos/swift/Sources`.

```
item,covered_by_row,swift
Log files (<data>/logs, daily, 7 kept), WORMHOLE_LOG level, last-panic.txt,none,missing
Help > Show Logs in Finder,none (PL01 only as one of the actions),partial
Running as the product: real data folder, real bundle id, Finder launch, packaging keys,none,missing
Data folder made owner-only at launch (files of earlier versions),none,partial
Settings > Components page (components flag, dev builds),none,missing
Settings > Changed page (badge, per-setting put-back, Copy as JSON, Reset All),none (ST01 names per-row reset only),written
Settings > Motion page and its preview,none,written
Settings > Hotkey page and its preview,doubtful (ST03 says "Menu bar"; PL02 the hotkey itself),written
Terminal welcome (wh welcome program in the first terminal, example band in the sidebar, replay),doubtful (A01 "welcome page"),written
Changed files counted per terminal (attribution; scope/plain/shared; remembered after restart),doubtful (S01 "count badges"),written
Recent projects and recent folders (Open Recent, status item, picker's Recent rows),doubtful (PL01 "dynamic items", O03),written
First command of a new terminal typed at its first prompt (--command),doubtful (X08),written
Project folder: prompts/*.md (paste, copy into a note), layout.json "Open terminals", action keys,doubtful (P08 "actions"),written
Hide from Agents (menu item, per-terminal flag),doubtful (A05; only O04's notes say it),written
DEC locator reports and horizontal wheel reports,none (T05 names modes 1000-1006),written
Double-width and double-height rows (DECDWL, DECDHL) drawn and pointed at,doubtful (T01, whose note says "not compared"),written
Words that change with time (working 3m, resets 34m) on a one-shot timer,doubtful (X09),written
Data folder that cannot be opened: a notice, stderr, the log,none,partial
Closing a terminal whose host runs but cannot be shown (end it or leave it),doubtful (T16/W08),written
Move a group with its splits to a window of its own; group folder and "only terminals in this folder",doubtful (S03/S07),written
Paste over a selection a full-screen program drew (Backspace first),none (T21),written
Git "dubious ownership" explained with the one command,none (P02/P04),written
Updated on disk from outside (Homebrew) noticed; "is ready - Restart" chip,doubtful (A03/O05),written
Send a test notification row; notification permission asking,doubtful (PL03),written
The wh script rewritten at every launch; the binary answering hook/statusline/welcome/control,doubtful (X08),written
```

#### Findings: missing and partial first

1. **The Swift app cannot run as Wormhole (no row).** `Wormhole/main.swift:26` calls `Isolation.require()`, which
   throws unless the bundle id is `app.getwormhole.terminal.swift` and `WORMHOLE_DATA_DIR` names a folder called
   `Wormhole-swift*` (`Wormhole/Isolation.swift:19-37`). A launch from the Finder exits with code 2. The legacy
   upgrades are written but switched off for the same reason (`Wormhole/Platform/LegacyLaunch.swift:22`). Its
   `Info.plist` lacks what `packaging/Info.plist` has: icon file and name, `LSMinimumSystemVersion`,
   `LSApplicationCategoryType`, `NSPrincipalClass`, `NSSupportsAutomaticGraphicsSwitching`, copyright. `macos/scripts`
   has no bundle, signing or entitlements step. This is the experiment's safety rule and deliberate, but no row names
   the switch to the real identity, so "100 %" does not include it.
2. **Logging is missing (no row).** GPUI writes a daily log to `<data>/logs`, keeps seven, honours `WORMHOLE_LOG`,
   and writes every panic to `logs/last-panic.txt` (`crates/wormhole/src/app/logging.rs:23`, `:57`; started at
   `main.rs:84-85`). The bridge depends on `tracing` only (`crates/wormhole-ffi/Cargo.toml:23`); nothing in the bridge
   or in Swift installs a subscriber or a panic hook, so every `tracing::warn!` of the core and the bridge goes
   nowhere. Help → Show Logs in Finder creates and reveals an empty folder (`Wormhole/AppController+Open.swift:61-65`),
   and Settings → Advanced still says "Set WORMHOLE_LOG=debug for more detail"
   (`SettingsModel/Resources/settings.json:2525`). Three of the changelog's Unreleased fixes promise "and in the log".
3. **Settings → Components is missing (no row).** GPUI: `settings_window/sections/components/` (1,170 lines, behind
   the `components` flag, development builds). Swift's schema has no such section; the bridge says so
   (`crates/wormhole-ffi/src/settings/search.rs:47`). A developer tool, not on the brief's list of things left out.
4. **Data folder made owner-only: partial (no row).** GPUI tightens the modes of everything in the data folder at
   every launch (`main.rs:93-95`, `storage::store::make_data_dir_private`). Swift only creates the folder itself
   with mode 0700 (`Isolation.swift:34`); nothing calls the core function. It matters the day finding 1 is lifted and
   the folder is an existing one.
5. **Data folder that cannot be opened: partial.** GPUI says it in a notice, on stderr and in the log
   (`app_state/data_folder.rs`). Swift has the notice (`Wormhole/Notices/NoticeReason.swift`,
   `TerminalUI/Notice/NoticeWords.swift`); the log half falls under finding 2.

#### Not named by a row's text, but written in Swift

I found the code and one path from the app to it for each; I did not audit them in depth, which no auditor does
if no row names them.

- Settings → Changed: `SettingsUI/Pages/ChangedPage.swift`, shown at `SettingsUI/SettingsView.swift:66`
  (GPUI `settings_window/changed.rs`).
- Settings → Motion and Hotkey pages: sections `motion` and `hotkey` in `SettingsModel/Resources/settings.json`,
  previews in `SettingsUI/Preview/Motion/` and `Preview/HotkeyScene.swift` (GPUI `sections/motion.rs`,
  `sections/hotkey.rs`).
- Terminal welcome: `Wormhole/Hooks/HookHost+Welcome.swift`, `SidebarUI/Band/ExampleBand.swift`, bridge
  `hook_server/welcome/` (GPUI `window/welcome_host.rs`, `sidebar/demo.rs`, `app/welcome.rs`).
- Changed files per terminal: bridge `workspace_git.rs:13`, `windows/claims.rs`; settings read in
  `Wormhole/Settings/HubSettings.swift` (GPUI `window/repo/attribution.rs`, `remembered.rs`).
- First command: bridge `hook_server/control/open.rs:220` (GPUI `terminal_view/first_command.rs`).
- Project prompts and layout: `PanelUI/Project/ProjectSectionCards.swift:49-63`, `ProjectView.swift:26`
  (GPUI `features/project/actions.rs:73`).
- Hide from Agents: `Wormhole/Actions/ActionRouter+Edit.swift` (`hideFromAgents`).
- DEC locator: bridge `interaction/pointer.rs:102`; wheel: `TerminalMetal/Pointer/TerminalWheel.swift`
  (GPUI `terminal_view/input.rs:325`, `:525`).
- Double-size rows: `TerminalMetal/RowLayout.swift`, `Stretch.swift` (GPUI `terminal_view/render/sized.rs`,
  `render/outline/`).
- Paste over a program's selection: bridge `interaction/app_selection.rs:17` (GPUI `terminal_view/input.rs:223`).
- Dubious ownership: bridge `git/problem.rs`, shown by `PanelUI/Changes/ChangesListView.swift:20`.
- Replaced on disk: bridge `aux_backends/updater.rs:152`.
- Test notification: `Wormhole/Settings/SettingsPlatform.swift:71`.
- `wh` at launch: `Wormhole/Platform/CommandLineSetup.swift:10`, called at `PlatformSetup.swift:34`; command-line
  modes of the binary: `main.swift:13`, bridge `hook_server/mod.rs:214-225`.
- Every action of the keymap table has a handler (`Wormhole/Actions/ActionRouter*.swift`,
  `PlatformUI/Menus/StandardActions.swift`); every control verb the GPUI app answers is answered by the bridge
  (`hook_server/control/mod.rs:98-135` against GPUI `app/control/mod.rs:87-182`); every setting key of the schema has
  a reader outside the schema file, or is read by the core through the bridge.

#### Rows that name something the GPUI app does not have

- **T22 "Print and Export text"**: GPUI only reserves ⌘P and ⌘S (`core keys/keymap/table.rs`, `.reserved()`); the
  actions do nothing (`crates/wormhole/src/actions/mod.rs:291`). Swift built both
  (`Wormhole/Actions/ActionRouter+Output.swift`). The row measures no GPUI behaviour.
- **S06 "Show filter and search"**: GPUI has neither any more; only the reading of an old saved `filter`
  remains (`core view/sidebar/arrange.rs:368`). The row's weight of 1 already says so; Swift's sidebar search is
  its own.
- **ST02 "Terminal list" and ST03 "Preview" as pages**: GPUI's sections are Profile, Changed, Appearance, Fonts,
  Motion, Terminal, Sessions, Git, Agents, Projects, Extensions, Feature flags, Components, Import, Notifications,
  Menu bar, Hotkey, Updates, Advanced (`core config/settings_index/table/mod.rs`). There is no Terminal list page;
  Preview is a nav group, not a page; Menu bar became two pages.
- **PL01 "113 actions"**: the table now makes about 132 entries (`core keys/keymap/table.rs`).

#### Not a gap, noted

GPUI's terminal has no input method support (no input handler in `terminal_view/`); Swift has one
(`TerminalMetal/Keyboard/InputClient.swift`, `MarkedText.swift`). Neither app has URL schemes, a Services
provider, or exported file types (`packaging/Info.plist`); document types and usage strings match (PL07).

#### Estimate

**About 4 % of the GPUI app's behaviour is named by no row's text; under 1 % is both unnamed and not written in
Swift, plus the one gate of finding 1.**

Reasoning: the GPUI files behind the unnamed items above come to about 5,500 lines (3,000 for the welcome,
attribution, first command, double-size rows, Changed, Motion, Hotkey, recents, clock, logging, host close, group
moves and the data-folder notice; 1,360 for the motion previews and the glyph outlines; 1,170 for Components)
of 144,000 non-test lines in `crates/wormhole/src`, against a ledger weight of 115,188. By importance to a person
the share is similar: most of these are small parts of features whose row exists under a broader name. The missing
ones are logging, the Components page and the data folder's modes: small in lines, but logging is what a bug
report rests on. Finding 1 is not a share of behaviour: it decides whether any of it can be used.

How sure: fairly sure of the inventory of actions, menus, verbs, settings keys, flags and settings sections, which
I compared one by one. Less sure of the "written" verdicts: for most I confirmed the code and one call path, not
every behaviour. Not checked: the devctl command list against Swift's driver, the Theme Studio's parts, the palette's
entries beyond the action table, and the engine-side sequences.
