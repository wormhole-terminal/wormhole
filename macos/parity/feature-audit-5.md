# Audit 5: what wave 7 closed (2026-11-13)

Audit 4 found the Swift app 93.4 % written and listed what was open. Wave 7 (six lanes) worked those lists; the branch built
and its tests passed (427 XCTest, 402 Swift Testing, the bridge's 327 Rust tests). Five readers who had not written the
fixes then read each one to where the app reaches it. Nothing was built or run for this audit. Line numbers are of the
branch `exp/swift-s5` at `e85a6720c`.

**Written and reachable: 93.8 %** by the tracker's weights (audit 4: 93.4 %); 69 of 107 rows with no gap left.

Two rows went down, and both are the truer number: X01 (the shared components) from 90 to 80, because this is the first
audit that compared fifteen of them state by state; and P06 (notes) from 85 to 55, because wave 7's merge fix doubled a
terminal's steps and notes when it was not on show (a load makes new ids, and the fix compared by id). That bug is fixed
after this audit (`NotesRebase.swift` compares by what an item says; `NotesRebaseTests`), and P06 keeps the audited 55
until someone other than its author has read the fix.

| Area | Share of the app | Audit 4 | Audit 5 | Rows with no gap |
|---|---|---|---|---|
| crosscut | 21.5 % | 90.7 % | 86.2 % | 1 of 15 |
| panel | 15.4 % | 92.7 % | 91.0 % | 2 of 9 |
| terminal | 13.4 % | 98.8 % | 100.0 % | 31 of 31 |
| studio | 9.6 % | 89.3 % | 91.6 % | 1 of 3 |
| window | 9.2 % | 95.2 % | 97.2 % | 8 of 12 |
| sidebar | 7.9 % | 97.5 % | 98.2 % | 8 of 10 |
| aux | 7.8 % | 93.1 % | 96.4 % | 4 of 7 |
| settings | 7.3 % | 93.1 % | 97.5 % | 5 of 7 |
| overlays | 4.3 % | 90.8 % | 98.6 % | 4 of 5 |
| platform | 3.6 % | 93.7 % | 97.0 % | 5 of 8 |

## Right panel (P01 to P09)

id,written
P01,95
P02,90
P03,95
P04,100
P05,95
P06,55
P07,95
P08,95
P09,100

(Swift paths under `macos/swift/Sources/`, bridge paths under `crates/wormhole-ffi/src/`.)

#### Gaps still PARTLY / OPEN / WRONG

##### P06 (55)
- WRONG, data-damaging: a merge for a terminal that is not shown is rebased against a fresh load, and every id differs → every step and note comes back twice. `land`/`store` read `now = persistence.load(session:)` (NotesCoordinator.swift:146) while `sent` was another `load` taken before the await (line 112). `RustNotesPersistence.load` builds `NotesModel(saved:)` with `Note(text:)`/`PanelTask(text:)`, which mint new `UUID()`s on every load (FilesBridge/Notes/NotesMapping.swift:11-15; TaskList.swift:5-14; NotesModel.swift:6-14). So `now == sent` is false for any non-empty terminal, and `NotesModel.rebase` runs with ids that never match: every item of `now` has `before[id] == nil`, so it is treated as "added meanwhile" and inserted again after the merged list (NotesRebase.swift:36-47). The doubled model is saved at once (NotesCoordinator.swift:158) and the next sync sees local lines the base lacks and writes them into `notes.md`. It happens whenever a merge applies (the file changed, or the first sync of a launch), for every terminal of the window that is not the shown one, and for the shown terminal after a switch-away during the await (then `now` comes from the database too). The new test cannot see it: its `Persistence` keeps `NotesModel` values, so ids are stable (NotesMergeSavedTests.swift:12-18, 51-59).
- WRONG, a person's tick, edit, delete or reorder of a step made during the await is still lost on the shown terminal whenever the merge applies. `applying` rebuilds all steps with fresh ids (NotesMapping.swift:44), so in `rebase` no merged step matches `sent`'s ids: the merged step list wins whole, a tick made meanwhile is reverted, a step deleted meanwhile comes back (NotesRebase.swift:36-42). Only a step ADDED meanwhile survives. Notes keep their ids through the slots, so notes behave as designed. No test covers `rebase` at all (no reference under Tests/). Audit 4 called "a tick during the await survives" closed; with the bridge's real model it is not.
- A reorder made during the await is dropped for notes too: `out` is built in the merged order (NotesRebase.swift:36-42) (audit 4 item 4, unchanged).
- `editing` is now read just before each send (NotesCoordinator.swift:110): item 2 is mostly closed. What is left: an editor opened AFTER the send, during the await, still has the model replaced under it (steps get new ids, so a step editor loses its row), `store` guards only `merged != now` (line 153).
- Moving a window to another profile (`follow(profile:)` swaps the store) while a merge is in flight: `store` loads from the new profile's database, which has nothing for that id, so `rebase` drops every item the merge returned that `sent` had (NotesRebase.swift:36-38 `guard let is_ = after[...] else { return nil }`) and saves that. Not covered by anything.
- `departed` is never emptied (NotesCoordinator.swift:20, 86): harmless (ids are not reused) but a leak.

Orders of events, checked against the code (the base is the core's `notes_bases`, set in `step`, ProjectService notes_sync.rs:177):
- Switched away during the await, merge returns lines: now stored for THAT terminal (NotesCoordinator.swift:134-150), the base and the database agree on the file's lines. No file line is read as deleted. CLOSED, apart from the duplication above when the model comes back from the database.
- Closed during the await: `land` calls `persistence.closed` again, base dropped, nothing saved (lines 131-135). Fine. `forgetClosed` may call `closed` a second time: idempotent.
- Moved to another window: `departing` sets `departed`, the merge is stored for it (line 133). The destination window's coordinator may have loaded the stale model for it before the store: the destination's view then shows the old model against an advanced base (the same stale-model hole, across windows). Not closed, not tested.
- Trust revoked: the core returns the empty outcome, `answered` is nil, nothing changes, base kept (notes_sync.rs:84-97). Fine.
- Second `sync` while one runs: `syncing` returns at once (line 100). CLOSED. A hung merge blocks every later sync of the window, and so does an `await root(of:)` that never returns (no timeout): minor.
- Person ticks/adds during the await: add survives; tick does not (above).
- Base against stored model drift: when the app and the core agree after `store` they are in step; they can drift only through the three cases above (duplication, lost tick/delete, profile move), each of which then reaches `notes.md` through the next merge.

##### P05 (95)
- The stored worktree list is read, changed and written under one process-wide lock, ports picked inside it (worktrees.rs:119-124, 197-215, test tests/git.rs:552). CLOSED. Two small things: a transient failure to read the stored list (`.ok().flatten()`, worktrees.rs:199-201) looks like an empty list and the next write replaces the stored list with that; two profiles pick ports independently from their own lists (the OS port probe is the only guard), as the GPUI app's per-profile store does.
- Second request per window, menu-opened sheets, and the question in the window the request reached: CLOSED (worktree.rs:57-60, mod.rs:152, AppController+Windows.swift:122, HookHost.swift:104-107). Residue: when the window named by the request is gone, the question falls to the first window while the flag is left for the dead id.

##### P02 (90)
- A request for a terminal that is not selected now survives the follow (ChangesStore.swift:99, WindowHub+Branch.swift:10); a non-repository ends it (ChangesStore.swift:145). Not closed: in a folder already settled as not a repository, `setPhase` is not called again by a chip click, so `pickerFor` stays set until the next refresh and the picker can open later (ChangesListView.swift:57).

##### P08 (95)
- Layout selection: the last terminal that asks for focus is selected again at the end, same as GPUI (PanelEffects.swift:40-45; feature_terminals.rs:100-126). CLOSED. A layout's `run` is written at once into the new terminal's view (made on first use, TerminalViews.swift:26): fine.
- Action key anywhere: the window-wide monitor takes only ⌘+character or an F-key (ProjectActionKeys.swift:64-69), asks the core for the key each time (`action_for_key` → `fresh(Feature::Actions)`: project folders off, untrusted or Actions off all answer none, service.rs:195, 268-286), and the run is gated again (`run_action`). A key that is nobody's returns the event untouched. Test an_actions_key_is_the_projects_only_while_it_is_trusted_and_on (tests/project.rs:145). CLOSED for the questions asked: it cannot run in an untrusted project or with Actions off, and a text field loses only ⌘/F-key chords an action owns. What differs from GPUI: no `has_active_dialog` check (GPUI app/handlers.rs:268), so a key pressed with an in-window overlay (palette, shortcut sheet) up still runs an action; the pre-filter reads the panel's cached `state.actions`, not the selected terminal of a pane the key was typed in; repeats (`isARepeat`) are passed on to the view instead of claimed.

##### P01 (95)
- Tab role per tab (window/chrome.rs:279-286) → Swift: still a button with `.isSelected` (WindowUI/PanelTabBar.swift:29-32); lane says NOT DONE. Minor.

##### P03 (95)
- Comment form as a dialog (review.rs:92-178) → Swift: still a popover (Diff/CommentEditor.swift); untouched.

##### P07 (95)
- Non-UTF-8 folder (PathBuf throughout) → Swift: folders are `String` (lane: NOT DONE).

#### Counts of audit 4's gaps closed
- P01: 6 of 7 (tab role open).
- P02: 5 of 6 (the non-repository request is only half closed).
- P03: 4 of 5 (dialog open).
- P04: 2 of 2.
- P05: 3 of 3 of the items audit 4 left (second request per window, window the request reached, store per window); residue noted.
- P06: 1 of 5 for the notes list (switch-away stored for the right terminal; overlap guarded by `syncing`; item 2 partly), and the claimed "tick survives" is not true with real ids; two new defects.
- P07: 1 of 2.
- P08: 3 of 3 (match by name, selection, action key), with differences listed.
- P09: 2 of 2 (per-profile reports and the nap cut short by a new report: `load()` cancels `nap`, UsageStore.swift:30; RustUsageBackend.swift:24-43).

#### New
- The duplication of notes and steps on a merge for a terminal read from the database (P06, first item): introduced by wave 7's `store`; before it, a non-shown terminal's merge was saved as returned. Highest priority: it writes into the person's `notes.md`.
- Wave 7's rebase was never run with the bridge's real model (ids), so "a tick made during the await survives" is untrue for steps.
- Nothing found broken in what wave 7 touched elsewhere in the panel (worktree store, usage, project key). Not read line by line: PanelUI diffs of 2-15 lines, studio/tab_sets/split_opening/status_counts files (other areas).

## Sidebar, window and terminal

id,written
T01,100
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
S01,100
S02,100
S03,100
S04,100
S05,100
S06,100
S07,100
S08,95
S09,95
S10,100
W01,90
W02,100
W03,95
W04,100
W05,100
W06,95
W07,100
W08,100
W09,100
W10,100
W11,100
W12,90

(T02 to T31 other than T01: copied from audit 4 / written.csv, not re-read. Only T01 was open. Read only, nothing built or run.)

#### Rows below 100

S08 (95)
- A group card animates the insert pose like a row in no group (GPUI sidebar list) -> Swift: `.transition(.listRow(...))` on the row (SidebarUI/Group/GroupRows.swift:45); whether the container animates the change needs a real Mac. Unchanged by wave 7; counted only as unverifiable.

S09 (95)
- The tab bar's sets are listed by the core (`tab_sets`, view/sidebar/tabs.rs:40) and the amber alert dot by `others_need_you` (tabs.rs:80) -> Swift: current / step / numbered / search ARE the core's over the bridge now (PanesModel/Tabs/TabSets.swift protocol, WorkspaceBridge/CoreTabSetRules.swift -> ffi tab_sets.rs -> `tabs::current_set/step_set/set_numbered/search`; installed by `CoreWindowRules.install()` from `WindowsHost.make`, only when `options.isReal`; callers WindowHub+Sidebar.swift:46-51, WindowHub+Menu.swift:46-53, GroupSwitcherView.swift:20). What is still Swift: building the sets (`TabSets.of`, WindowHub+Sidebar.swift:46, a groups-then-"Ungrouped" listing) and the alert (`TabBarModel.make`, TabBarModel.swift:76-90: loudest of attention/failed over the other sets' items). Same rules as read; drift risk only.
- A build without the bridge (sample/mock mode) has inert rules: no key steps and nothing is found. Not the product.

W01 (90)
- GPUI plays an opening only for ⌘D / ⇧⌘D (`split_pane` -> `begin_opening`, window/panes/splits.rs:297; a drop or a join is a cut, the core's own doc says so) -> Swift: the tracker decides from the layout change alone (SplitOpeningTracker.swift:41-49: exactly one new pane, count+1, the core's `of_rects` says one pane gave half its room), so a terminal dropped on a pane's edge or joined from the sidebar (`.joinSplit`) that happens to be half a pane's room also plays the spring. Wrong only in that the motion plays where GPUI cuts.
- GPUI holds the old terminal at the size its grid measured in the last frame (`size_for_grid`, opening.rs:100) -> Swift: holds it at the box of the layout before (`opening.whole`, PaneArea.swift:112). Same size by construction; a lone pane's terminal is resized once at the start when its header appears (the lane says so, GPUI has the same header-from-the-start).
- A window resize while it plays: GPUI compares the area and cuts (`opening.area == pane_area_size`, opening.rs:119) -> Swift: no explicit area check, but a resize goes to the core (`setPaneArea`), changes `panes`, and the tracker sees a layout change with no new pane (`came.count == 0`) so the change becomes `.none` and the layout is drawn as it is: a cut. Closed in effect.
- Written without compiling (lane's note): the opening has no test of the view; the core pieces (`of_rects`, ffi `split_opening`) have tests.

What I read for W01 and found right: `PaneArea.swift` body asks the tracker once per layout, `placement` keeps the target's terminal in `opening.whole` (its old size) clipped to `frame.kept`, the new pane at its final box moved in over the target's room clipped to `whole`; dividers are `[]` while playing; `TimelineView(.animation(paused: playing == nil))`; `.task(id: playing?.began)` one-shot bumps `settled` after duration+0.05 s so the body redraws the final layout; `.animation(...)` is `nil` while playing or for a cut. So a terminal changes size: the new pane never (final box from the start), the old one once at the end (plus the lone pane's header at the start). Reduce Motion: `plays = playsMotion && !reduceMotion`, so a qualifying opening is `.cut` (no animation at all), as GPUI (`plays_motion` false -> no opening). Motion feature off: same (`look.motionFeature`, TerminalPanes.swift:18). Second split while one plays: the tracker's `seen` is the layout of the first split's end, so the second is a new opening from it (the first pane that was held jumps to its final box = one resize it would have had anyway); a close: no new pane -> none -> immediate. Wired: `CoreSplitMotion()` set only when `options.isReal` (AppController+Windows.swift:40) and `splitMotion` goes into `paneActions` (WindowHub+Panes.swift:25) which `paneActionsWithKeys` builds on.

Core change: `Opening::of` now builds the two rectangle lists exactly as before (lone pane = whole area, the one that is not new) and calls `of_rects` (core split.rs:60-130). The body of `of_rects` is the old code moved; the only changes are the `new_pane` lookup moving inside and the `find(new)?` kept before. For the GPUI app (window/panes/opening.rs:96, `Plan::of`) nothing changes; a new test shows the two agree. Verified by reading the diff.

W03 (95)
- Counting, longest-waiting order, the rest's order and width classes are now the core's: StatusSummary.init -> `StatusRules.rules.counts(of:)` -> CoreStatusRules -> ffi `status_counts` -> `Summary::of` / `rest()`; `StatusBarDetail(windowWidth:)` -> `Detail::at` (WindowUI/StatusSummary.swift:60-81, WorkspaceBridge/CoreStatusRules.swift, ffi status_counts.rs). `since` is passed as ms; unknown since counts as the last to have waited. Installed in `CoreWindowRules.install`. CLOSED for those.
- The tooltip's composition (first, "then" up to two more, "and N more", "go there / go to the first", core `needs_you_tooltip` status_bar.rs:186) is still written in Swift (StatusSummary.swift:36-52), with catalog words in place of the core's English. Same rule as read (3 named, rest counted); drift risk only.

W06 (95)
- A moved window's project trust follows it (new `follow_dir`, ffi project/service.rs, test `trust_follows_a_window_to_the_profile_it_moved_to`; ProfileChipBinding.swift:159), usage follows (`follow(profile:)`, :160), the hosted desk was already re-seated (:155, was there before wave 7, so audit 4's "hosted desk stays" was wrong on that point), access drivers are per profile and ask on a window of the question's profile (AuxController.swift:70-90), Project commands uses the front window's profile (AuxController.swift:39). Not found: the Imports desk (Settings', follows the shown profile; I did not check GPUI's rule line by line). Counted as 1 of 1 mostly closed, 95 for that.

W12 (90)
- A B terminal's colours: `TerminalFactory.make` now calls `source.setEngineColors(palette.engineSlots)` before the view exists (TerminalFactory.swift:46), so a terminal made through a window's factory is answered in its window's colours. But the core has already started the program in `real(id)` just above it, so an OSC 11 asked at once can still be answered from the engine's global look (the lane's own NOT DONE: `ffi/src/engine_look.rs` is one global read at launch.rs). And a B terminal with no view yet still hears the shown profile's theme changes. Open, rare.

#### Closed, counted per row
- T01: 1 of 1 CLOSED. RowBuilder.swift:192-203: when the cell is under a filled focused block cursor (`covered(column)`, and the second cell of a wide one) and its own colour (`colors(... underBlockCursor: false).bg`, inverse honoured) is not the default background, one `SolidInstance` with style 0 (not `fixedBit`) is appended to `backgrounds`; style 0 is what the shader extends into the padding at an edge, the cursor's own quad is `fixedBit` and flushed later, so it draws over it. For the last column of a stretched row the right edge is `gridRight`. Nothing is added on a row without the cursor, for a default background, for beam/underline/hollow cursors or an unfocused window; the extra work is only on the cursor's row, which the blink flip already rebuilds.
- S01: 2 of 2 (kept). S02: 1 of 1 (kept).
- S03: 2 of 2 plus the group-gone note CLOSED. `GroupFolderQuestion.openTerminal` (SidebarUI/Group/GroupFolderQuestion.swift:22-37) is the one function all three callers use (SidebarView+Group.swift:30, SidebarContext+Away.swift:28, WindowHub+Sidebar.swift:144). After the question, `stands(group, named:)` (group present and same name) decides: the answer is applied and `.newTerminalInGroup`, else `.newTerminal`, which the core opens in no group (workspace/terminals.rs:35, no group inheritance), in the selected terminal's folder. That is `answer_group_folder` (window/group_folder.rs:164-170: gone or other name -> `open_session_in(Some(None))`, loose). Offer nil: no question, one terminal. The static `asker` is a test seam only.
- S04 to S07: nothing listed, 100 kept.
- S08: 5 of 5 as audit 4; the card pose is unverifiable (not counted against).
- S09: 3 of 4 as audit 4, and the fourth now PARTLY: the four rules are the core's, the set listing and the alert are Swift.
- S10: 1 of 1 (kept).
- W01: 2 of 3 gaps from audit 4 closed in effect, the third (the opening) is now written and wired; the differences above remain.
- W02, W04, W07 to W11: nothing open, 100 kept.
- W03: 4 of 4 as audit 4 and the "Swift copy" now 3 of 4 core (the tooltip composition remains).
- W05: 1 of 1 CLOSED. `openWindow(inProfile:)` (ProfileChipBinding.swift:132-141) takes `hostBeside(id)` which returns `profileHosts[id]`, and the shown profile's host is stored there at launch (AppController.swift:125) and at a switch (:76), so the shown profile gets its own host and a window made with `makeSession(window:in: target)` (openPlanned). ⌘N (`openNewWindow`, AppController+WindowOpening.swift:28-33) uses `front.windows`, the front window's profile. The two roads no longer share code.
- W06: 1 of 1 mostly (see above).
- W12: 3 of 4 as audit 4; the fourth partly improved.

#### New
- Nothing wave 7 removed or broke in these areas that I could find. The `paneActions` overload hazard is gone (renamed `paneActionsWithTiles` / `paneActionsWithKeys`, one caller, TerminalPanes.swift:19).
- Half-wired by design: `CoreWindowRules.install()` runs only from the real path of `WindowsHost.make`; a mock/sample run has inert tab-set and status rules (no key stepping, no counts). Not the product.
- The split opening plays for a join or a drop that is geometrically one pane giving half (see W01).
- Everything is written without compiling, per the lanes; I read for logic, not for compile errors.

## Settings and Theme Studio

id,written
ST01,100
ST02,100
ST03,90
ST09,100
STX1,100
STX2,95
STX3,100
SD01,90
SD02,90
SD03,100

#### Rows below 100

##### ST03 (90)
- Permission row follows macOS, including a change made in System Settings (GPUI re-reads every 5th tick while the Settings window is open, `notifications/mod.rs:166-171`) -> Swift: PARTLY, and worse than the lane says. `SystemNotifier.refreshPermission()` is called only from `init` (`PlatformUI/Notifications/SystemNotifier.swift:39`). `publishStatus()` (`Wormhole/Notices/NoticeFeed+Test.swift:13`) reads the cached `notifier.permission`, and `refreshNotify` (page appear, `NotifyControl.swift:73`) only calls `publishStatus`, so reopening the page does not re-read macOS either. The usual flow (Open System Settings, change, come back) leaves the row on the old state until the app restarts. "Allow notifications" itself is closed: `askPermission()` awaits `askPermissionAndWait()` and then publishes (`NoticeFeed+Test.swift:56-62`). A cheap fix is `didBecomeActive` or page-appear calling `refresh()`.
- A permission switched off after launch makes Send test say "Sent": `finishTest` checks the cached `.denied` only, and `center.add` succeeds silently when notifications are denied (same cache cause as above).
- Agent-login folder check (`settings/agent_logins.rs resolved_within`): see "Checks" below. A symbolic link as the final file (`<login dir>/settings.json` -> outside) passes the check, and core's `install_in` writes through such a link (own-file `Follow` policy). It needs a link planted inside `WORMHOLE_AGENT_HOME`, so low.

##### STX2 (95)
- The page re-reads the project's files AND trust about once a second (`sections/projects.rs:44-46`) -> Swift: PARTLY. The project's `.wormhole` folder is now watched by core (`ffi/src/settings/project_watch.rs`, `ProjectsModel.select`, `RustProjectsBackend.follow`), so a hand edit arrives within about 100 ms. Trust (the approvals store) is not watched: a project trusted or revoked elsewhere (another window, the CLI) shows only after the window comes forward (`appearsActive`).

##### SD01 (90)
- The Studio's rows and Low filter mark pairs that fall short from core's contrast check (`studio/token_editor.rs`, `window_tests.rs`) -> Swift: PARTLY. The card now comes from core (`studio_contrast_issues`, `StudioDocument+List.swift:63`, `LowContrastCard.swift`), but the row warning triangle, the Low filter and the editor's per-token checks still use Swift's own pair table and ratio maths (`lowContrastTokens` `StudioDocument+List.swift:20`, `ContrastCheck.lowTokens`, `checks(of:)`). They can disagree with the card. The old simple path is also the silent fallback when core's call throws (`try?` at `:64`).
- "n uses" in the metric preview and the "Used by" lists come from core's `TOKEN_USES` (`studio/token_editor.rs:~340`) -> Swift: uses `GalleryUses.swift`, a table generated from the Swift sources (`macos/scripts/gen-gallery-uses.py`), which differs from core's (for instance `neutral.chip` and `badge` do not list the same tokens; wave 7 removed `metrics.radius_chip` from several rows). The count shown beside the metric preview is therefore not GPUI's.
- An open Studio uses `MainActor.assumeIsolated` in its `@Sendable` folder closure (`Wormhole/Platform/StudioLauncher.swift:65`): any call from a background task traps. I found none that is off the main actor, so it is a risk, not a bug.
- Closed here: metric preview (`MetricEditor.swift preview`), expression completions through `studio_completions` (`ExpressionField` calls `document.completions` on every body, Tab/Return/arrows/Esc, UTF-8 ranges handled in `FieldCompletions.taking`), appearance chip on every row (`TokenRowView.reach`, `TokenListView.swift:61`, click moves via `document.move`), theme folder and versions through core, profile switch (`StudioLauncher.show`, folder closure asked at each call).

##### SD02 (90)
- Status tiles held part-way through every change, counts included (`studio/gallery/tiles.rs`) -> Swift: PARTLY. A flare and a nudge are held at one third (`TileStatesSample.swift caught`, `StatusPlay.swift statusCaught`); the other variants and counts still only play on press.
- Tooltip and dialog specimens marked with their `TOKEN_USES` ids (`gallery/navigation.rs:227`, `windows.rs:191`) -> Swift: OPEN. `SystemSpecimen.swift` explains that macOS draws them; no `tooltip` or `dialog` marked element exists in `StudioUI`.
- Core's `TOKEN_USES` -> Swift: OPEN (generated Swift table, above).
- Lazy gallery: CLOSED (`GalleryView.swift` `LazyVStack` for the page and each column; the outer lazy stack holds the two-column `HStack` as one child, so the laziness is the inner stacks').

#### Counts (audit 4 gaps CLOSED out of total)
- ST01: 4 of 4 (unchanged, copied).
- ST02: 13 of 13 (opacity slider off `SettingSlider.offWhenOpaque`, wired from `EntryControl.swift:31`; shell menu command lines `SubtitledPicker` fed by `host.rs shell_options` -> `RecordMapping.option` -> `OptionPicker`; fonts specimen follows the slot `FontPreview.slot` -> `FontsScene` `onChange`).
- ST03: 7 of 9 (the Allow press and the refused state are CLOSED; the System-Settings change is OPEN, and the logins path is hardened with one residual, above).
- ST09: 1 of 1.
- STX1: 1 of 1 (unchanged).
- STX2: 2 of 2 on the files, 1 of 2 on the trust part: counted as 1.5 of 2.
- STX3: 2 of 2 (unchanged).
- SD01: 5 of 6 audit-4 gaps (folder, metric preview, completions, appearance chip, versions/theme reading through core CLOSED; contrast fixes PARTLY: card yes, rows and filter no).
- SD02: 1 of 4 closed, 1 PARTLY.
- SD03: 7 of 7 (unchanged).

#### Checks asked for
1. Studio folder through core (`studio_folder.rs`, `FfiThemeFolder.swift`).
   - Save: `studio_folder_write` makes the folder private, reads the old file capped at core's `MAX_THEME_BYTES`, keeps it with `keep_version_now` (which thins by `keep_policy`), then `write_atomic`. Failing to keep a version is logged and does not stop the save, as in GPUI (`studio/themes.rs:264-275`). Versions listing and `read_version` are core's (`Within` the version's own folder).
   - Size cap: reads (`studio_folder_read`, versions) use core's cap. A save does not cap the text written (GPUI does not either).
   - Symbolic links: `studio_folder_read`/`write` use `LinkPolicy::Follow`, the same as the GPUI Studio (`themes.rs:198, 265, 338`) and core's registry (`registry.rs:336`). So the Swift copy's refusal of links is gone, as GPUI never had it. Yes, a link in the themes folder can make a Save write outside it: `Evil.json -> /elsewhere/x.json` is listed (the file list stats), opened if the target parses as a theme, and a save replaces the target's content through the link (keeping a version in `.versions/<id>`). GPUI does exactly the same. It needs a link made by the person (or a shared themes folder). The `.versions` folder is the safe part: a link at `.versions` or `.versions/<id>` is refused (`saved_versions.rs is_real_dir`, core test `a_linked_versions_folder_is_never_followed`). The bridge also takes any `path` (not confined to `themes_dir`), as GPUI's path picks do; Swift passes only listed files and its own Save-as names.
   - Small difference: GPUI picks the version id from the file being saved (`themes.rs:276`), the bridge from the old text (`studio_folder.rs version_id`). Same result when the file is listed by `UserThemes` (the usual case), possibly a different id for an unlisted hand-written file without an id.
   - Removed Swift tests: version thinning is covered by core's `the_policy_thins_by_age`, `a_burst_in_the_last_hour_is_capped`, `saves_are_kept_listed_pinned_and_thinned`; the contrast fix by core's `fixes.rs` tests (`a_follower_that_needs_more_contrast_gets_its_own_fix`, the `readable(` starts-with cases) and by `ThemeFilesFFITests.testCoreOffersFunctionsAndNamesAtTheCaretAndFixesAPairThatFallsShort`. The bridge test `a_save_keeps_what_it_replaced...` and `testASaveKeepsWhatItReplaced...` cover save, pin and read-back. Not tested anywhere on the bridge: the size cap, a link in the themes folder.
2. `resolved_within`: `..` is refused by component, links on the way are canonicalized, a dangling link is refused, the missing tail is plain names, and the result must start with the canonical root. Paths I tried: `<home>/link/.claude` (refused), `<home>/../x` (refused), `<home>/./a` (allowed, inside), a relative path (resolved against the cwd, passes only if it lands inside), the root itself (allowed, inside). One passes and lands outside: a link as the final file (`settings.json` or the backup file in the login dir) pointing out, because core's installer writes through own-file links. Also a race: the path is checked and written later (a link swapped in between). Environment-side, unchanged from audit 4 and not the login path: `WORMHOLE_AGENT_HOME=<home>/../<home dir name>` is not equal to the home path, so the "not the real home" filter (`agents.rs:126-129`) passes it. The test covers `..` and a link out, not a file link.
3. Send test and the permission row: reachable outcomes are waiting, sent, denied (cached), unanswered (`askPermissionAndWait` returns with no answer), refused (`deliver` throws from `center.add`; only a real failure of the request, since macOS accepts the request when denied) and, in a test instance, sent through the log. A test instance never reaches the system: `SystemNotifier` is made only for `NoticeRoute.center`; `LogNotifier.deliver` posts into the log. Polling: only `publishStatus`'s one-shot re-look every 500 ms while the permission is `.checking`, which ends when the system answers. The stale-cache problem above is the main defect.
4. Completions and contrast fixes: the Studio does use them. `ExpressionField` -> `StudioDocument.completions` -> `FfiStudioFiles.completions` -> `studio_completions` -> core `completions`; `LowContrastCard` -> `StudioDocument.contrastIssues` -> `studio_contrast_issues` (resolves the theme in the profile's folder, wraps what was typed, `follows` shown). Exception: row marks and the Low filter (SD01 above).

#### New
- Permission cache never refreshed after launch (ST03): a Settings page that can lie about macOS's state, and a Send test that says "Sent" after the person turned notifications off.
- Row warning marks and the Low filter use a different contrast source than the card (SD01): a pair can show a card line without a marked row or the reverse.
- `GalleryUses` edits by hand-looking removals (`metrics.radius_chip` out of `badge`, `neutral.chip`, `tint.chip`, `commit.row`, `icon.tab`) lower the "n uses" count shown for a metric; the file says it is generated.
- Nothing removed that worked: the Swift Studio folder copies (`DiskThemeFolder`, `VersionPolicy`) are gone and nothing refers to them; the tests for them moved to the bridge.

## Secondary windows, overlays, platform

id,written
A01,95
A02,100
A03,100
A04,100
A05,95
A06,100
A07,95
O01,100
O02,100
O03,100
O04,90
O05,100
PL01,95
PL02,100
PL03,95
PL04,95
PL05,100
PL06,100
PL07,100
PL09,100

#### Gaps still open

A01 (95)
- The trust card is built from the facts every frame, so it follows an approval made anywhere (`onboarding/mod.rs:201-212`) -> Swift: the card is rebuilt only when the root changes or after this session's own `trust()` (`OnboardingBridge.swift:79-95,117-135`). The `WakeHub` handed to `TrustDesk` (l.37, `hub`) is never observed, so an approval made in another window or by a hook leaves the card up. Small.

A05 (95)
- Mode settled for every profile on the app's tick (`ask.rs:tick`) -> Swift: `ProfileLook.apply` calls `hooks.settingsChanged(profile:)` (`ProfileLook.swift:83-84`) and `SettingsApplier.swift:81` does it for the shown profile, so every profile with a window in sight is settled. A parked profile's windows wear the app's own look (`ProfileLooks.swift:65`, `ownSessions`) and have no `ProfileLook` following their `config.json`: an edit of a parked profile's `agent_access` is not settled until that profile is shown again (the lane's own caveat, true).

A07 (95)
- Same as A05: the keep-it question for a widening made outside the app appears at once for the shown profile and for profiles with a `ProfileLook`; not for a parked profile.

O04 (90)
- Not touched by wave 7 and not re-read by me; audit 4's 90 stands.

PL01 (95)
- Show Project greyed also when project folders are off (`features/project/def.rs:101`) -> Swift: only when the terminal has no folder (`WindowHub+Menu.swift:21`); NOT WRITTEN, visible difference small.
- Open Recent reads a per-profile list cached in memory the first time it is asked (`RecentProjects.swift:22-29`); nothing re-reads the store, so a change by the GPUI app or another process is not seen until restart. GPUI reads its store once as well, so this is small.

PL03 (95)
- A dev build without a bundle posts through the platform's script -> Swift: the log only (dev only, lane says NOT WRITTEN).

PL04 (95)
- Untouched by wave 7 (DockController: logging only); audit 4's reservations stand (`.reply` does not activate; the jump key not followed to a recorded key).

#### Per row: audit 4's gaps found CLOSED / total

- A01: 1 of 1 (card approving flag, card goes after approval via `trustRevision`/`observeTrust` `OnboardingSession.swift:18-42`, `OnboardingBridge.swift:117-135`; no card without folder or front project, `:79-88`; folder's own project by core's rule `first_run.rs project_root`; profile passed `AuxBackends.swift:185`; Touch ID through `authenticator` = `LocalAuthenticator`, scripted in a test instance `AuxBackends.swift:26-28`; Touch ID reason names the project root `AgentsStep.swift:36`). Not closed: external approval refresh (95).
- A02: skimmed, untouched.
- A03: 2 of 2 (Check for Updates opens Settings > Updates and starts the check, `AuxController.swift:59-66`; "This is a development build; it doesn't update itself." `AuxText+About.swift:61`, `swift-aux.ftl:146`, shown by `UpdateStatus.swift:61`; `UpdateWindow` deleted with no remaining reference).
- A04: 1 of 1 (`HostConnect.swift:32-33` keeps the error's reason, `HostBook`, `RustWorkspaceBackend+HostProblems.swift:24`, `AuxBackends.swift:226`, then `hosts.rs:277`).
- A05: 1 of 1 for what audit 4 listed (shown profile + every profile with a ProfileLook; Rust test `hook_server/tests/profiles.rs` for a per-profile settle); parked profiles open.
- A06: skimmed, untouched.
- A07: 1 of 1 listed; same parked caveat.
- O01: 2 of 2. Commits: the log of the terminal's folder, read through the backend at palette open (`PaletteCommits.swift`, `PaletteSourcing.swift:44-54`, `PaletteView.swift:71` `onAppear`), empty with History off, not the History tab's model; recents `PaletteSourcing.swift:63`, `recents.roots(of: window profile)`.
- O02: skimmed, untouched.
- O03: 1 of 1. Per profile: `FolderPicking.swift:105-115` (`recents(of: profile)`, `storedRecents(profile:)`), `folder_picker/mod.rs:52-64` (same `recent_folders` meta key, core's `to_json`/`from_json`, `keep_out` applied, `dir_of` refuses anything outside an isolated data folder). `follow()` visits go to the holding window's profile, flag off clears once per profile.
- O04: untouched (see above).
- O05: 1 of 1 (not a gap: `TerminalSelection.swift:34` calls `clearHistory()` -> `pointer.rs:228` returns `host_forgets_history() == Some(false)` -> `onHistoryKept` -> `TerminalOverlay+Clipboard.swift:23` posts `.historyKeptByHost`; all three Clear Buffer paths (menu, ⌘K, context menu, router) go through `clearBuffer`).
- PL01: 4 of 5 listed (Open Recent per profile `RecentProjects.swift:19-29`, `ActionRouter.swift:46-48`, `menuState` `:68-72`, `recent_projects.rs` same key `recent_projects`, JSON array, limit 8; Undo/Redo titles `MenuController.swift:40-43` from `StudioLauncher.editTitles`, refreshed on key change by `windowDidBecomeKey/ResignKey` -> `keyChanged -> menus.windowsChanged()`, plus the studio's own `NSUndoManager` titles; ⌥⌘↑/↓ `WindowHub+Menu.swift:25` greys the pane items while a list or the notes editor holds the keyboard). Show Project open.
- PL02: 1 of 1 (`ExtraKeys.swift:25-31` takes Settings, Studio and sheets by `sheetParent` or the front workspace window; a non-main panel is left out).
- PL03: 4 of 5. Retraction: `NoticeFeed.swift:78-87` observes `didBecomeActive` and `didBecomeKey`, calls `coordinator.look(... appActive: NSApp.isActive)`, the core's `observe` retracts by `Seen` (selected and `key` window and active) so a banner of a terminal nobody looks at stays. Per-profile: `noticeTabs` sets `tab.settings` from the holding window's `settingsBackend(of:)` (`NoticeFeed.swift:117-118`), `NoticeMapping.swift:84` passes it, `notify/mod.rs:87,226` uses `facts.settings` before the look's. Dev script open.
- PL04: skimmed.
- PL05, PL06, PL07, PL09: untouched.

#### Key monitors (item 4)

- `ExtraKeys` (live table entries in scope App that no menu holds as first key, plus second keys) and `ProjectKeys` (a project action's key: ⌘ or an F-key) are both local keyDown monitors; a monitor that runs a key returns nil, so the event never reaches the next monitor, the menus or the terminal: a key cannot run two things through them. AppKit's order between local monitors is not specified, but it does not matter here: the core refuses a project action key that the keymap table already uses (`project/actions.rs:39-52` `validate_key` via `keymap::uses`), so the two sets are disjoint. Menu first keys: `ExtraKeys` returns false and AppKit runs the item. The terminal view's own ⌘ list and `claimProjectKey` (`ProjectActionKeys.swift:23-35`) are reached only when no monitor took the key (the monitor takes project keys before the terminal in a workspace window, as GPUI's `claim_first`).
- One difference: `ProjectKeys` ignores windows that are not workspace windows (a sheet's window is not in `sessions`), as GPUI refuses a project key in a dialog; `ExtraKeys` does run under a sheet (`sheetParent`), so ⌘1-9, ⇧⌘N and so on act while a sheet is up (GPUI's app bindings: not checked for dialogs).

#### New

- `RecentProjects.menuState(for:)` still writes (`recents.remember`) from a state getter that the palette and `ExtraKeys` also call (`RecentProjects.swift:68`); no-op when the first entry is unchanged. Side effect in a read, not wrong.
- `recent_projects_*` and `folder_recents_*` answer nothing outside a `Wormhole-swift*` data folder (`profile::dir_of` -> `isolation::require`): Open Recent, the picker's Recent rows and the palette's recents are empty in any non-isolated run. By the experiment's rule, but a thing to undo when the app ships as the product.
- Parked profiles: no settings follower (A05, A07).

## Crosscut (X01 to X15)

id,written
X01,80
X02,95
X03,90
X04,95
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

Method: read-only; nothing built or run. Tree swift-s5. GPUI `crates/wormhole/src/style/*`, Swift `macos/swift/Sources/DesignSystem` and the callers. Rows X02, X06, X09, X11, X13, X14, X15 were not touched by wave 7 in any way I could find (X14: the Studio's theme folder now goes through core, `studio_folder.rs` and `StudioBridge/FfiThemeFolder.swift`, wired at `Platform/StudioLauncher.swift:68`; the version-1 caveat is not re-judged); their numbers are audit 4's, copied from written.csv.

#### X01 (shared components): 80, audit 4 had 90

##### What I did
1. Fifteen components compared GPUI file against Swift file, state by state (rest, hover, pressed, selected, disabled, keyboard focus, window behind, Reduce Motion, Increase Contrast, Reduce Transparency, VoiceOver): row, tab (icon tab, title tab), chip, count badge, icon button, segmented, card band, section label, key cap, fields, focus ring, banner, find bar, palette row, status tile. Verdict per component:
   - key cap (`controls.rs` keycap vs `DesignSystem/KeyCap.swift`): matches, plus a spoken label. OK.
   - banner (`messages.rs` vs `Banner.swift`): kinds, tints, Increase Contrast outline, container label. `flush` (full width) is not written but nothing in GPUI calls it with `true`. OK.
   - status badge, section label: OK.
   - palette result (`palette.rs` vs `PaletteUI/ResultRow.swift`): selected, hover (none on selected), marked text, second line, key caps, combined VoiceOver label with button and selected traits. One miss: `detail_color` (a result's own detail colour) has no Swift counterpart; the detail is always muted. Otherwise OK.
   - status tile (`tile/` vs `SidebarUI/Tile/`): badge, rim, dot for unread, light, amber edge, still under Reduce Motion. The seam is real: `WindowHub+Tiles.swift` gives the row's own tile to title row and pane header, `WindowHub+Sidebar.swift:110` to the status bar, and `TabBarActions.tile` to tabs and group cards. The plain `TerminalTile` is only the fallback. CLOSED as the lane says.
   - panes (`panes.rs` vs `PaneFrame`, `PaneHeader`): 72 % dim, focus line, outline. OK.
   - icon tab, title tab: states and VoiceOver present (read in the lane's table, spot-read `PanelTabBar`). OK.
   - row, chip, count badge, icon button, segmented, find bar, fields, focus ring: gaps below.
2. Ten "header only" lines read in full: bands, info, row_keys, reorder, panel_list, checkbox, preview, panes, palette, find_bar. What the header read missed:
   - `panel_list.rs` `scroll_edges`: a hairline under the header while rows are scrolled up beneath it, in the sidebar and the panel. Table said "to check". NOT in Swift (grep of `onScrollGeometryChange`, `scrolled`, edge across SidebarUI, PanelUI, WindowUI finds only the diff's sideways scroll).
   - `row_keys.rs`: table said "not found by grep". It IS there: `SettingRow.swift:70-78` `.contextMenu` (Copy Key, Copy as JSON, Reset) and the key button with a spoken label. The table was wrong in the other direction. ⌃↩ on a control inside the row opening that menu and `own_menu` (a text field keeps its own menu) are not written; the system context menu is all there is.
   - `info.rs` (the (i)): GPUI's is a focusable button, role, `aria_expanded`, hover fill, open-state text colour. Swift `RowAccessories.swift:24-37` `InfoButton` is a borderless button with a system popover, no hover fill, no open colour, and `accessibilityHidden(true)` (the description reaches VoiceOver through the row's hint instead: equivalent for speech, but the (i) is not reachable by VoiceOver navigation).
   - `bands.rs`: Swift `CardBand` has no drop-target fill, no held-⌘ count dim (`HELD_COUNT_OPACITY`), no folder-tint pill. (`sticky` is never true in GPUI, so not a gap.)
   - `checkbox.rs`: the panel's box is drawn, `accessibilityHidden`, and the row's button speaks the task and "done / not done" as a value (`TaskRowView.swift:28-31`); GPUI says role checkbox with checked state. Close, not the same role.
   - `reorder.rs`: GPUI has no keyboard path at all; Swift adds VoiceOver Move up / Move down (`PartsList.swift:50-51`). Swift is the superset. OK.
   - `preview.rs`: Increase Contrast `card.edge` on the preview frame: only `GitScene.swift:40` uses `contrastOutline`; `PreviewTheme` re-resolves the tokens with increase contrast. Not shown to reach every scene; left as "not verified".
   - `panes.rs`, `palette.rs`, `find_bar.rs`: read in full, panes OK, palette see above, find bar below.
3. The lane's own writes, each followed to its callers:
   - Shared `Chip` and its five callers (`ExtraChip`, `BranchCrumb`, `CommitRefs`, `TintChip`, `RowChip`): `ExtraChip` and `CommitRefs` keep every state they had. `Chip`'s `hoverFill` is passed by no caller, so hover and `selected` reach nobody outside the gallery. One regression: `BranchCrumb.swift:25-27` now applies `.frame(width: width, alignment: .leading)` OUTSIDE the chip, where before the frame sat inside the background; the wash now fits the content, not the planned `plan.fit.chip` width (equal while the text is cut, shorter when it is not, so a glyph-only chip next to room is narrower than planned). Small, but a state change nobody asked for. `TintChip` keeps `textMuted` where GPUI `tint_chip` is `selected_text`, medium weight and an optional icon; `RowChip` keeps `controlBackground`/`textMuted` where GPUI `neutral_chip` is `band_hover` with `text.primary`: unchanged by the lane, still different. Chip adds an Increase Contrast outline GPUI's chips do not have.
   - One `CountBadge` (`CountBadge.swift`): `TabCountBadge` is gone and `PanelTabBar.swift:76` uses it, but (a) it draws a `Capsule()` where GPUI, `TabCountBadge` and `BandCount` all use `radius_chip`: the panel's tab badge changed shape; (b) it is not the only copy: `BandCount.swift`, `FoldedSummary.swift:27` and `TerminalTabView.swift:62` draw the same badge themselves. "One concept twice" is now "one concept four times".
   - `keyboardFocusRing` and the four rings (`TerminalRowView:64`, `GroupHeadView:60`, `WalkRing:22`, `WindowUI/Focus/FocusRing.swift:16`): reaches all four, same token and width as before, now hidden in a window behind (`windowFront`, default true, set at `WorkspaceRootView.swift:72`), as GPUI. Nothing lost. Still hand-drawn: the other rings (settings, studio, onboarding card, find field, sidebar `ringWhileMenuOpen`).
   - Reduce Motion consumers: `motionTransition` on pin pop-in, tile dot, group alert dot, review bar, pane bar swaps scale/slide for a fade; the second line's roll stops (`SecondLineView.swift:36`); drag-drop settle uses the fade. Not complete: `HeadChevron` (`HeadControls.swift:17`) and `SectionLabel`'s chevron still ROTATE under Reduce Motion, only over 0.15 s (`.motion` swaps the curve, not the rotation); GPUI's disclosure swaps at once (`controls.rs` Disclosure; Swift's `PanelUI/Disclosure.swift` does this right). `Row`/`Segmented` selection still slides (`matchedGeometryEffect`) over the fade time.
   - `IconButton` selected and disabled: written correctly in `IconButton.swift:130, 151, 184-196` (faint, no hover or press, selected tint, trait). But `.selected(` has exactly one caller outside DesignSystem: the gallery (`ChromeSpecimen.swift:30`). The find bar's option toggles (`FindToggle.swift`), title buttons (`TitleIconButton`, `TitleButton`), band + (`BandButton`) and row actions remain their own drawings, so the toggle state reaches no app screen.
   - `Row` disabled and selected trait and `Segmented` disabled and container: correct in the files; `Row` is used only by the Studio token list, the font picker and two Onboarding steps (the sidebar, panel and palette draw their own rows), `Segmented` by the diff header and the Studio. Reach is small.
   - `FlatField`: the lane says "gallery only"; true, its only callers are four gallery specimens (`InputsSpecimen.swift`). The app's 20 `TextField`s are the system's rounded field (Settings, the owner's choice) or each module's own; no `control.background` well, hover, prefix, clear or disabled look reaches the app.

##### Gaps still open (most important first)
- A scrolled list shows a hairline under the header (`panel_list.rs:23-39` `scroll_edges`, sidebar and panel) -> Swift: none (no scrolled-state anywhere in SidebarUI, PanelUI).
- A button's VoiceOver name leaves its key out ("Settings (⌘,)" is spoken "Settings", `buttons.rs` `spoken_name`) -> Swift: no equivalent. `TitleIconButton.swift:35-36` uses the key-bearing string for both tooltip and label (`PanesText.splitRightKey` = "Split right (⌘D)"), and the find bar's previous, next, close do the same (`FindBar.swift:34-37`, labels "Previous match (⇧⌘G)", "Close find (esc)"); VoiceOver reads the glyphs. (`TitleButton.swift` in WindowUI does split label and help correctly.)
- Find bar steps and close have hover fill and `text.primary` while hovered (`find_bar.rs` via `icon_button`, `band.hover`) -> Swift: `FindBar.swift:34` plain button, muted always, no hover, no press; toggles (`FindToggle.swift`) selected text is `textPrimary` and hover `stateHover`, GPUI `selected_text` on `ui.selected`, hover `band.hover`, 22 pt face radius `radius`.
- `IconButton` extras: active-filter dot `marked`, `focus_ring` (keyboard ring round the face), `spoken` (a different VoiceOver name from the tooltip), `strip` fills, `in_row` 22 pt face, `narrow`, `menu_chevron` (`buttons.rs`) -> Swift: none in the shared `IconButton`; each caller draws its own (TitleButton has the dot and the ring, BandButton, RowActions); `.help(label)` and the label are one string.
- Text field (`text_field.rs`): the app's fields do not use `FlatField` (gallery only); `multiline` has no Swift field; the two panel fields and the Studio's are their own.
- Count badge: one concept four times, and the shared one is a capsule where GPUI is chip-cornered (above).
- Branch chip fills only its content (above, `BranchCrumb.swift:25-27`).
- Tint chip and neutral chip colours (`labels.rs` `tint_chip`, `neutral_chip`) -> Swift: `textMuted` / `controlBackground` instead (`TintChip.swift:13`, `RowAccessories.swift:46`).
- Row (`controls.rs` `row`): `min_h(ROW_H)`, row padding, selected text colour, `cursor_pointer` -> Swift `Row.swift`: no min height, content decides its colour and padding.
- Segmented: GPUI's form segments are one Tab stop with arrows and role RadioGroup/RadioButton, position in set (`controls.rs` `FormSegments`) -> Swift: Settings uses the system's segmented Picker (owner's choice, which has the keys); the shared `Segmented` has one stop per segment, no arrows, `.contain` only.
- Seven hand-drawn rings are still not on `keyboardFocusRing`.
- Reduce Motion: chevron rotation and selection slide still move over 0.15 s.
- Button, checkbox, switch, link, slider, stepper stay the system's: not counted against the number (owner's choice for Settings).

X01: 4 of the 8 gaps audit 4 and the lane listed that I could check are closed (shared ring on four callers; seam for the row's tile; CountBadge selected state merged; Chip merged); 4 are not (text field in the app, per-state read gaps above, one badge concept, IconButton extras). Audit 4's "1 of 3 closed" counted roll in the status bar only.

#### X03 (VoiceOver): 90
- Key glyphs spoken in button names (above) -> not written.
- Panel's task box is a button with a value, not a checkbox with a checked state (`TaskRowView.swift:28-31`, GPUI `checkbox.rs` role + `checked`).
- The (i) is hidden from VoiceOver navigation (`RowAccessories.swift`), the row hint carries the words.
- Nothing about what VoiceOver says per SwiftUI control has been run (unchanged).
- ChangedPage "Copied" posted once: I did not re-read; the lane reports it fixed. Not verified by me.
X03: not re-counted against audit 4's four closed; 0 of 1 open item re-checked closed; 3 new items.

#### X04 (keyboard): 95
- Esc's popup rung: CLOSED as NOT REAL, I checked it. GPUI passes `popup: false` (`window/focus/mod.rs:202`), so core's rule never reports a popup there either. Swift passes the same (`WindowFocus.swift:206`), with the text field case handled before it: `firstResponder is NSText` returns false (`WindowFocus.swift:202`), `RenameField.swift:31` and `GroupPopover.swift:29` catch Esc themselves, and native menus and popovers are modal. Same behaviour as GPUI.
- Also checked: the "option-arrows go to the ends where GPUI moves the row" caveat of audit 3 is wrong: GPUI's `list_key` maps alt-up/down to First/Last (`focus_keys.rs:115-116`), as Swift does (`SidebarView+Keys.swift:26-27`); ⌃↩ opens the row's menu (`:18-23`).
- Not re-checked: arrows inside Notes/Project/Usage, one Tab stop per panel section.
X04: 1 of 1 closed.

#### X05 (Reduce Motion, Transparency, Contrast): 90
- Reduce Motion: see X01. Rotation of chevrons still runs 0.15 s; the lane's list of consumers is real (I read the diffs, all are wired through `motion`, `motionTransition` or a `reduceMotion` test), but its three leftovers (scroll-to animations in Settings and the Studio, `PageScaffold.swift:44`, `TokenListView.swift:35, 54`) are also uncut.
- Increase Contrast: present on chip, banner, key cap, badges, segmented, field; GPUI's focus ring solid under it (`flatFocusRing`) is written.
- Reduce Transparency: no wave-7 change; `SecondaryWindow` opaque path as before.
X05: the check "each consumer against GPUI's list" was done by the lane for the list, not line by line; I found two consumers short (above). 0 of 0 counted.

#### X07 (words): 95
- I could not run `words.py` (read-only). New literals in the wave-7 diff: only gallery specimens ("Rest", "Selected", "On", "Unavailable"), which the checker skips. `AppLog` lines are English by rule. Unverified, not contradicted.
X07: nothing new to close.

#### X08 (CLI and control socket): 95
- `Wormhole --json list`: CLOSED. `main.swift:14` now lets a first argument `--json` through to `HookBridge.exitCodeIfCommandLine()`; core's `ipc::command` (`client.rs:38-47`) turns any other leading-dash word into `Command::Unknown`, which `cli_exit_code` answers with "unknown command" and exit 2, as `main.rs:68-75` does in GPUI. (Other leading dashes, `--shell`, `--background`, still launch the app; GPUI has no such options.)
- `new-window` answering `unsupported`: NOT REAL, I checked it. `structure.rs:53-57` answers it only when `core.shared.windows()` is `None`, i.e. a workspace not made by `Workspace::in_window`. The app builds every window's workspace that way (`RustWorkspaceBackend.swift:73-86` passes `windows`; `RustWindowsBackend.swift:28`), and `set_windows` is called again on `rehome` (`windows/rehome.rs:172`). The branch is reachable only from a test or a windowless build; no real app state reaches it. Its model is the front window (`plan_like(Some(&core.front_share()))`), which `didBecomeKey` feeds.
- `events`, `read`, `blocks`, `wait`, `run`, `mouse`, `send-keys` flag by flag and the marking-shell `run` fixture: still not read by anyone (mine either).
X08: 2 of 2 closed (1 by fix, 1 by being not real).

#### X10 (performance guards): 85
- GPUI's seven counting guards (`terminal_view/render/guards.rs`) -> Swift: five `@Test`s now in `FrameWorkGuardTests.swift` (lines 32, 42, 48, 59, 76): the new cursor-blink test (`:59-74`, `setCursorBlinkOn` exists at `MetalRenderer.swift:263`) is a real guard of GPUI's "a phase change rebuilds nothing" kind. Not found: a test that a screen without blinking text asks for no clock (GPUI `guards.rs:234`), shaping-without-shaping for half blocks (`:164`).
- `scripts/perf.sh` baseline per machine that fails on a regression -> Swift: `macos/tools/baselines/swift-perf.json` is committed with every number `null` and "Empty skeleton"; nothing compares against it. Still OPEN.
X10: 1 of 2 closed in part (the file exists, the numbers do not): 0 of 2 closed, 1 improved.

#### X12 (dev tooling): 90
- `new-shell <key> [dir]`: CLOSED. `AppController+ControlRead.swift:49-65` takes the folder (`~/` expanded), sends `.newTerminalWithShellIn`, and `RustWorkspaceBackend.swift:120` sets `folderOfNext`, read at `:192` when the terminal starts; the shell comes from the key.
- `a11y` showing SwiftUI's nodes: PARTLY. `AccessibilityDump.swift` `children(of:)` now tries `accessibilityChildren`, then the navigation order, then subviews. Whether SwiftUI vends nodes to an in-process ask is the very thing audit 3 found it did not; the fallback to `subviews` returns NSViews, not SwiftUI nodes. Written, not run, and `a11y live` is still not there.
- Swift's own log lines: CLOSED. `AppLogging/AppLog.swift` (a sink), installed at `LaunchLogging.swift:19` into `logLine`; callers: `TerminalLinks.swift:48`, `DockController.swift:46`, `SystemNotifier.swift:50`, and the data-folder error line at `ProfileChipBinding.swift:98-100`. Only four call sites, but the seam is real.
- Still open: `front`, `paints`, `refresh`, `scaling`/`settle`/`reveal-test`/`grow-hold`, `scroller`, `notice-material`, `settings-preview`; `pointer` still ROW COL; `hosts` and `notices` keep other meanings; the control folder is compiled into every build (`AppController+Open.swift:30-35`, switched on by `WORMHOLE_DEVCTL` alone), safe only while `Isolation.swift` stands.
X12: 3 of the 5 line items closed (new-shell, logging, data-folder log), a11y partly, the rest open.

#### X02, X06, X09, X11, X13, X14, X15
Untouched or only touched beside the row (X14: Studio folder through core). Numbers copied from written.csv.

#### New (wave 7 broke or left half-wired)
- `BranchCrumb.swift:25-27`: the branch chip's wash no longer fills the planned width (frame moved outside the chip).
- `CountBadge.swift:34`: `Capsule()` for the tab badge, where it was chip-cornered (`TabCountBadge`) and GPUI's is `radius_chip`.
- `IconButton.selected`, `Chip` hover and `selected`, `FlatField`, `Row` disabled: written, reach no app screen (gallery only); the lane's table counts them DONE.
- `HeadChevron` and `SectionLabel` chevron still rotate under Reduce Motion (a faster rotation, not none).
- Table errors in the lane's header-only lines: `row_keys.rs` is present in Swift (table said not found); `panel_list.rs`'s scroll-edge hairline is absent (table said to check).
- Everything is written without compiling (the lanes say so); none of this was run by me.
