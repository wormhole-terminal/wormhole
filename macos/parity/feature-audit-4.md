# Audit 4: what wave 6 closed (2026-11-13)

Audit 3 found the Swift app 88.6 % written and listed every gap. Wave 6 (seven lanes) worked those lists; the branch then
built and its tests passed (413 XCTest, 400 Swift Testing, none failing). Six readers who had not written the fixes then
took every gap line of audit 3 and read its fix to where the app reaches it: closed, partly, open, or wrong. Nothing was
built or run for this audit. Line numbers are of the branch `exp/swift-s5`.

**Written and reachable: 93.4 %** by the tracker's weights (audit 3: 88.6 %); 57 of 107 rows with no gap left.
`written.csv` holds the per-row numbers; `built.py` prints the figure. Rows no fix touched keep audit 3's number, not re-read.

| Area | Share of the app | Audit 3 | Audit 4 | Rows with no gap |
|---|---|---|---|---|
| crosscut | 21.5 % | 86.6 % | 90.7 % | 1 of 15 |
| panel | 15.4 % | 87.8 % | 92.7 % | 1 of 9 |
| terminal | 13.4 % | 97.1 % | 98.8 % | 30 of 31 |
| studio | 9.6 % | 89.2 % | 89.3 % | 1 of 3 |
| window | 9.2 % | 88.8 % | 95.2 % | 7 of 12 |
| sidebar | 7.9 % | 88.9 % | 97.5 % | 7 of 10 |
| aux | 7.8 % | 84.7 % | 93.1 % | 2 of 7 |
| settings | 7.3 % | 85.5 % | 93.1 % | 3 of 7 |
| overlays | 4.3 % | 88.3 % | 90.8 % | 1 of 5 |
| platform | 3.6 % | 84.8 % | 93.7 % | 4 of 8 |

First to fix, found by this audit: a Notes merge whose terminal was switched away mid-way can remove lines from `notes.md`
(P06); two windows of one profile lose each other's worktree records (P05); "Open Window in <profile>" opens the front
window's profile, a regression of wave 6's own fix (W05).

## Terminal (T01 to T31)

id,written
T01,95
T02,100
T03,100
T04,100
T05,100
T06,100
T07,100
T08,100
T09,100
T10,100
T11,100
T12,100
T13,100
T14,100
T15,100
T16,100
T17,100
T18,100
T19,100
T20,100
T21,100
T22,100
T23,100
T24,100
T25,100
T26,100
T27,100
T28,100
T29,100
T30,100
T31,100

#### Gaps still open

T01 (95)
- The block cursor's cell keeps its own background colour under the cursor in GPUI, so an edge cell with a coloured background still runs into the padding there (`terminal_view/render/paint.rs:160-176`; the cursor is painted apart from the cell backgrounds) -> Swift: the cursor cell's background is replaced by the cursor colour (`RowBuilder.swift:46-47` `colors`) and marked `fixedBit` (`RowBuilder.swift:97`), so with a coloured full-screen background and the cursor in column 0, the last column or row 0, the padding beside that one cell shows the clear colour: a small notch. Only the block cursor when focused; beam, underline and hollow cursors are decoration quads and are not stretched.

#### Per row

- T01: 1 of 1 gap CLOSED in the main (edge cells run into the padding on all four sides), with the notch above as a new small one. Read: `Shaders.swift` solid vertex (`extend`, `origin`, `gridEnd`), `MetalRenderer.swift:406-425` (whole-pane viewport only for the backgrounds draw, restored after; images and glyphs use the grid viewport), `RowBuilder.swift:90-100`. The Uniforms layout matches Metal's (40 bytes, float2 aligned at 8).
- T07: 2 of 2 (the not-judged keyboard-layout note is not a gap): CLOSED. `TerminalShortcuts.swift` now asks `terminalAction`, set for every terminal in `TerminalLook.dress` (`TerminalLook.swift:105-125`) and answered from `Keymap.shared` rows in the Terminal and Edit scopes and live; ids (`copy`, `paste`, `selectAll`, `clearBuffer`, `deleteLine`, `lineStart`, `lineEnd`) and key names (`backspace`, `left`, `right`) match the table. A rebound key follows.
- T16: 3 of 3 CLOSED. Exited bar: ffi `workspace_waited.rs` `shell_name` -> `RustWorkspaceBackend+Hosting.swift:185` -> `WindowHub+Overlays.swift:58` -> `TerminalPanes.swift:61`. Notice kept behind a question or host bar: `TerminalOverlay.swift` `note`, `sayInBar`, `askEnded`. Block notices go to their own terminal's bar (`say` closure, `chipOn`); program copies already went to their own terminal; the window's route (`WindowHub+BarNotice.swift`) only carries what is left.
- T19: 1 of 1 CLOSED (no behaviour was missing). The order you asked about is the same in both: `refresh_agent` in core (`workspace/status/front.rs:36-59`) is line for line `refresh_foreground_agent` (`terminal_view/status.rs:372-393`); a hook sets `facts.agent` and `hook_driven` (`status/hooks.rs:44-45`, GPUI `apply_signal`); a later look that sees no agent keeps the hook's status unless the shell holds the terminal (`!shell_in_front`), and only then goes idle. The bridge passes `shell_in_front` from `pty.shell_in_foreground()` (`workspace_feed.rs:175-181`, `terminal.rs:454`), as GPUI reads it. So a real agent's first report is not lost: while the agent runs the shell is not in front. What the build's test showed (row back to idle) is the case where the shell is in front (the `/bin/cat` test shell), which GPUI does the same. I compared front.rs, hooks.rs and the reported path, not every branch of the rest.
- T20: 2 of 2 CLOSED. `hosting/checkpoints.rs:76-100` removes the old checkpoint on the host's word or keeps it after 10 s, whichever first (same constants as `checkpoint_plan.rs:48`); `HostConnect.swift:51-54` `keptPastQuit` detaches on the thread that made the host, set by `quit()` which `RustWorkspaceBackend+Hosting.swift:164` calls.
- T25: 2 of 2 CLOSED. Wheel: ffi `interaction/pointer.rs:197` `pty.scroll_rows`; drag: `DragSelection.swift:14` -> `UniFFIFrameSource.swift:195` -> `terminal.rs:297`. ⌘↑/⌘↓: `BlockController.swift:105` `source.topLine` -> `UniFFIScrollback+Blocks.swift:27` -> ffi `blocks.rs:146`.
- T29: 1 of 1 CLOSED. `WindowOps.swift` `asked` is set after a resize, used by `pane(of:in:)` while the grid is unchanged and under 1 s, forgotten on maximise, restore and full screen.
- Other rows: audit 3 gave 100; wave 6 touched `KeyDown.swift` (a drop on the title row, `insertDropped`), nothing that read as a regression.

#### New

- T01 notch under the block cursor at an edge (above), small.
- `TerminalOverlay.swift`: the `WeakOverlay` box is set after the stored properties; fine as written, noted only because it is compile-unverified.
- Nothing broken or half-wired found in the terminal area. Not checked: the whole of `Wormhole/AppController+Control*.swift` (other lanes' area).

## Right panel (P01 to P09)

id,written
P01,95
P02,95
P03,95
P04,100
P05,85
P06,85
P07,95
P08,85
P09,90

Read only; nothing built or run. Swift paths are under `macos/swift/Sources/`, bridge paths under `crates/wormhole-ffi/src/`.

#### The three things asked about

##### 1. The repository root: where a git path becomes a path on disk

`PanelController.repoRoot` is `root ?? folder`. `root` is `git.repositoryRoot(folder:)` (ffi git/service.rs:75, the same `find_root` that `status` and `diff` use), asked once per folder and cached. `settle` puts it in `changes.root`. The three panel views put it in the environment as `panelRoot`: PanelContentView:28, PanelCoverView:28, PanelCoverTitle:48. Each place, followed:

- Changes row menu, Copy Path: joins `panelRoot` (Changes/PathMenu.swift:44). Joined.
- Changes row menu, Open and Reveal: `RepoFiles.resolve(root:relative:)` (the core's resolver), and `exists` is asked of the joined path (PathMenu.swift:27-31, 49). Joined. The `?? absolute(path)` fallback is gone, so a refused path is not handed on.
- Changes row's open icon: FileRowView.swift:49 resolves against `root` and does nothing when the core refuses. Joined, no fallback.
- Diff header's open button: DiffHeader.swift:75, `root.flatMap { resolve }`. Joined.
- Diff per-line icons: DiffView.swift:128. Joined.
- Reveal on the too-large card and the LFS note: DiffNote.swift:82. Joined.
- History's file rows and menu: they use the same FileRowView and PathMenu inside PanelContentView or PanelCoverView, so the environment is the same. Joined.
- Submodule row's terminal button: SubmoduleRowView.swift:34 uses `root + "/" + file.path`. Joined. It is not resolved within the root, which was not asked of it.
- `ChangesStore.stamp`: `resolve(root: root ?? folder, ...)` (ChangesStore.swift:172). Joined.
- The Files tab uses `files.follow(root ?? folder)` and `store.join`. It was already right.

Verdict: CLOSED for every place audit 3 named. Two small leftovers:
- Until git answers, `repoRoot` is the terminal's folder, so a menu opened in that first moment in a subfolder is still wrong. The window is short.
- `root` is not asked again after `git init` in a folder that was not a repository, until the next folder change.

##### 2. Worktrees (P05)

- Trust asked of the repository: CLOSED.
  - Swift: `RealWorktreeBackend.repository(of:)` runs the FFI's `repo_of`, which gives the main working tree. `problem` and `create` both ask `allowed(repository)`, which is `project.worktreeRefusal(of:)`. That is `ProjectService.worktree_refusal`, which returns nil when project folders are off (service.rs:77-90).
  - Command line: control/worktree.rs:35-43 checks `worktree_refusal_in(seat.profile.dir(), &repo)` only when `ProjectFolders` is on, using the profile of the seat the request reached.
  - The window's own profile's store: the trust check uses the window's `project` backend (bound to its profile). The worktree store follows the profile (`follow_profile`, ProfileChipBinding.swift:157) and `env_for(cwd, profile)` reads the terminal's own profile.
- `--worktree --command`: CLOSED, with a timing risk that only a run settles.
  - Chain: control/worktree.rs `command` goes into `HookEvent::WorktreeAsked`, then `HookAction.worktreeAsked`, `HookHost.perform`, `askWorktree`, `WorktreeStore.askedCommand`, `create`, `backend.create(... command)`, the `worktreeCreated` closure (PanelBackendsFactory.swift:66-80).
  - The closure builds `setup && command` and sends `.paste(line, submit: true, in: .new(cwd:))`.
  - `PanelEffects.terminal(for: .new)` makes the terminal, waits up to 1 s for its view, pastes into that view, then sends Return 120 ms later. It no longer writes into whichever terminal is selected.
  - With no setup and no command it sends only `.newTerminalIn`.
- The second request answered Failed: PARTLY.
  - The flag is `Core.worktree_question`, one per server, not per window. With two windows, a sheet in window 1 makes a request for window 2 answer "already up in that window", although window 2 has no sheet. When either sheet closes it sets the flag false while the other may still be up. GPUI's check is per window, so yes, this is wrong with two windows.
  - `sheetChanged` is only assigned inside `HookHost.perform(.worktreeAsked)` (HookHost.swift:103), and only on the store of `app.hub`, the front window. A sheet opened from the menu before any command-line request does not tell the server, so the second request is not answered Failed. The doc comment on `worktree_sheet` says "however it was opened"; the code does not do that.
  - The sheet is shown in the front window (`app.hub.askWorktree`), not the window the request reached. The event carries no window. With a profile-B window behind a profile-A window, the FFI judged trust by B's profile and the sheet then asks A's store.
  - Not reset: `WorktreeStore.ask` returns early when `asking` is already true (WorktreeStore.swift:80-93), so a second request does not reset the sheet.
- Trust refusal at Create keeps the sheet up: CLOSED. `create` sets `error` and returns with the sheet up. `footnote` shows `error` first, and `isRefusal` includes it.
- Closing a group sees folded terminals and whole path components: CLOSED. It uses `state.all` and `inside` by components (AppController+WorktreeClose.swift:21, 41).
- Per profile: CLOSED for the profile. See "New" for a cross-window loss.

##### 3. The Notes merge (`NotesModel.rebase`, `NotesCoordinator.sync`)

The merge that audit 3 named is closed. In the shown terminal the answer is applied on the main actor, with no suspension between reading `now` and writing the model. The rebase logic:
- It keeps a changed or added item as the person left it.
- It keeps a deleted item deleted.
- It keeps the folds the person set.
- It puts a new item after the one it followed.
- A person's tick during the await survives, as does an item added during it.
- If the observation callback runs before or after `sync` resumes, the result is the same: `model` equals `saved`, or `changed()` saves it.

Orders of events that still lose or overwrite something:

1. **The terminal is switched during the await.**
   - `guard let answered = await persistence.sync(...), id == shown else { continue }` (NotesCoordinator.swift:119) drops the answer.
   - The core has already merged against `notes.md`: the file is written and the merge base is advanced (`syncNotes`). The coordinator never saves `answered`.
   - The stored model for that terminal is the old one. The next sync merges it against the advanced base, so lines the file gained read as lines the person deleted, and they can be removed from `notes.md`.
   - This is the exact loss the code's own comment ("a merge left for the next edit is lost on quit ... lines the person deleted") describes.
   - The same hole, mirrored, is in the not-shown branch (lines 127-130). It loads the stored model, awaits, then saves the merged model. If that terminal becomes the shown one during the await, `changed()` has loaded the old model into the view. Saving the merge underneath it, with the view's old model left in place, gives the same stale-model against advanced-base state.
2. **`editing` is read once, before the loop** (line 107). Terminals before the shown one are awaited first (`root(of:)`). A person who opens a step editor in that time is merged as "not writing", and `hub.panel.notes.model = merged` replaces the model under an open editor.
3. **Nothing stops two `sync()` runs overlapping** (the 2 s timer starts a Task each time). Run 2 is rebased against run 1's result as if it were the person's edit. It converges but can bring back a line the file deleted in between.
4. **A person's reorder during the await is lost.** The rebase takes the merged order. Ticks, edits and additions are kept.
5. **A tick on an item the file also edited keeps the person's old text for one round.** The file's edit arrives on the next sync. Nothing of the person's is lost.

So: a person's tick or new step made during the await is no longer lost. A merge whose terminal was left during the await is still lost and can delete file lines.

#### Per-row gaps below 100

##### P01 (95)
- Tab has the tab role in a tab list, each tab with its own words (window/chrome.rs:279-286) → Swift: words and counts are core's (`TabSpeech`, WindowHub+TabSpeech.swift; `installRules` installs it, FilesBridge.swift:46); the strip is `.isTabBar` but each tab is a button with `.isSelected` (PanelTabBar.swift:29-32), not a tab. Minor.
- Closed and read: a tab's action is bound only while offered (WindowHub.swift:113-121; menu hides Usage and greys Project, MenuStateMaker.swift:86-93); Zen changes the tab without opening the panel (`.setPanelTab`, line 121); a tab no longer offered leaves on observation, panel open or not, and at init (`followTabs`, `leaveMissingTab` through `setPanelTab`); the title-row button gives the terminal the keys (WindowBody.swift:61-65); tooltips name their key (`host.keyed` → `Keymap.withKey`; RightPanel.swift:25; NotesCard:25; StashPromptButton:26); slide direction by `PanelTab.barOrder` (RightPanel.swift:52).

##### P02 (95)
- The branch chip shows Changes and opens "compare with" (features/git/changes/def.rs:108-117) → Swift: wired (`actions.pickBranch` assigned at WindowHub.swift:72; `pickBranch(of:)` sets `changes.pickerRequested`; RevPicker `requested` opens it). Two rough edges:
  - For a pane that is not the selected terminal, the request is set before the panel follows the new terminal (WorkspaceRootView.swift:85 follows later). The picker opens for the old terminal, then `.onChange(of: scope)` closes it (RevPicker.swift:44).
  - The request is only consumed when a Changes list with a picker is on screen. With a non-repository folder the flag stays set and the picker pops open the next time a list shows (ChangesListView.swift:55-57).

##### P03 (95)
- The comment form is a dialog titled "Comment on <place>" with Add Comment / Save (features/git/diff/review.rs:92-178) → Swift: still a popover (Diff/CommentEditor.swift); the lane did not write it. A design choice in the lane's view; not counted as more than a few points.
- Closed and read: root for the header button, line icons and Reveal; the quote cut at 160 Unicode scalars with all white space trimmed (Review.swift:83-89); "Loading diff…" (the `swift-panel.ftl:111` line is changed, not only the fallback); a tab width change measures the widest line again (DiffSession.swift:54-59, through `PanelPreferences.tabObservers`, which only a shown diff reacts to).

##### P05 (85)
- A second request while a sheet is up, per window, answered Failed (ask.rs:120-125) → Swift: PARTLY, see section 2 (one flag per server; menu-opened sheets do not set it; the sheet goes to the front window, not the window the request reached) (hook_server/mod.rs:152, control/worktree.rs:50, HookHost.swift:103).
- The question comes up in the window the request reached → Swift: `app.hub` is the front window (HookHost.swift:104); `HookEvent::WorktreeAsked` has no window (hook_server/control/worktree.rs:57).

##### P06 (85)
- A merge is applied to what the terminal holds now (features/notes/api.rs:54-98) → Swift: the tick-in-await case is closed; a merge for a terminal left during the await is dropped after the core already merged (NotesCoordinator.swift:119, 127-130). See section 3, items 1 to 4.

##### P07 (95)
- A terminal whose own folder is not UTF-8 (paths are `PathBuf` throughout) → Swift: the root comes from the workspace as text; the lane did not write it.
- Closed and read: ↩ and Space on the path button open the path menu under it (FilesStore+Walk.swift:24-31 `pressesBand` and `pathMenuRequests`; FilesPathButton.swift:35 `showKeyMenu(nsMenu(menu), frame)`, wired by RightPanel.swift:33 and AppController+Context.swift:41). Walking onto it with a viewer up does nothing, since `following` is true there. The menu items are the same as the SwiftUI menu's (FilesPathMenu.swift).

##### P08 (85)
- Every layout terminal is selected as it is made only when `focus` says so (window/feature_terminals.rs:100-126) → Swift: PanelEffects.swift:67-73, `make` runs `.newTerminalIn`, which selects the new terminal whatever `spec.focus` says, and nothing puts the old selection back. A layout of five terminals leaves the last one selected.
- An action's key is the window's wherever the focus is, and lets the key through once the project is not trusted (features/project/actions.rs:56-70) → Swift: not written (the lane's own NOT WRITTEN line; a text field still takes the key first).
- Closed and read: "Open terminals" matches by `customName` (PanelEffects.swift:63-65), so a title that happens to equal the name no longer blocks a layout terminal and a renamed terminal is not opened twice.

##### P09 (90)
- Reports are kept per profile and each window draws its profile's (features/usage/state.rs:26-51, 85-95) → Swift: one shared service, bound to the first window's profile (PanelBackendsFactory.swift:33, 51-53); "Move Window to profile" moves it for every window. The lane's own NOT WRITTEN line.
- Closed and read: the words redraw at core's `next_change` (UsageStore.keepTime, UsageStore.swift:35-42, through `UsageService.next_change`, usage.rs:171). The task is the view's, so nothing ticks while the tab is hidden. The sleep is chosen from the reports at that moment and nothing wakes it when a new report arrives, so a report that puts a limit into its last minute is shown per second up to 60 s late. Small.

#### Gaps closed out of audit 3's
- P01: 6 of 7 closed, 1 partly (the tab role).
- P02: 4 of 4 closed (root; chip; compare survives `cd`; planted link no fallback), with the two rough edges above.
- P03: 3 of 4 closed (root; quote; wording; tab width are four, the dialog is the open one), so 4 of 5 counting the dialog.
- P04: 2 of 2 closed.
- P05: 6 of 7 closed, 1 partly (the second request).
- P06: 1 of 2 closed (draft left on terminal change closed; merge race closed for the case audit 3 named, but not for a terminal switch).
- P07: 1 of 2 closed (↩ on the path head; non-UTF-8 not written).
- P08: 1 of 3 closed (the match by custom name); selection as made and the window-level key are open.
- P09: 1 of 2 closed (per-second clock); per profile open.

#### New
- **The worktree service keeps its own cache of the stored worktrees, per window, and writes the whole list back** (git/worktrees.rs `load`, `save`, `create`). Two windows of one profile each hold a `WorktreeService` with their own cache. Window B's `create` or `forget` saves B's list over window A's. A forgotten record loses that worktree's ports (`env_for`) and its "Close and Remove" offer. Ports are also picked from each window's own cache (`free_ports`), so two windows can hand out the same port. Wave 6 made the store per profile, which makes this reachable with two windows. It was not in audit 3.
- **The sheet's flag and the sheet's window**: see section 2 (one server-wide flag; sheet in the front window).
- **Notes**: a merge dropped on a terminal switch can remove file lines (section 3, item 1).
- **Branch chip request**: stays pending when no Changes list is on screen, and is closed by the terminal switch for a non-selected pane (P02).

## Sidebar and window (S01 to S10, W01 to W12)

id,written
S01,100
S02,100
S03,95
S04,100
S05,100
S06,100
S07,100
S08,95
S09,95
S10,100
W01,80
W02,100
W03,95
W04,100
W05,95
W06,95
W07,100
W08,100
W09,100
W10,100
W11,100
W12,90

Read only, nothing built or run. Judged from the code and followed to where the app reaches it.

#### Gaps still open, per row

S03 (95)
- A group gone while the question is up: GPUI opens the terminal loose (group_folder.rs:~150-160) -> Swift: `.newTerminalInGroup(group)` goes to the core, whose `new_terminal_in_group` refuses for a missing group (workspace/groups.rs:125), so no terminal opens (SidebarView+Group.swift:31, SidebarContext+Away.swift:29, WindowHub+Sidebar.swift:145). Rare.
- Test seam: `GroupFolderQuestion.asker` is a static the control folder sets; with it, two quick + presses can queue two questions (the system alert is modal, so not in the product).

S08 (95)
- The insert pose in a group's card is `.transition(.listRow(...))` on the row (GroupRows.swift:45), the same as a row in no group, but whether the container animates the change cannot be settled from the code (needs a real Mac).

S09 (95)
- The sets, stepping, numbering and search are still a Swift copy of core's `sidebar::tabs` (PanesModel/Tabs/TabSets.swift, TabBarModel.swift). The alert dot's source is fixed (`TerminalStatusTile.alert(of:)`, WindowHub+Sidebar.swift:60; amber or red by the tile's urgency).

W01 (80)
- A split opening plays from the layout before it: the new pane slides in, the old terminal keeps its grid size until the end and is resized once (window/panes/opening.rs:1-19) -> Swift: NOT WRITTEN by the lane; still `.animation(Motion.layout, value: panes)` on the whole layout (PanesUI/Panes/PaneArea.swift:49). The wave-6 note says so itself.
- Closed in this row: the pane header's tile (PaneArea.swift:73 -> PaneHeader.swift:31), the keys in the pane menu and tooltips (WindowHub+PaneKeys.swift via TerminalPanes.swift:18).

W03 (95)
- The counting, longest-waiting rule, width classes and tooltip are still a Swift copy of core's `layout::status_bar` (StatusSummary.swift). Same rules as read today; no behaviour gap found. Drift risk only.

W05 (95)
- "Open Window in <profile>" for the profile Wormhole is in calls `openNewWindow()` (Settings/ProfileChipBinding.swift:130), which now plans in the FRONT window's profile. Front window of profile B, choose "A (shown)": a B window opens. GPUI opens in the chosen profile. Side effect of the Cmd-N fix.

W06 (95)
- A moved window's hosted desk and the Access/Trust/Imports desks stay on the old profile (ProfileChipBinding.swift `move`); the lane left it.

W12 (90)
- A terminal of a B window has A's colours for colour queries until its view dresses it, and a B terminal with no view hears A's theme changes (wave 5 "not written", untouched).
- Closed: Cmd-N, status bar in the window's own profile, Notes preview font (NotePreview.swift uses `@Environment(\.tokens)`, lines 18, 43, 49).

#### Closed, counted per row

- S01: 2 of 2 (enter on a folded selection, SidebarView.swift:138; RenameField.font is already `Font?`, so NOT REAL was right)
- S02: 1 of 1
- S03: 2 of 2 (alert with box and leaving count, Cancel gone, closing is Not Now; asked from every path), plus the two new notes above
- S04 to S06: nothing listed, 100 kept
- S07: 1 of 1 (held part: DropLines.over(at:held:part:) -> core `dropPart`; ffi drop_part.rs; only with Motion on)
- S08: 5 of 5 (room, group-card pose, Classic without the feature, split bracket, rolling counts); the pose in a card is the unverifiable one
- S09: 3 of 4 (alert source, tile in search hits, click focuses the terminal; the three + paths are the S03 fix); the TabSets copy remains
- S10: 1 of 1 (OnboardingWindow.onEnded -> showNoAgentsTip, AuxController.swift:109)
- W01: 2 of 3
- W02: 7 of 7 (state words with time and exit code; hidden lock; "in <parent>"; detached chip; agent tile; file drop on the row; Insert off only for exited)
- W03: 4 of 4 (status_bar written to the window's profile and read back; exit code; key in tooltip; tile)
- W05: 1 of 1 (Cmd-N in the front window's profile)
- W06: 0 of 1 (left)
- W08: 1 of 1 (`state.all`)
- W09: 2 of 2 (window name follows the selection; window animation from the core's rule)
- W11: 1 of 1 (HookHost one-minute timer -> reloadProbes)
- W12: 3 of 4 (Cmd-N, status bar, notes font; colours of a B terminal open)

#### Your four questions

1. Group folder question: all three callers (SidebarView+Group.swift:30, SidebarContext+Away.swift:28, WindowHub+Sidebar.swift:144) run `settle` and then send `.newTerminalInGroup` once, in the same Task. Offer nil means no question and one terminal; one answer means one terminal; `decline` is remembered by the core. No path opens two or none, except the missing-group case above (none). Tab bar +, the empty pinned set (openSet) and Ctrl-Opt-N (through openSet) all reach `newTerminal(inGroup:)`. Leaving count comes from the core (`members_leaving`, ffi workspace.rs:454); the box sends `.setOnlyInFolder`, which the core applies with `keep_to_folder`.
2. Title facts: every one reaches the view. hidden: `terminal.isAgentHidden` (core agentHidden) -> TitleFactsMaker:23 -> TerminalCrumb. superproject and detached: core `TitleTerminal` (title.rs, from `GitFacts.superproject`, filled by `git_feed` from `git::superproject`) -> TitleMapping:35 -> TitleFactsMaker:21,27-30 -> BranchCrumb:35,36. State words: StateWords.swift with the core's `waited(since:)` and `failureCode`, one-shot timer in TerminalColumn.swift CentreTitle; only while the status bar is off. Tiles: WindowHub+Tiles.swift reaches title, pane header, status bar attention button. `TerminalFactsRecord` in the ffi still sets `superproject: None`, but that is the test-seeding record, not the live path.
3. View > Show Status Bar: ActionRouter+View.swift:25 writes `status_bar` through `settingsBackend(of: front)`, the window's profile backend; `RustSettingsBackend.set` reloads and observers re-apply (LookSettings.swift:37), so Settings follows and it persists. Cmd-N: `front.windows` plans and makes the session (AppController+WindowOpening.swift:28-36, makeSession `in:`). Only the profile-chip path above regressed.
4. Room: the dropped row's landing is still the core's `dropPlan` (SidebarView+Drag.swift `aim`); ListRoom takes the slot from that plan, and the pushes from the core's `make_room` (ffi room_pushes -> make_room). Swift geometry only chooses which line the pointer is over (as before, and now with the core's `drop_part_at` holding the edge). Lines are offset before they are measured, so the frames the drop uses do not move. Looks right in code.

#### New

- openWindow(inProfile:) for the shown profile (W05 above).
- group-gone case (S03 above).
- `paneActions(appearance:)` (WindowHub+Tiles.swift) and `paneActions(keys:appearance:)` (WindowHub+PaneKeys.swift:12) share a base name and the second calls the first without `keys`: it compiles to the first only because Swift prefers the overload with no defaulted argument. Fragile; one rename away from infinite recursion. The only caller passes `keys:` (TerminalPanes.swift:18).
- Everything written without compiling per the lane's own note; the title drop tint uses `controlAccentColor` (a look matter, not counted).

## Settings and Theme Studio

id,written
ST01,100
ST02,90
ST03,85
ST09,95
STX1,100
STX2,95
STX3,100
SD01,85
SD02,90
SD03,100

(Components page, the last section of audit 3 "Coverage": CLOSED, see STX3.)
Read only, nothing built or run. Swift paths under macos/swift/Sources/, bridge under crates/wormhole-ffi/src/.

#### Rows below 100

##### ST02 (90)
- Opacity slider is switched off, with the note, while Reduce Transparency or Increase Contrast makes windows opaque (`sections/appearance.rs:171-192`) -> Swift: PARTLY. The note is there (`SettingsUI/Rows/RowExtras.swift` `OpaqueNote`, wired by `EntryRowView` `below:`), but the slider is still always live: `EntryControl.swift` `.slider` has no `.disabled`.
- Default shell: each choice says what it runs ("Login shell: /bin/zsh") (`terminal.rs:293-345`) -> Swift: PARTLY. "Not found: the login shell runs instead" is there (`RowCaptions.gone`, `OptionPicker.unknown`), and the Shells list shows command lines (`EntryRowView.swift` `ShellLines`), but the picker's choices are names only (`settings/host.rs:52-58` drops the command line).
- Fonts specimen follows the slot being picked, with that slot's size and line height (`sections/fonts.rs:79-90`) -> Swift: PARTLY. Tabs Terminal | Code | Interface and a facts line now exist (`SettingsUI/Preview/FontsScene.swift`), but a hovered font in an open picker replaces family and size whatever tab is shown (`model.fontPreview`, `FontsScene.swift:19-20`); the picker does not switch the tab.
- Closed in ST02 (10 of 13): Custom branch colour swatches writing `branch_color_custom` (`RowExtras.BranchSwatches`), blur warning with "Turn on blur" (`BlurWarning`, core's `settings_blur_unsafe`), Liquid Glass Beta chip and note, primary colour description and theme swatches (`RowCaptions.primary`, `PrimarySwatches`), terminal font caption, Open in "Not installed" note, default terminal tooltips, Custom fonts line, title row notes and core's `show_title_part` (`settings/page_rules.rs`), hand-written scrollback label.

##### ST03 (85)
- Permission row follows macOS state by state (`notifications.rs:112-184`) -> Swift: PARTLY. All six states and Allow / Open System Settings are written (`SettingsUI/Panels/NotifyControl.swift`, `Wormhole/Notices/NoticeFeed+Test.swift` `publishStatus`). But after "Allow notifications" `SettingsPlatform.perform(.askNotifications)` calls `askPermission()` (async, fire and forget) and then `publishStatus()` at once, which still reads "not asked"; only `.checking` re-polls, and `refreshNotify` runs on page appear. The row stays on "Not asked yet" with the Allow button after the person has answered, until the page is reopened. Same for a change made in System Settings while the page is up.
- Test row shows refused with the error (`test_banner.rs` `Test::Refused`, `posted`) -> Swift: OPEN for that state. `NotifyStatus.Test.refused` exists and is worded, but nothing sets it: `SystemNotifier.post` (`PlatformUI/Notifications/SystemNotifier.swift`) only logs the failure, and `finishTest` sets `.sent` unconditionally. Waiting, sent, denied, unanswered are written and reachable (`NoticeFeed+Test.swift` `runTest`). The lane lists this as NOT DONE; true.
- Per-profile agent logins (`sections/agents.rs:72-88`) -> Swift: CLOSED for an isolated instance, but `settings_agent_logins` calls `isolation::require()` and the view reads it with `try?` (`RustSettingsHost.swift:172`), so in any run that is not an isolated `Wormhole-swift*` data folder the logins list is empty and `settings_set_login_hooks` refuses. By the owner's "running as the product is a later step", not counted.
- Closed in ST03 (6 of 8): the master-switch rule, "A terminal needs input" Time Sensitive chip and plain description, the approved programs' ticks, "Looking for agents...", Reset tips tooltip, logins line with Install/Repair/Remove.

##### ST09 (95)
- Specimen follows the slot being picked (`fonts.rs:79-90`) -> Swift: PARTLY, as under ST02 (tabs exist now, the picker does not drive them). 0 of 1 fully closed.

##### STX2 (95)
- The page reads the project's files and trust again about once a second (`sections/projects.rs:44-46`) -> Swift: PARTLY by design. It reads on open, after its buttons and whenever the window comes forward (`SettingsUI/Projects/ProjectsPanel.swift` `appearsActive`). A project edited on disk while the page stays in front is stale. Return in the search takes the first match: CLOSED (`onSubmit`). 1 of 2 closed, the other by the events rule.

##### SD01 (85)
- Studio reads and writes the active profile's themes folder (`studio/themes.rs:134`) -> Swift: CLOSED. `StudioLauncher.themesFolder` is a closure asked at each open (`Wormhole/Platform/StudioLauncher.swift:54`), and `AppController.swift:84` returns `themes.themesDirectory`, which `ProfileLooks.swift:28` and `AppController+Look.swift:29` set to the active profile's folder. Caveat: an open Studio keeps its folder on a profile switch (the lane says so); GPUI reads it per call.
- Metric editor preview of what the metric shapes and its use count (`studio/token_editor.rs:274-355`) -> Swift: OPEN (`StudioUI/Editor/MetricEditor.swift` unchanged).
- Expression completions offer functions (`mix(`, `readable(`) from core (`studio/completion.rs`) -> Swift: OPEN, token names only (`StudioModel/Document/StudioDocument+List.swift:77-79`).
- Contrast fixes from core, following named tokens and spotting cycles -> Swift: OPEN, Swift's own simpler `ContrastFix.swift`.
- Appearance chip on every row of a pair, click moves the value (`origin_tag.rs:48-52`) -> Swift: OPEN, header chip for the selected token only.
- Theme reading and saved versions through core -> Swift: OPEN, `DiskThemeFolder` and `VersionPolicy` stay Swift copies.

##### SD02 (90)
Wave 6 touched only `ControlsSpecimen.swift` and `GalleryUses.swift` (a line). All four audit-3 gaps are OPEN as the lane says: half-turned status tiles, tooltip/dialog uses, core's `TOKEN_USES` (Swift's generated `GalleryUses.swift` instead), lazy gallery.

#### Counts (gaps of audit 3 CLOSED out of total)
- ST01: 4 of 4 (nothing-found card `ResultsPage.NothingMatches`; picking a section keeps the marks, `SettingsModel.browsing` and `SettingsPage` `marks`; nav warning dot `SettingsNav.attention`; project-set row chip and held-back swatches, `EntryRowView`).
- ST02: 10 of 13 (3 PARTLY, above).
- ST03: 6 of 8 (permission row PARTLY, test refused OPEN).
- ST09: 0 of 1 (PARTLY).
- STX1: 1 of 1 (`ProbesModel.pending`, `ProbeRowView` `.disabled`).
- STX2: 1 of 2 (Return closed; the one-second read is replaced by a read on coming forward).
- STX3: 2 of 2 (Components page `SettingsUI/Pages/ComponentsPage.swift` reached by `SettingsModel.showsComponents` from core's `visible_sections`, nav and `SettingsView.swift:69`; skill rows "Looking" and the agent tile `SkillList.swift`). The Components page is a developer page drawn from the Swift controls on sample state, not a copy of GPUI's gallery of settings controls; counted as closed.
- SD01: 1 of 6 (the folder). The other five were declared NOT DONE and are.
- SD02: 0 of 4.
- SD03: 7 of 7 (shelves by mode `shelves`; newer-file mark `ThemeCard.newer` from `theme_resolve.rs`; project notice with Project settings from `ProjectSettingsFollower`; hide by core's rule `theme_picks.rs`; no-match line and counted placeholder; arrow/Home/End walk; Dark/Light beside hidden names).

#### The four checks
1. Master switch: with `notifications: false` every kind reads off (`settings/state.rs:61-69`, `notify_kind_on`); turning a kind on writes that key and then `change_notify_kind` recomputes `notifications` from all five kinds (`state.rs:86-90`), the same as GPUI's `set_kind` (`notifications.rs:103`). A kind stored true while the master was off lights up again when another is turned on, as in GPUI. Only gap: the per-row reset arrow (`State::reset`) does not go through the rule, so resetting one kind with the master off leaves the master off (GPUI's reset does likewise unless it goes through `set_kind`; not checked). "Send test": waiting, sent, denied, unanswered written; refused not. A test instance cannot reach the system: `NoticeRoute.choose` returns `.log` whenever `policy.background` unless `WORMHOLE_NOTIFY` is set to something other than `log` and the process is a bundle; then it posts for real (same as GPUI's route). In the log notifier `permission` is `.unavailable`, shown as "Outside the app bundle", and Send test goes straight to "Sent" (GPUI does the same for OutsideBundle).
2. Capability ticks: `set_capability` has one caller path, `ApprovalList.swift:33` -> `RustSettingsHost.setCapability` -> `SettingsPlatform.setApprovalCapability` -> `HookServerHandle`. No control-socket or bridge verb reaches it; a terminal program cannot turn one on. Turning one off calls `bus.revoke_holder(seat.holder(stored, id))` (`hook_server/mod.rs:196-199`), which ends every stream that approval let in (broader than the one capability, the safe side). Stored caps are read at decision time (`stored_caps`), so the change bites at the next request. Turning ON ends nothing and widens only what the person ticked. An unknown id or Close returns false. CLOSED and sound.
3. Agent logins: writes only under `WORMHOLE_AGENT_HOME` (`agent_logins.rs:90-96`, `agents.rs:126`); with it unset the call is refused, with it equal to the real home refused. A test instance cannot write a real `~/.claude`/`~/.codex` by that route. Hardening gaps: the check is `Path::starts_with` on a non-canonical path, so a profile login dir written as `<AGENT_HOME>/../../.claude` or reaching through a symlink passes (the dir comes from the profile's own config, `login_dirs` keeps it verbatim); and an `AGENT_HOME` set to a parent of the real home passes the "not the home" filter. Both need the person's own config or environment, so low.
4. Studio folder: follows the active profile (above), asked at each open; not after a switch while open.

#### New
- The "Allow notifications" press leaves the page stale (ST03 above). Half-wired: `publishStatus` is called before the answer.
- `OpaqueNote` explains a switched-off slider that is not switched off.
- No file seen removed or broken in the larger wave-6 changes I sampled (`RowExtras`, `RowCaptions`, `ThemeCards`, `SettingsNav`, `ResultsPage`, `ComponentsPage`, `agent_logins`, `approvals`). The ThemeCards arrow walk relies on the card buttons being focusable (keyboard navigation on); with it off Tab and arrows do nothing, which is the system's rule.

## Secondary windows, overlays, platform

id,written
A01,90
A02,100
A03,95
A04,95
A05,95
A06,100
A07,95
O01,90
O02,100
O03,90
O04,90
O05,90
PL01,90
PL02,95
PL03,85
PL04,95
PL05,100
PL06,100
PL07,100
PL09,100

#### Gaps still open (per row, most important first)

##### A01 (90)
- The trust card follows the folder pick, shows "Waiting for approval…" while Touch ID is up, and is gone once the folder is trusted (`agents_step.rs:159-230`, `onboarding/mod.rs:201-212`) → Swift: the card now appears (`OnboardingBridge.swift:79-88` `trustCard(for:)` builds a `TrustDesk` for the picked folder, else `frontProject`), but `card()` (l.197-201) never sets `approving`, the card is cached per root and never recomputed after `approveReviewed()`, and `hub` (WakeHub) is not observed by the session. After a successful approval the button stays and "Waiting…" never shows (`TrustCardView.swift:26`). The Touch ID reason is worded with the folder `?? "/"` when none was picked (`AgentsStep.swift:36`). The desk is made with `profile: nil` (the window's profile is not passed), GPUI asks as the window's profile.

##### A03 (95)
- A development build says "This is a development build; it doesn't update itself." (`update/mod.rs:307-311`) → Swift: still the "no feed" words (`AuxBackends.swift` `SampleUpdateBackend(phase: .unsupported(AuxText.noUpdateFeed))`). Also Check for Updates… opens its own Updates window, not Settings > Updates; the check does start now (`AuxController.swift:60-64`).

##### A04 (95)
- An unavailable host's card carries the error's detail → Swift: `unavailableDetail: nil` still (`AuxBackends.swift` in `hostAttachments`); the lane said NOT WRITTEN and it is.

##### A05 (95)
- The mode is settled for every profile on the app's tick (`ask.rs:tick` over all windows' profiles) → Swift: `Core::settings_changed` runs only when the settings applier sees a change of the SHOWN profile (`Settings/SettingsApplier.swift:81` passes `app.windowsHost.profile`); a file edit of another profile's `config.json` (Off or Open) is not settled until something else asks. Within the shown profile the port is right (see below).

##### A07 (95)
- Same: the keep-it question for a widening made outside the app appears at once only for the shown profile (`SettingsApplier.swift:81`); another profile's waits for its first request (`core.rs` `Seat::settings`).

##### O01 (90)
- Commits are the log of HEAD of the palette's terminal, read when the palette opens (`command_palette/mod.rs:304-324`) → Swift: with History off the list is now empty (`PaletteSourcing.swift:22-26`), but it is still the History tab's model (`session.hub.panel.history.model.commits`), so it follows the tab's rev and path filter.
- Recent folders are the profile's list in its database (`app_state/recent_projects.rs`) → Swift: one `recent-projects.json` for the app (`Menus/RecentProjects.swift:16`); NOT WRITTEN, still so.

##### O03 (90)
- Recent folders per profile (`app_state::recent_folders`) → Swift: one store (`FolderPicking.swift` `PlatformBridge.storeRecents`); not written.

##### O04 (90)
- Not re-read item by item. Group colour panel exists (`Menus/GroupMenuKeyboard.swift:74`, `GroupColourPanel.swift`); the pane header's keys now come from the keymap per commit `f2d728043` (not opened by me).

##### O05 (90)
- `HistoryKeptByHost`: the poster exists (`TerminalOverlay+Clipboard.swift:23`) but nothing on the host side fires it (lane: not built); I did not find a caller either.

##### PL01 (90)
- Undo/Redo titles of a window with its own history ("Undo Change “primary”", `window/menu_state.rs:36-52`) → Swift: `MenuBuilder.swift:46-47` reads `state.undo/redo`, nothing sets them (grep of `Wormhole/`, `WindowUI/Menu`, `MenuStateMaker`): OPEN, NOT WRITTEN.
- Open Recent is the profile's list (`recent_projects.rs`) → Swift: app-wide file (see O01): OPEN.
- Show Project is greyed also when project folders are off (`features/project/def.rs:101`) → Swift: only when the terminal has no folder (`WindowHub+Menu.swift:21`); with the feature off the action is hidden anyway by `featuresOff`, so the visible difference is small.
- ⌥⌘↑ / ⌥⌘↓ are both Select Pane Above/Below (menu first keys, `table.rs:162-163`) and Move Item Up/Down (list scope, `table.rs:242-243`). The notes rows take the second through SwiftUI `onKeyPress` (`ReorderDrop.swift:61`), which runs after the menu's key equivalents; in a split (the pane items are enabled) the menu wins and a note cannot be moved by key. Read from the code, not run.

##### PL02 (95)
- `ExtraKeys` handles only windows of the workspace (`ExtraKeys.swift:25`); with Settings, the Theme Studio or a sheet key, ⇧⌘N, ⇧⌘], ⇧⌘[, ⌘=, ⌃⌥digit do nothing (GPUI's app-level bindings work from any window). Small.

##### PL03 (85)
- A terminal's banner is taken down when the person looks at it (`Event::Seen`: selected, key window, app active; `watch.rs:150`) → Swift: the condition is right now (`NoticeFeed.swift:67-70`: only when `NSApp.isActive` and the front window is key), but `follow()` runs only when `app.noticeTabs` (an @Observable read) changes. Nothing observes the app becoming active or a window becoming key (no `didBecomeActive` / `didBecomeKey` in `NoticeFeed`; the comment's "the look that follows the window's focus" does not exist). A banner posted in the background for the selected terminal stays until some terminal's state changes after the person has come back.
- An event is announced by the settings of the profile its terminal is in (`notifications/mod.rs:191-196`) → Swift: `app.settingsNotice` (shown profile's); NOT WRITTEN.
- A dev build without a bundle posts through the platform's script → Swift: the log only (dev only).

##### PL04 (95)
- Not found open. `run(ambient:)` brings the app forward for New Terminal, Open Folder, Open Project (`AppController+Ambient.swift:18-21,34-36,66`); a terminal row and Jump go through `show(terminal:)` which now activates and orders the window front (`AppController+Windows.swift:142-146`); `pushStatus` sets `hotkey` and `jumpKey` (`AmbientController.swift:62-69`); Projects lists every shown-profile terminal's project (`ambientActions`). Left 95 only because `.reply` does not activate (GPUI also types without a window) and `jumpKey` is `Keymap.shared.key("goToWaiting")`, which I did not follow to a user-recorded key.

#### Closed, per row (audit 3 gaps found CLOSED / total)

- A01: 6 of 8 closed (recents `AuxBackends.swift:~157` passes `recents`; agents note `onPrimer` plus `onEnded`; first terminal replaced only `firstRun`, only when the window has one terminal and it is not already there (`AuxBackends.swift` finish closure); dropped file picks its folder (`FolderStep.swift:42-46`); notes fact `notesOn` reaches the primer (`PrimerStep.swift:28,36`); Help > Welcome replays (`AuxController.swift:103-107`)). Trust card PARTLY; the "steps' flow re-implemented" line is not a gap.
- A02: 1 of 1 (`Wormhole/Info.plist:82-83` has `NSHumanReadableCopyright`, read by `AppInfo.from(bundle:)`).
- A03: 1 of 2.
- A04: 1 of 2 (`hostAttachments` now walks `state.all`, folded and filtered rows included, `AuxBackends.swift` end).
- A05: 3 of 3. Resolver is now passed (`AuxBackends.swift:66-70,82-100` -> `makeAccessBackend(profile:resolver:)` -> `AccessBridge.swift:69,73`), `Workspace::terminal_running` exists (`workspace_access.rs:12`, uses `Peer::runs_in` as `terminal_of`); Scripts has no "For this session" (`AccessDisplay.swift:55`); the mode settle is a port of `settle_mode`: `core.rs:285-312` once per change of the mode in force (`applied`), Open -> `settle_questions(Open)` retries every waiting request, Off or Automation off -> every held and stored approval's streams revoked on the bus (`holder(false|true, id)`, same ids as `end_streams_under_approvals`) then every waiting question refused `agent_access_off` (`access/mod.rs:244-262`); `revoke_reads` for anything but Open. Held and stored approvals themselves stay in both apps; after Off the gate refuses on mode. Nothing left able to act that I found. The shown sheet goes because its question is taken from the queue (the driver checks `isWaiting`).
- A06: 1 of 1 (`ClosedProjectTrust` returned when `real, root == nil`, `AuxBackends.swift:60`; the caller `AuxController.swift:39`).
- A07: 2 of 3 (the keep-it question is noticed when `settingsChanged` is called, which `SettingsApplier.swift:80-81` does on a change of `agent_access`, `control_write`, flags; shown profile only).
- O01: 1 of 3 (hidden/greyed actions flow from `menuState(for:)` into the palette, `PaletteSourcing.swift:60,85-91`; History-off gate closed; HEAD log and per-profile recents open).
- O02: 1 of 1 (the sheet's `hidden` is `actionFacts(session).hidden`, which now holds the feature-off actions).
- O03: 1 of 3 (`follow()` walks every window's `state.all` including folded groups, `FolderPicking.swift:~108`); the rewrite of core's model is a design choice, per-profile recents open.
- O04: 1 of 1 checked.
- O05: 0 of 1.
- PL01: 5 of 7 (see the key table below; feature-off hiding; Show Usage hidden / Show Project greyed; status bar writes `status_bar` via `settingsBackend(of:).set`, `ActionRouter+View.swift:20-24`; Go to Waiting/Finished across every window with the longest-waiting order and the window brought forward, `AppController+Ambient.swift:40-47`). Undo/Redo titles and per-profile Open Recent open.
- PL02: 2 of 2.
- PL03: 3 of 5 (Time Sensitive: `NoticeFeed.timeSensitive` = `SystemNotifier.deliversTimeSensitive`, i.e. permission allowed with time-sensitive AND the entitlement, passed to `look` and `sendTest`, as `notifications::time_sensitive`; the seen condition; Send test reports waiting/sent/denied/unanswered via `runTest` and `NotifyStatus.test`). Per-profile announcement and dev script open.
- PL04: 4 of 4. PL05: 1 of 1. PL06: 1 of 1. PL07, PL09: skimmed, untouched.

#### The key monitor, entry by entry (`ExtraKeys.swift` over `table.rs`)

How it decides: a local keyDown monitor (started at `AppController.swift:150`), only for a window of the workspace, no repeats, only ⌘ or ⌃⌥ held with a one-character key. It takes the first LIVE entry holding the key (`sameKey`: US shifted characters canonicalised), only if its scope is App and the router implements it. If the pressed key is the FIRST key of an action some menu holds, it lets AppKit run the menu item (with enabled state). Otherwise it refuses what `menuState` hides or greys, else `router.run` and swallows the event.

- Road = menu item's key equivalent (first key; `NSMenuFactory.swift:66-75`): NewWindow, NewTerminal ⌘T, NewTerminalNoGroup, OpenFolder, CloseTerminal, CloseWindow, Next/PreviousTerminal ⌃⌘↓/↑, Next/PreviousGroup ⌃⌥]/[, SplitRight/Down, ZoomPane, FocusPane* (terminal scope), GoToWaiting/Finished, Minimize, ToggleSidebar/RightPanel/StatusBar/ZenMode, ShowChanges..ShowFiles, FocusTerminalOrPanel, FocusNext/PreviousArea (F6, ⇧F6, function-key equivalents), Bigger ⌘+, Smaller, ActualSize, FullScreen, CommandPalette, KeyboardShortcuts, Undo..SelectAll (AppKit selectors), ClearBuffer, Find family, Previous/NextPrompt, ShowHiddenLines, InsertFolderPath, NewNote, StashPrompt, Settings, ThemeStudio, Hide, HideOthers, Quit, Inspector (debug). Help ⌘? is the system's.
- Road = this monitor: Terminal(1..9) ⌘1-9 (the Window menu's terminal items are `.dynamic`, not counted as "in menus", so the monitor runs them once and swallows the event: not twice), Group(1..9) ⌃⌥1-9, PlayIntro* ⌃⌥⌘1-7, PlayWelcome* ⌃⌥⌘8,9,0 (handlers exist: `ActionRouter+Terminals.swift:32`, `ActionRouter+Edit.swift:24-29`; ids built from `IntroStyle.onTrial` and `Handover.allCases`, all seven and three present), second keys NewTerminal ⇧⌘N, NextTerminal ⇧⌘], PreviousTerminal ⇧⌘[, Bigger ⌘=.
- Road = the terminal view's own ⌘ list (`TerminalShortcuts.swift`, via `terminalAction` now wired at `TerminalLook.swift:123`): Copy/Paste/SelectAll/ClearBuffer, LineStart ⌘←, LineEnd ⌘→, DeleteLine ⌘⌫; ClearBuffer's menu item is not reached while the terminal has focus (one run).
- Road = a view: MoveItemUp/Down (notes rows `onKeyPress`, see PL01 conflict), RowMenu ⌃↩ (sidebar `SidebarView+Keys`), SaveTheme/DuplicateTheme (Studio), plain-key and Switch/OpenWindowIn/MoveWindowTo profile (menu/palette by index).
- Runs nothing: none found in the live table for a workspace window. In Settings, Studio and sheets only the menu bar's first keys work (PL02 note). Print ⌘P and Export Text ⌘S are `reserved` (not live) in both apps.
- Runs twice: none found. Menu first keys are passed to AppKit and the monitor swallows only what it runs.
- Runs an action the menus hide or grey: closed for what the monitor runs (it checks `state.hidden`/`state.disabled` of the window's own hub, `ExtraKeys.swift:36-38`); AppKit does not fire a hidden or greyed item.
- A key typed in a text field: the monitor looks only at ⌘ or ⌃⌥ presses of a one-character key, so ordinary typing, including Option-composed characters, never reaches it. In a sidebar rename, the palette's field or the notes editor inside a workspace window, ⌘= / ⇧⌘N / ⇧⌘[ ] / ⌘1-9 / ⌃⌥1-9 are taken as app actions (GPUI also binds them app-wide). Settings' search field is in another window: not taken. One thing differs from GPUI's focus rule: a key the table scopes to Terminal (⌘K, ⌘J, ⌥⌘arrows) still fires from its menu item when the focus is in a panel field, because the menu key equivalent is not scoped.

#### New (broken or half-wired by wave 6)

- Notice retraction has no trigger on app activation or key-window change (`NoticeFeed.follow`), see PL03; the comment at `NoticeFeed.swift:68-69` describes a mechanism that does not exist.
- The onboarding trust card never leaves after approval and has no waiting state (`OnboardingBridge.card`, `TrustCardView.swift:26` reads a flag nothing sets).
- ⌥⌘↑/↓ Move Item vs Select Pane conflict when split (PL01).
- `settings_changed` is only called for the shown profile (A05, A07).
- `RecentProjects.menuState(for:)` calls `recents.remember(root)` on every menu-state build (a write to disk from a getter that the palette and ExtraKeys also call; `RecentProjects.swift:~44`). Harmless (no-op when the first entry is unchanged) but it is a side effect in a read.

## Crosscut (X01 to X15)

id,written
X01,90
X02,95
X03,95
X04,90
X05,90
X06,95
X07,95
X08,95
X09,90
X10,85
X11,100
X12,90
X13,95
X14,90
X15,90

Method: read-only. Ran `python3 -I macos/scripts/words.py` (the only thing run). Everything else read in the tree.
Paths: GPUI under crates/wormhole/src, Swift under macos/swift/Sources, bridge under crates/wormhole-ffi/src.

#### Gaps still open, per row (most important first)

X12 (Dev tooling)
- `front` really brings the window forward (devctl/mod.rs header) -> Swift: not written on purpose (background rule, Driving/InFront.swift); `window-front` only sets the focus-ring flag (AppController+Look.swift:63). OPEN, a decision, not a bug.
- `paints`, `refresh`, `scaling` (+`settle`, `reveal-test`), `grow-hold`, `scroller`, `notice-material`, `settings-preview`: no Swift command at all (grep of every quoted name in Sources/Wormhole finds none). TO-VERIFY lists them as "no Swift counterpart"; for `scaling`/`settle`/`reveal-test`/`grow-hold` this is a real tool gap (Swift `geometry` only prints), the rest are GPUI mechanisms. OPEN.
- `new-shell <key> [dir]` takes a key only (AppController+ControlRead.swift:49-55, the folder is dropped). `new` with a folder works (ControlRead.swift:44). PARTLY.
- `pointer` still means ROW COL, not a point in points (AppController+Control.swift:49); `hosts` still opens the window, `hosts-list` prints; `notices` keeps another meaning. Deliberately kept, scripts written for GPUI will misread. OPEN.
- `a11y` still dumps AppKit's part of the tree only; `a11y live` not found. OPEN.
- The control folder is compiled into every build and switched on by `WORMHOLE_DEVCTL` alone (AppController+Open.swift:30-35; DevCtl.swift); GPUI has it only in `--features devctl` builds. Today harmless (the app refuses to start outside a `Wormhole-swift*` data folder, Isolation.swift:26-33), but when the isolation is lifted this is a hole: see New.

X04 (Keyboard navigation)
- Esc's popup rung: a menu, popover or rename field closes first and the keys go back to its row (core view/focus/escape.rs) -> Swift: `popup: false` still passed (WindowUI/Focus/WindowFocus.swift:206) and `.closePopup` just returns false (:207). Unchanged by wave 6. OPEN.

X10 (Performance guards)
- GPUI's seven counting guards (terminal_view/render/guards.rs:129-235) -> Swift: four `@Test`s in FrameWorkGuardTests.swift (lines 32, 42, 48, 57) plus the renderer counters; wave 6 did not touch the tests or perf-guards.sh. Blink phase / "no clock without blinking text" not found there (FindAndBlinkTests.swift exists, not read). OPEN.
- `scripts/perf.sh` baseline per machine that fails on a regression -> Swift: perf-guards.sh only runs the tests, perf-compare.sh compares with GPUI; no stored Swift baseline. OPEN.

X05 (Reduce Motion / Transparency / Contrast): nothing re-read; wave 6 did not list a change. The "each Reduce Motion consumer against GPUI's list" check was never done. Not judged, kept at 90.

X09 (Events): Notes sync is still a repeating `Timer` (Wormhole/Panel/NotesCoordinator.swift:100-104), not compared with GPUI's features/project/notes_sync.rs. Not judged, kept at 90.

X01 (Design language): tile and field components still two each (SidebarUI/PanesUI tiles, PanelField/PaletteField); the per-state read of ~80 `style/` files was not done. Status bar count roll is CLOSED (see below).

X14, X15, X13, X02, X06: no wave-6 change found in these; the audit-3 caveats (version-1 theme files in the Studio, light/dark icon theme family, legacy upgrades never running in a `Wormhole-swift*` instance, one app-wide project theme) stand.

X08 (CLI and control socket), what is left
- `Wormhole --json list` / a leading dash: main.swift:13 still gives the line to core only when the first argument has no leading dash. Unchanged. OPEN (small).
- `new-window` when the workspace is not several windows answers `unsupported` (hook_server/control/structure.rs:53-57), a case GPUI does not have. Its model is now the front window (`plan_like(Some(&core.front_share()))`, :59). PARTLY.
- `events`, `read`, `blocks`, `wait`, `run`, `mouse`, `send-keys` flag by flag and the marking-shell "run shows output only the first time" fixture: not read by me either.

X07 (Words), what is left
- words.py reads clean (0 literals, 0 unknown ids, 0 mismatches), SKIP_PATH is now only `AppController+Control`, `StudioLauncher+Control`, `ControlSpec`, Mock, Sample, Fixture, DevCtl, Driving, FramesRun, RenderPNG, Isolation, Options*, UniFFISource, WormholeBindings (words.py:36). The allowlist is empty (words-allow.json). Blind spots remain in the heuristic, small: a lone lowercase word is not "text" (`Text("esc")` SettingsUI/Pages/ResultsPage.swift:40, `.accessibilityLabel("changed")` StudioUI/List/BandView.swift:70), and any line containing `log...(`, `assert`, `#if`, `case "` or `fallback:` is skipped whole (words.py:38-42, 148). Neither hides a sentence I could find.

#### Per row: how many of audit 3's gaps found CLOSED

- X01: 1 of 3 closed (roll in status bar: StatusBar.swift:126-133 `statusRoll`, used for both word and dot forms); tile/field and per-state read remain.
- X02: 0 of 0 counted gaps (one question, unchanged).
- X03: 4 of 5 closed. Tab speech: WindowUI/Hub/WindowHub+TabSpeech.swift:8 -> FilesBridge.swift:47 `panelTabSpoken` (panel_spoken.rs:9), used by RightPanel.swift:26 and PanelTabBar.swift:35. Status bar: WindowBody.swift:169-170 assigns `spokenAttention`/`spokenCount` to RustWorkspaceBackend+Hosting.swift:190, 201 (workspace_spoken.rs:14, 28), read at StatusBar.swift:74, 83. Zero-count bands: NotesCard.swift:24, StepsCard.swift:24, HistoryView.swift:59, ChangesListView.swift:49, with ids in swift-panel.ftl. Settings "Copied": ChangedPage.swift:35-36 (posted twice, a duplicate line). Not closable by reading: what VoiceOver says for each SwiftUI control (still never run).
- X04: 0 of 1 (popup rung).
- X05: 0 of 0 counted (nothing re-checked).
- X06: 0 of 0.
- X07: 4 of 4 closed (TerminalMenu through `Words`: PlatformModel/Context/TerminalMenu.swift:41-44; close questions AuxText+Questions.swift:9-32 used at AppController+Quit.swift:113, 126, 159, 175; WorktreeClose; WindowControls.swift:45-49; open panel `AuxText.openFolderPrompt` AppController+Ambient.swift:58; skip pattern narrowed). The three "smaller" ones (esc, changed, fix/login) are still literals but defensible: GPUI's placeholder is also the literal "fix/login" (features/git/worktree/ask.rs:150).
- X08: 4 of 5 closed, 1 partly. `split --size`: open.rs:125-133 `set_split_share` (core workspace/splits.rs:105) and reply `size`. `new-terminal` front window: open.rs:54-56 `front_share()` with `set_front_window` fed from `didBecomeKey` (Hooks/HookHost.swift:38-46). `--worktree --command`: worktree.rs:33-61 puts the question up with the command, HookHost.swift:101-104 -> WindowHub.askWorktree -> WorktreeStore.swift:97, 131 -> RealWorktreeBackend.swift:50-55 -> PanelBackendsFactory.swift:69-81 pastes `setup && command` into the new terminal (so TO-VERIFY's "NOT BUILT" is out of date, it is built). `split` no room before the budget: open.rs:110-113 `split_fits`, budget after, :125 fallback. `Wormhole` leading-flag and `new-window` case remain.
- X09: 0 of 0 counted.
- X10: 0 of 2.
- X11: 2 of 2 (AboutPanel.swift:15-20 `TestInstance.isBackground` -> orderBack; GroupColourPanel.swift:27 `TestInstance.bring`).
- X12: about 100 of about 112 listed no-counterpart commands now have a Swift command (my grep of each name in audit 3's list: all found except `front`, `grow-hold`, `notice-material`, `paints`, `refresh`, `reveal-test`, `scaling`, `settle`, `scroller`, `settings-preview`). Keys to a second window are closed: DevCtl.swift:46-55 reads `cmd<n>` into per-window queues, `front` honours `commandWindow` (AppController.swift:131-136), `reply` writes `out<n>.txt` (AppController+Control.swift:84). `delay` now holds the batch (ControlFlow.swift:15-19, `pause`), `sync` and `control` exist. `close <n>` now closes row n without the question (ControlRead.swift:56-60).
- X13, X14, X15: 0 of 0 counted.

##### X12 check 1: ten new Swift commands, read to the action
1. `layout columns|rows|grid|single` -> `hub.send(.applyLayout(preset))` (ControlRead.swift:26-33), the View > Layout action. Window: `hub` is `front.hub`, `front` follows `commandWindow`. OK.
2. `new-shell <key>` -> `hub.send(.newTerminalWithShell(...))` (:49-55), the menu's action. Window ok. Folder argument dropped.
3. `new <dir>` -> `.newTerminalIn(dir)` (:44), the same action a worktree's terminal uses. OK.
4. `close <n>` -> `finishClosing(id, in: session(holding: id) ?? front)` (:56-60), the closing the question ends in; skips the question as GPUI's `close <n>` does. Reaches the window that holds the row, not the commanded one, fine.
5. `agent-hide <n> [on|off]` -> `.setAgentHidden` (:61-68). OK.
6. `zen` -> `setZen`/`toggleZen` (:36-45), the app's own. OK.
7. `files-pin` / `files-follow` / `files-root` / `files-rename` -> FilesStore `setPinned`, `followTerminal`, `choose`, `startRename`+`commitRename` (ControlFiles.swift:25-58), the store the views use. OK (the rename writes to disk).
8. `rename-edit <n>` -> `hub.sidebarLink.ask(.rename(.terminal(id)))` (ControlSidebar.swift:33-38), the menu's path. OK.
9. `profile-open|move|delete` -> `openWindow(inProfile:)`, `move(front, toProfile:)`, `host.removeProfile` (ControlProfiles.swift:26-34), the same calls Settings makes. OK.
10. `notice-click` -> `chip.dismiss()`, `clickInfo()`, `clickBody()` (ControlNotices.swift:19-30), the chip's own handlers. OK.
Also read: `drag`/`drag-end` (ControlDrag.swift) builds an `NSDraggingInfo` stand-in and calls `draggingEntered` on the registered view the drop lands on, a real path through the view; `close-window` -> `window.performClose` (:69-74), the red button. `group-ask` answers a held question (PendingQuestions) rather than driving a GPUI-style sheet: same effect, test instances only. None of the ten has logic of its own beyond argument parsing.

##### Check 2 (X08): done above under X08.

##### Check 3 (X07): marks
40 files carry `// text-file:`, 343 lines `// text:`. Opened: AppController+Look.swift, +Workspace.swift, +Windows.swift, +Idle.swift (all control replies and idle report; the only person-facing line is `onOpenLogs`, which opens a folder, no text), InspectNote/Inspector (developer panel), PendingQuestions, SystemOpenIn (brand names), FontPickerSpecimen, Gallery specimens. Nothing a person reads is hidden. Risk, not a gap: AppController+Windows.swift is a large app-window file whose whole-file mark would hide any interface text added to it later; a per-line `// text:` would be safer. SettingsUI/Pages/ComponentsPage.swift (a Components page with English captions marked `// text: a developer page's caption`) exists, which contradicts audit 3's "Settings > Components is missing"; not my row.

#### Logging (checked with extra care): CLOSED, with three small notes
Against crates/wormhole/src/app/logging.rs the bridge file (ffi logging.rs) is a line-for-line copy: `EnvFilter::try_from_env("WORMHOLE_LOG")` default info (:51), stderr only when a terminal (:52), `ensure_private_dir` on `<data>/logs` (:57), `Rotation::DAILY`, prefix `wormhole`, suffix `log`, `max_log_files(7)` (:60-65), non-blocking writer with guard in a `OnceLock` (:17, 28), panic hook writing `logs/last-panic.txt` with `write_atomic` before the default hook (:75-95), same `panic_report`/`panic_message`. `data_dir()` is core's, which honours `WORMHOLE_DATA_DIR` (storage/paths.rs:87), so a test instance logs in its own folder. Called once: LaunchLogging.start() at AppController.swift:120 (inside init, after `Isolation.require()` in main.swift:25), `loggingStart()` and `makeDataDirPrivate()` (LaunchLogging.swift:15-16). The subscriber is `try_init`, a second install is ignored, and the hook sits behind the same OnceLock.
Notes:
- Swift's own lines never reach the log: `LaunchLogging.log` / `log_line` has no caller anywhere in Sources (grep). The "data folder cannot be opened" log half (GPUI app_state/data_folder.rs:27-29: `tracing::error!` + `eprintln!`) is therefore still only the notice in Swift (Wormhole/Notices/NoticeReason.swift:14), unless the core logs it itself. PARTLY, small.
- The guard lives in a static, which Rust never drops: a quit that ends in `exit()` can lose the last buffered lines (GPUI keeps the guard in `main` and has the same risk if it quits with `process::exit`; not verified).
- Settings > Advanced still says "Set WORMHOLE_LOG=debug for more detail" (SettingsModel/Resources/settings.json:2525); true now. Help > Show Logs: AppController+Open.swift:61-65 creates and reveals `<data>/logs`, which now holds a file. Not run; the bridge's Cargo.toml has `tracing-subscriber` and `tracing-appender` (Cargo.toml:25-26) and Cargo.lock lists tracing; whether it builds I did not test.
- The data folder owner-only step: `make_data_dir_private()` (ffi logging.rs:112-116) spawns the core's `storage::store::make_data_dir_private` on a thread named `private-data`, exactly as main.rs:93-95 does. CLOSED. Difference: in GPUI it runs after the legacy-data move; in Swift concurrently with logging start (harmless).

#### `control_line` / `welcome_line` (checked with extra care)
- Who can call: only Swift code, `HookServerHandle.controlLine` and `welcomeLine` (HookBridge/HookServerHandle.swift:43, 49). Callers: `runFlow`'s `control` (Wormhole/AppController+ControlFlow.swift:27-41) and `welcome-probe` (AppController+ControlPages.swift:54, 101-107). Both run only from control-folder lines: DevCtl.drain -> run(line:) -> ControlTooling.swift:24 / ControlPages. Nothing on the socket, in a menu, in a hook or in the agent API calls them. They are uniffi exports on a `HookServer` object, so only code linked into the process can.
- What reaches the control folder: `WORMHOLE_DEVCTL=<dir>` in the app's environment (Options+Parse.swift:42), folder created 0700 (DevCtl.swift:25-26), and, today, only in a process that passed `Isolation.require()` (bundle id `app.getwormhole.terminal.swift` and a data folder named `Wormhole-swift*`, Isolation.swift:20-37). So nothing outside the test control folder reaches them; a caller needs the same user to write that 0700 folder.
- What they bypass: the peer. `Peer::default()` (no pid, no ancestors, no tty; control_line.rs:48) is the same value GPUI's devctl uses (devctl/control.rs:35, welcome.rs:57). The request still goes through `answer` -> `gate` (control/mod.rs:57-60, gate.rs:61), so access mode (off/ask/open), `control_write`, budgets, the Automation flag and scope all apply; with no pid there is no identity to resolve, so a request that would need a question is "not carried here: the request waits for the person's approval" (control_line.rs:52). A `--from <tab>` is only a claim (core's tests: "the claim alone" is refused). `welcome_line` is keyed by the welcome terminal's token, so a wrong token gets an error line, never state.
- Verdict: same exposure as GPUI's, no extra. The one structural difference: the Swift control folder is not behind a build feature (see X12), so the day the `Wormhole-swift*` rule goes, a release build would honour `WORMHOLE_DEVCTL` from a launching process's environment and `control list` / `control send-text ...` would run as an unidentified peer. It should be behind a cargo/Swift build flag before the identity switch, or require the isolation rule to stay for it.

#### New (wave 6 broke or left half-wired)
- Nothing broken that I found in my area. `AppController+Windows.swift` and three siblings now carry a whole-file `// text-file:` mark (see Check 3), which widens a blind spot.
- ChangedPage.swift:35-36 posts the "Copied" announcement twice (duplicate line, VoiceOver may say it twice).
- LaunchLogging.log and the exported `log_line` are dead code: the Swift side never logs.
- TO-VERIFY's wave-6 text is stale in two places: it says the worktree command is NOT BUILT (it is, via PanelBackendsFactory.swift:69-81) and words.py is 0 literals (true, and after the audit-3 15 literals were fixed).
- Nothing compiled or run by me except words.py, so every "closed" is read-and-followed, not exercised; all lanes also say "written without compiling".
