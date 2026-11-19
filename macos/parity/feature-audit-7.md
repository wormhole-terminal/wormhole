# Audit 7: what waves 9 and 10 closed (2026-11-14)

Audit 6 found the Swift app 95.9 % written and listed what was open. Wave 9 (five lanes: hosting, menus, style, project,
commands) and wave 10 (two: parts, panel) worked those lists. Four readers who had not written the fixes then read each
one to where the app reaches it, on a snapshot of `exp/swift-s5` at `f578f1527`. The 59 commits they read had been
written without compiling and were being compiled while they read: a fix that plainly could not compile would count as
partly done (none was found), and what the compiler changed afterwards is not in this audit. Nothing was built or run
for it. Line numbers are of that snapshot.

**Written and reachable: 98.0 %** by the tracker's weights (audit 6: 95.9 %); 87 of 107 rows with no gap left.
Verified by a test or a driven check is a separate figure (`features.csv`, `macos/scripts/built.py`).

What is left, in points of the whole app (the row's number in brackets): X01 0.49 (95), X12 0.49 (90), P07 0.12 (95), P08 0.12 (95), P03 0.11 (95), X10 0.11 (85), P06 0.09 (95), S08 0.09 (95).

The three faults audit 6 found in what two windows of one profile share are closed by the readers' account: one store
per profile in the process for the approvals and the switches, read at the moment a gate acts and under its lock, and
one lock per `notes.md`. The readers found no way left for a revoke or a switch-off made in one window to be ignored in
another, and no path that drops a saved terminal's row or a field of it through End All, a quit, a crash or a restart.
What they did find is listed per row below; wave 11 works it.

| Area | Share of the app | Audit 6 | Audit 7 | Rows with no gap |
|---|---|---|---|---|
| crosscut | 21.5 % | 88.6 % | 93.7 % | 3 of 15 |
| panel | 15.4 % | 94.5 % | 97.1 % | 5 of 9 |
| terminal | 13.4 % | 99.1 % | 100.0 % | 31 of 31 |
| studio | 9.6 % | 95.8 % | 100.0 % | 3 of 3 |
| window | 9.2 % | 99.5 % | 99.5 % | 10 of 12 |
| sidebar | 7.9 % | 98.9 % | 98.9 % | 9 of 10 |
| aux | 7.8 % | 100.0 % | 100.0 % | 7 of 7 |
| settings | 7.3 % | 100.0 % | 100.0 % | 7 of 7 |
| overlays | 4.3 % | 98.6 % | 100.0 % | 5 of 5 |
| platform | 3.6 % | 98.6 % | 99.0 % | 7 of 8 |

# Audit 7, right panel (P01 to P09)

Tree `swift-audit7` at `f578f1527`. Swift paths under `macos/swift/Sources/`, bridge under `crates/wormhole-ffi/src/`, GPUI under `crates/wormhole/src/`.

```
id,written
P01,100
P02,100
P03,95
P04,100
P05,100
P06,95
P07,95
P08,95
P09,100
```

## Gaps still PARTLY / OPEN / WRONG

##### P03 (95)
- GPUI's comment dialog is a window alert, separate from the diff's rows, so a diff that reloads under it leaves it up (features/git/diff/review.rs:72-118, `dialog::open_alert`) → Swift: the sheet hangs off one `DiffLineView` inside a `LazyVStack` keyed by row offset (PanelUI/Diff/DiffView.swift:94-95; Diff/DiffRowView.swift:70, 102, 105). A reload while the person types closes the sheet and loses the text. A reload happens when an agent edits that file. It closes in two cases: the rows shift so the offset becomes a header or another line with a different `comment` (`.onChange(of: comment) { editing = nil }`, line 102), or the diff gets shorter than that offset. This was already so with the popover and audit 6 did not list it. Minor.

##### P06 (95)
- OPEN (the lane's stated trade). The database can't be read at landing for a terminal that isn't shown: the base is forgotten, and the next merge brings back, once, a line the file had deleted (Wormhole/Panel/NotesCoordinator.swift:175-181). GPUI keeps notes in memory, so this cannot happen there (features/project/notes_sync.rs:95-123).
- OPEN (cost, not correctness). Every terminal not on show is read from SQLite on the main actor every 2 s (NotesCoordinator.swift:195-196, via `holding` at 147 and 175). GPUI merges from memory.

##### P07 (95)
- New find, not in audit 6. GPUI lays the project of the terminal in front over Settings and the theme, from its real `PathBuf` (features/project/detect.rs `apply_project_settings`). Swift: `ProjectSettingsFollower` asks with the drawn, `~`-shortened, lossy `folder` instead of the `readPath` key (Wormhole/Settings/ProjectSettingsFollower.swift:52-54, 62-65 → `settings_overrides`/`theme_overlay`, files_panel/project/overrides.rs:34). In a folder whose name is not text, the project's settings and theme are not applied. If a `caf\u{FFFD}` sibling exists and the person has trusted it, that sibling's are applied instead. The Studio's source was moved to `selectedPath` (Wormhole/Platform/StudioProjectSource.swift:19), but this follower was not.
- Left, shared with GPUI (core, not a parity gap): `project::features::root_key` keys approvals and switches by the root's lossy text (the lane notes it).

##### P08 (95)
- WRONG (minor, possibly intended). GPUI runs a project key again on every auto-repeat: `claim_window_key` has no held check, and `intercept_keystrokes` sees every key-down (app/handlers.rs:254-276; terminal_view/input.rs:43-49). Swift claims the repeats and runs nothing again, "one press, one run" (Wormhole/Panel/ProjectActionKeys.swift:38, 72). Nothing leaks to the terminal any more. If the owner accepts one run per press, P08 is 100.

## Audit 6's gaps closed
- P03: 1 of 1. The comment editor is a `.sheet(item:)` (PanelUI/Diff/DiffRowView.swift:105). `hasActiveDialog` holds the project keys while `changes.commenting` or an attached sheet is up (ProjectActionKeys.swift:44-46). One minor new gap, above.
- P06: 6 of 8.
  - **Two windows' notes.md writes.** CLOSED. `stores::notes_file` gives one `Arc<Mutex<NotesFile>>` per canonical `notes.md` path in the process (files_panel/project/stores.rs:180-187). `sync_notes` holds it from read to write (notes_sync.rs:93-95). Audit 6's sequence now runs this way: B waits, reads F0+a, and writes F0+a+b✓.
  - **Stale per-window prefs and trust.** CLOSED. `Hub` holds the profile's `Stores` (one `Arc` per canonical profile dir, stores.rs:114-118). `prefs()` and `trust()` look at the file's stamp each time (hub.rs:58-66). `sync_notes` locks the switch, then the approvals, then the file, and holds them to the write (notes_sync.rs:87-105). `set_feature`, `set_project_default` and `clear_feature_choice` go through `change_prefs`, which changes what the file holds now (hub.rs:70-80; service.rs:159; settings_page.rs:155, 164).
  - **Wrong project after a switch.** CLOSED. The panel's root is used only when `panel.folder == terminal.readPath` (NotesCoordinator.swift:138), and `ProjectStore.follow` clears `state` in the same step it changes `folder` (PanelUI/Stores/ProjectStore.swift:32-35, 52). Otherwise the root is asked of the terminal's own folder.
  - **A deleted step the file ticked.** CLOSED. `revive` brings it back as the file has it (PanelModel/Notes/NotesRebase.swift:33, 82-83), as core merge.rs:41-42 does.
  - **A terminal moved during a merge.** CLOSED. `NotesMerging.app` is one set for every window (NotesCoordinator.swift:147-151, 225-228; AppController+Windows.swift:53). B skips the terminal until A lands and forgets the base.
  - **Text typed while the notes were unread.** CLOSED. `changed()` calls `reread()` first (NotesCoordinator.swift:62). The typed text is laid after the saved notes and saved on the switch.
- P07: 7 of 7. One new gap, above.
  - `repo_file_resolve` takes a key and returns a key (open_file.rs:72-77). `open_file_run` decodes it (47-49), `reveal` goes through `DiskName.url` (WindowUI/SystemPanelHost.swift:68-73), and `copyPath` refuses keys (58-61).
  - Project effects and layouts carry keys (files_panel/project/effects.rs:48-56, 124), and `launch::options` decodes them (launch.rs:28). `Opened { cwd }` is a key (workspace.rs:91-95).
  - Worktrees carry keys (git/worktrees.rs:38-40, 100, 113) and are drawn through `DiskName.shown` (WorktreeStore.swift:81-88).
  - The Settings page is keyed: `choice`, `project_page` and `known_projects` (settings_page.rs:84-90, 195, 240), `project_watch` (settings/project_watch.rs:36). Show .wormhole in Finder uses `project_dir_bytes` (settings_page.rs:259-261 ← SettingsBridge/RustProjectsBackend.swift:53-54 ← SettingsUI/Projects/ProjectPageView.swift:47).
  - The theme is written into the folder the gate passed (theme.rs:39-42, 47-64). The trust desk's root is a key (aux_backends/trust.rs:75).
- P08: 2 of 3 (one changed into a minor WRONG, above).
  - **Trust and switches per window.** CLOSED. Every gate reads the profile's one store under its lock, at the moment it acts:
    - `run_action`, `action_for_key`, `open_layout` and prompts go through `fresh` → `Hub::refusal` (service.rs:267-284; hub.rs:121-134).
    - The notes sync holds the switch and the approvals to the write (notes_sync.rs:88-105).
    - A terminal's start uses `stores::allows_in` (launch.rs:98; stores.rs:138-143).
    - `gh` uses `trust_status_in` (workspace_extras.rs:151).
    - The trust window's review, approve and revoke use `trust_status_in` / `change_trust_in` (aux_backends/trust.rs:102, 137-140, 148).
    - Worktree refusal and setup go through `Hub::load` → the shared stores (service.rs:79, 90, 241).
    - The Settings page and the theme read the same stores (settings_page.rs:173, 211, 215; theme.rs:54-59).
    - A revoke whose save failed holds in the process: `OnDisk::change` keeps `known` with the old stamp (stores.rs:82-93). Every change tells every window's tab (`tell_every_tab`, service.rs:61-64).
  - **Key filter on the panel's cached list.** CLOSED. It now requires `project.folder == hub.state.selectedPath`, then asks the core (`action_for_key` → `fresh`) at the moment of the key (ProjectActionKeys.swift:35-39).
- P01, P02, P04, P05, P09: none open. Waves 9 and 10 only added row frames, Reduce Motion through `\.reduceMotion`, spoken names for open-in icons, and keyed worktree folders (PanelUI/Shell/*, History/CommitRowView.swift, Files/FileEntryRow.swift, Stores/WorktreeStore.swift). Nothing removed.

## Can a revoke or switch-off still be ignored? (the brief's question)
No sequence found inside the process. Each gate locks the shared `Mutex<OnDisk<…>>` and reads the file's stamp when it decides. Locks are taken in one order (window hub → prefs → trust → notes file → bases), and no lock is held while a tab is told. Four residuals:
1. **Failed save, then another program writes the file.** A revoke whose save failed holds only until another program rewrites the trust file. The stamp then changes, `fresh` reads the file, and the project is trusted again (stores.rs:66-78). The sequence: the profile folder is unwritable → revoke P (the tab says it couldn't save) → another process writes `trusted-projects` → P is trusted in every window. The comment says this is meant, and GPUI cannot save either.
2. **Effects run after the check.** `run_action` returns effects that Swift performs later on the main actor. A revoke from another window that lands in that gap (milliseconds) does not stop them. GPUI checks and acts in one update.
3. **Worktree setup.** `worktree_setup` is judged before `git worktree add` runs (service.rs:235-245).
4. **Lossy approval keys.** The approvals key on the root's lossy text, `root_key` in core, the same as GPUI.

## Orders of events for the notes merge (P06)
- **Two windows.** The file lock serialises read → write, and the bases are shared, so no step is lost.
- **A terminal moved during a merge.** The destination waits on `NotesMerging.app` until the source lands and forgets the base. The next merge is a union, so nothing is taken out of notes.md.
- **A switch of terminal.** The panel's root is used only when the panel reads that terminal's folder. Otherwise the root is asked of that folder.
- **A step deleted while the file ticked it.** It comes back ticked. A step the file left alone stays deleted. Both match core merge.rs.

## New
- Nothing waves 9 and 10 broke in P01 to P09 that I found.
- Half-wired: `ProjectSettingsFollower` still uses the drawn folder (P07, above). The lane moved the Studio's path to keys and missed this one.
- Two notes on the fixes:
  - `theme_target` still returns the lossy root text. Only the Studio's mark shows it, and nothing acts on it (StudioProjectSource.swift:25).
  - `sync_notes` now holds the profile's prefs and trust locks across the notes.md read and write. Every other window's Project-tab read waits on that disk I/O. That costs time and cannot deadlock, since nothing takes these locks in the other order.

# Audit 7: terminal, sidebar, window, overlays, platform, studio, settings, aux

Tree read: `.claude/worktrees/swift-audit7` at `f578f1527` (waves 9 and 10 = `9db1b1da6..HEAD`). Swift paths under
`macos/swift/Sources/`, bridge paths under `crates/wormhole-ffi/src/`, GPUI under `crates/wormhole/src/`. Read only;
nothing built or run.

```
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
S01,100
S02,100
S03,100
S04,100
S05,100
S06,100
S07,100
S08,95
S09,100
W01,100
W02,100
W03,100
W04,100
W05,100
W06,95
W07,100
W08,100
W09,100
W10,100
O01,100
O02,100
O03,100
O04,100
O05,100
PL01,100
PL02,100
PL03,95
PL04,100
PL05,100
PL06,100
ST01,100
ST02,100
ST09,100
ST03,100
SD01,100
SD02,100
SD03,100
A01,100
A02,100
A03,100
A04,100
A05,100
A06,100
T23,100
T24,100
T25,100
T26,100
W11,100
T27,100
T28,100
T29,100
T30,100
T31,100
PL07,100
PL09,100
A07,100
S10,100
W12,95
```

## Rows below 100: gaps still open

S08 (95)
- GPUI plays a group card's insert pose → Swift: the code is there (`.transition(.listRow(...))` in the keyed `ForEach`,
  SidebarUI/Group/GroupRows.swift:45); whether SwiftUI plays it needs a Mac. Not worked by waves 9-10 (wave 9 only made
  the Classic fold a cut under Reduce Motion, SidebarSectionView.swift:44). Audit 5's number kept.

W06 (95)
- GPUI's Imports desk follows the window's profile → Swift: Settings' Imports desk still follows the shown profile. Not
  worked (only `ImportCards.swift` changed, a Reduce Motion chevron).

PL03 (95)
- A dev build without a bundle posts through the platform's script in GPUI → Swift only logs. Not worked (dev-only).

W12 (95)
- GPUI: each window draws and tells its programs its own profile's palette at all times (`settings_in(&self.profile)`)
  → Swift: on a switch BACK to a profile, `switchProfile` re-dresses its returning windows with the app's terminal look
  (Wormhole/Settings/ProfileChipBinding.swift:90 → ProfileLooks.swift:54 `terminals.dress`) before
  `followProfileSettings` moves the shared theme (:93) and before `followTheme`'s Task re-dresses them
  (Look/TerminalLook.swift:74). So their views tell the engine the profile just left's colours (`setPalette` →
  `set_colors`), then theirs a main-actor turn later: a program with mode 2031 on hears light/dark flip twice, and the
  window can draw a frame in the other theme. Transient; the lasting part of audit 6's residue 2 is closed.

## Audit 6's gaps, per row

- T20: 2 of 2 CLOSED.
  - Rows kept through Quit and End All: `finish(true)` now calls `end_keeping_row` on every made terminal (sets the
    slot's `kept`, then `close`, whose `HostSlot.closing` skips `on_end`/`Registry.forget` while `kept`;
    hosting/slot.rs:30-35, 76-81) and `Registry::unname_hosts(None)` (hosting/registry.rs:123-132), which rewrites each
    stored `SessionRecord` with `host_id: None` through `save_session` (position kept, notes/tasks rewritten from the
    loaded copy; groups and order are not in the session row and are untouched). `forget_all` is gone. The window
    saver now writes `terminal.persisted_host()` = core `persisted_host(id, lost, kept)` (windows/registry.rs:263-266,
    core ipc/hosts/launch.rs:96), so the final `saveAllWindows` in `applicationWillTerminate` writes None too. A start
    that lands during the quit (`taken`, `Fate::Ends`) does the same for its own row (desk.rs:452-458). Hosted text is now
    saved before the end (`beginQuit` closure: `saveTexts` then `finish`), and the next launch starts each row (no host
    named, so not in the plan) via `start(id, in:)` → `request` with `facts.savedText`, which `seed` now draws even with
    no `old_host` (desk.rs:362-379). `remember` keeps the stored row's name, notes, tasks, pin, split (registry.rs:94-106).
    Restart (`startAgain`) and a start overtaken by a restart use `endKeepingRow`; `finish_restore(listed:)` keeps a
    failed attach's or a fallen-back start's row. Sequence checked: ⌘Q with the box ticked → `goesNow` saves (hosts
    still named) → main-thread `quitting(true)` marks the gate → off-main `saveTexts`, `finish(true)` (end, unname,
    `end_all_live`) → `hostsEnd = .over`, reply → `applicationWillTerminate` saves again (no host) and skips
    `finishQuit`; views' `shutdown` → `close` → kept, nothing forgotten. Crash during the wait: rows still name ended
    hosts → next launch `Gone` → start again above saved text, row rewritten with the new host by `remember`. Plain quit
    and Mac restart unchanged. I found no path left that drops a row or a field without the person asking (the dev-only
    `--idle` check aside, as the lane says).
  - The wait is off the main thread: `Task.detached` over the windows' closures (AppController+Quit.swift:44-52),
    `.terminateLater` while `underWay`, second ⌘Q answered by the first. Reached from the menu, the quit question and
    `terminateUnattended`.
- T07: 2 of 2 CLOSED. `shortcut(for:)` no longer refuses ⌥/⌃ (TerminalMetal/Pointer/TerminalShortcuts.swift:37-43):
  exact `terminalAction` (Terminal|Edit scope, live: core `terminal_action`), else unless `isAppKey` (now any row of any
  scope, Look/TerminalLook.swift:103-112 = core `action_for(key, App)`, which overlaps every scope) the same key with ⌘
  alone. `.prompt` added; `onPrompt` → `TerminalFactory.jumpPrompt` → `WindowHub.jumpPrompt`
  (AppController+Windows.swift:67), so ⌘↑/↓ without blocks are taken quietly as GPUI's `Prompt`. Key by key (table.rs
  rows: ⌘C/V/A Edit, ⌘K ⌘⌫ ⌘← ⌘→ ⌘↑ ⌘↓ Terminal, ⌥⌘←→↑↓ pane focus, ⌃⌘↑↓ previous/next terminal, ⌥⌘↑↓ List move):
  - C, V, A, K, ⌫ with ⇧, ⌥, ⌃ or any mix: no row → both apps copy / paste / select all / clear / ^U.
  - ← →: ⇧⌘, ⌃⌘ and mixes → both send ^A / ^E; ⌥⌘ → both leave it to pane focus (exact action is FocusPane*, not a
    terminal command; GPUI returns None, Swift returns nil and the menu runs it).
  - ↑ ↓: ⇧⌘ (and ⇧⌥⌘ / ⇧⌃⌘ / ⌃⌥⌘, no rows) → both jump prompts; ⌥⌘ → both pane focus; ⌃⌘ → both previous/next terminal.
  - The flags are the four modifiers only, so an arrow's function flag no longer matters. Same in every case.
- O04: 4 of 4 CLOSED. (1) Part toggles and Reset write `title_row` in the window's own profile (`TitleRowSetting.show/reset`
  via core `show_title_part` / key reset, Wormhole/Settings/TitleRowSetting.swift; wired in `connectMenus` →
  `connectTitleRow`, AppController+Context.swift:41; WindowHub+Title.swift:29-30); every window, Settings and the next
  launch follow through `HubSettings.applyTitleRow` from both appliers; ⌃↩'s copy uses the same actions. (2) Zen ticks
  the person's own row (`ownRow`, WindowHub+Title.swift:35-38; TitleRowMenu.swift, TitleKeyMenu.swift). (3) "Show
  beside…" NOT REAL holds: `terminals_snapshot` walks `listing.entries` (folded members included), GPUI's
  `session_order` is `listing.order()`. (4) The terminal menu selects first, then greys (AppController+Context.swift:18-21).
- PL02: 1 of 1 CLOSED. `RecordsKeys` (DesignSystem/KeyRecording.swift) on both recorders (`KeyCapture.CaptureView`,
  `ShortcutRecorderView`); `ExtraKeys` and `ProjectKeys` refuse `event.goesToRecorder`; the recorder's
  `performKeyEquivalent` runs before the main menu.
- SD01: 1 of 1 CLOSED. `.onChange(of: document.increaseContrast) { document.refresh() }` (StudioUI/Window/StudioView.swift:68);
  `increaseContrast` reads the @Observable `ThemeStore`, which the system snapshot updates.
- SD02: 1 of 1 CLOSED. Six variants caught at a third from the core's pose (`RowMotion.tables.statusPose` →
  `status_motion_pose`, `ring` now carried, CoreRowMotion.swift), drawn part by part (`posed`, `rolled`, `statusCaught`
  for flare and nudge), SidebarUI/Samples/TileStatesSample.swift.
- W12: 3 of 4 CLOSED, 1 PARTLY. Residue 1: `makeSession` makes the profile's look before `WorkspaceHost.make`
  (AppController+Windows.swift:34), and `switchProfile` makes the target's look before its saved windows restore
  (ProfileChipBinding.swift:74); `ProfileLook.start` shares synchronously. Residue 3: `shareEngineColours` takes the
  theme as it is now (TerminalLook.swift:96-100), called after `followProfileSettings`. Residue 4: `engine_look::move_to`
  re-files and tells a moved window's registered terminals (engine_look.rs:51-69, from windows/rehome.rs:172-175);
  dressed ones are left to their views. Residue 2: PARTLY (above).
- S08 0 of 1, W06 0 of 1, PL03 0 of 1 (not worked).

## New

- W12 (above): the switch-back double flip comes from wave 9's `(leaving + back).forEach(dressForProfile)` running before
  the shared theme moves. Smaller than what it replaced.
- Nothing else I read in waves 9-10 removed or broke something that worked in these rows. Read: the hosting files and
  quit path in full; `ProfileLook(s)`, `ParkedAccess.wanted` with the `profileLooks` didSet (closes audit 6's parked
  follower edge); `TitleRowSetting`; key routing; `OnboardingWindow.watchesTrust` (closes audit 6's intro-watch item);
  `SettingsWindowController.windowShown` (only delegate set); `ThemeShelves` (a faithful move out of `ThemeCards`);
  `FontsScene`, `EntryRowView`, `SettingsNav` (statics for the control folder); `SettingRow` ⌃↩ menu (additive);
  `StatusBar` hit overhang; palette and quick panel glyphs (`told`; the quick panel's old `isBell` is the same rule);
  `PaneButton`/`PaneHeader`/`OpenInButtons` onto `IconButton`/`IconMenuButton` (`.inRow`, `.narrow`, `.chevron` exist);
  `GroupSwitcherView` rows; `ProfileAvatar` ink (PanesUI already depends on SidebarUI; `RGBA.contrast` made public);
  `GroupHeadView`/`HeadChevron` VoiceOver; `HomeFolder` replacing `expandingTildeInPath` (same rule as core's home).
- Side note for the P07/P08 reader: `Opened { cwd }` now crosses as a `names` key, and `Registry::remember` writes
  `what.cwd` (that key) into the row's `cwd`; identical to the path for any name that is text.
- Not counted (the owner's): all wave-9 Reduce Motion and VoiceOver changes in these folders are additive.

```
id,written
X01,95
X02,95
X03,95
X04,100
X05,100
X06,95
X07,95
```

Method: read only, tree `swift-audit7` at `f578f1527`, waves 9 and 10 = `9db1b1da6..HEAD`. Nothing built or run. Swift paths under
`macos/swift/Sources/`, GPUI paths under `crates/wormhole/src/`. Commits read whole: `38a1f9a39` (group head), `ba8d48f2e`
(Reduce Motion), `da929666a` (menu icon button, pane targets, hovers, avatar, status bar), `c3d2ad7a7` (segmented, open-in
names), `80dbdba5b` (rings), `e875604c0` (status glyph), `d2f212246` (grip), `8b4a2b760` (⌃↩), `ce9d411f8` (task checkbox),
`027f13efd` (Studio contrast). Nothing found that plainly cannot compile (checked: `RGBA` public in DesignSystem and
`RGBA.contrast` made public, PanesUI already depends on SidebarUI, `ActionMenuItem`, `TestInstance`, `Icon.grip`,
`Image(Icon)`, `menuChevron`, `Metric.statusBar`, `Chrome.statusItem`, `panelText` with arguments, `SpokenName.split`,
`AmbientTerminal.source.told`, `StudioDocument.increaseContrast/refresh`).

## Rows below 100: what is still open

### X01 (95)
- A row with a text field keeps the field's own Cut/Copy/Paste menu and opens no row menu there (`style/row_keys.rs:231` `own_menu`, used by `style/text_field.rs:182`) → Swift: nothing written for it. The row's `.contextMenu` wraps the whole row, and whether SwiftUI gives a field's right click to the field or to the row is not settled by reading (`SettingsUI/Rows/SettingRow.swift:75-82`). The lane says so too. PARTLY (⌃↩ is written, see below).
- Submodule row: the "open a terminal here" button is a child of the row that stays in the tree at rest (`style/submodule.rs:57-69` through `row_actions`, `style/buttons.rs:40-63`: no width, not removed) → Swift: the button exists only while the pointer is over the row, inside the row's own `Button` (`PanelUI/Changes/SubmoduleRowView.swift:32-36`), and the row has no menu. VoiceOver and the keyboard cannot reach it. New (my ten).
- A setting set by a project: the project's chip, no key, no menu, and no changed dot or reset arrow (`style/setting.rs:225-232`, state `Project` from `settings_window/registry.rs:531-535`) → Swift: chip, note, no key and no menu are there, but `RowState.isChanged` ignores the project (`SettingsUI/Rows/EntryRowView.swift:83-88`, `SettingsModel/SettingsModel.swift:81`), so the dot and the reset arrow still show and Reset puts back the person's value of a row the project overrides. The `Disabled` state (faint, control covered so it takes no click: `style/setting.rs:333-341`, used by `settings_window/sections/projects.rs:512-516`) is only an opacity on the Swift row; its picker still takes clicks (`SettingsUI/Projects/ProjectPageView.swift:30-35`). New (my ten), small.
- Choice card and folder colour choice: a radio button with a selected state, and the shared focus ring (`style/choice_card.rs:23-29`, `style/folder_colors.rs:54-60`) → Swift: a plain button with the `isSelected` trait and the system's focus (`SettingsUI/Rows/ThemeCard.swift:63-67`, `AuxWindows/Onboarding/ThemeCard.swift:44-47`, `PanelUI/Project/ColorChoice.swift:40-43`). The same words, not the radio role. New (my ten), small.
- Tint and neutral chip colours (`style/labels.rs:141-177`): unchanged. A look, not counted.

### X03 (95)
- Submodule enter button not reachable by VoiceOver (see X01). New.
- Theme cards, onboarding theme cards and folder colour choices are heard as "selected" buttons, not radio buttons (see X01). New, small.
- Not counted, for a person on a Mac: whether the right panel's tab strip, now a segmented `Picker` representation, keeps each tab's "n of 6" value (`WindowUI/PanelTabBar.swift:45-53`). The same for `Segmented` (`DesignSystem/Segmented.swift:45-52`), whose radio group has an `EmptyView` label (a nameless group).

## Audit 6's gaps closed, per row
- X01: 10 of 11 counted lines CLOSED, 1 PARTLY (row_keys: ⌃↩ written, `own_menu` not). The text field (owner's decision) and the tint/neutral chip colours (a look) are not counted. All six lines of audit 6's twenty-line table found INCOMPLETE or WRONG (status, popover, profile, groups, panes, section) are among the closed ones.
- X02: no gap listed by audit 6; waves 9 and 10 did not touch theme application in DesignSystem (the profile-colour fixes to the engine, `7204e77f3`, `c04f68947`, belong to other rows). 95 copied.
- X03: 3 of 3 CLOSED (task checkbox, open-in-editor names, group head's +). The tab strip's Picker shape stays "for a person to check", not a gap.
- X04: audit 6's only line (the find bar's toggles as a keyboard stop) is now the owner's decision, so nothing counted is open; the wave added the segmented control's one stop with arrows and ⌃↩ on settings rows. 0 open.
- X05: 2 of 2 CLOSED (the other chevrons, including the panel's `Disclosure`; the scroll-to animations).
- X06: untouched. 95 copied.
- X07: no gap listed by audit 6. New interface words in the diff go through `Words.text` / `panelText` (`swift-window.ftl:30-32`, `swift-panel.ftl` `panel-spoken-open-file`); the only new literals are gallery samples (`StudioUI/Gallery/ChromeSpecimen.swift:50-51`). `notes-task-not-done` was removed and `notes-task-done` is still used (`PanelUI/Stores/NotesStore+Walk.swift:15`). 95 copied.

## Audit 6's X01 lines, one by one
| line | verdict | the code, and where the app reaches it |
|---|---|---|
| text field (well, hover, ring, prefix, clear, multiline) | not a gap (owner's decision) | `FlatField` still only in the gallery |
| seven hand-drawn rings | CLOSED | the open-menu ring is `keyboardFocusRing` (`SidebarUI/Row/RowRings.swift:25-34`, on rows and heads through `ringWhileMenuOpen`); the Studio's ring specimen too (`StudioUI/Gallery/ShapeSpecimen.swift:39`). Every other stroke at focus-ring width is a selection mark or a field's own edge: grep of `focusRingWidth`/`borderFocus` strokes leaves the group popover and profile colour marks, the swatch mark, the recorder's edge, the onboarding card's current ring, the switcher card's current ring, the drop mark, and the find field (`outlined_always(ui.focus)` in GPUI too, `style/find_bar.rs:98`) |
| group head's + for VoiceOver | CLOSED | head is `.contain` with the core's words (`SidebarUI/Group/GroupHeadView.swift:73-79`); the name is a button "Edit group …" (`:106-108`), the chevron a button "Fold/Unfold group …" (`HeadControls.swift:26-29`, as GPUI's `role(Button)`, `style/groups.rs:226-230`), the + is in the tree whenever VoiceOver runs (`GroupHeadView.swift:51`), named "New terminal in …" (`HeadControls.swift:40`, key `swift-window.ftl:333`). GPUI's chevron also says expanded; the head's own words say open or folded |
| pane header buttons: 28 pt hits, hover on "…" | CLOSED | `PaneButton` is the shared `IconButton(...).inRow()` (22 face, 28 hit, `band.hover`, `text.primary`; `PanesUI/Panes/PaneButton.swift:15-20`); "…" is the new `IconMenuButton(...).inRow()` with the same face and hover (`DesignSystem/IconMenuButton.swift:52-70`, `PanesUI/Panes/PaneHeader.swift:73`); the tab's × uses `PaneButton` too (`PanesUI/Tabs/TerminalTabView.swift:65`) |
| form segments: one stop, arrows, radio group | CLOSED | `DesignSystem/Segmented.swift:37-52` (`focusable(interactions: .activate)`, shared ring, ← → ↑ ↓ Home End wrapping, `:55-70`; segments are tap targets, no stops; VoiceOver a segmented `Picker` = AppKit's radio group). Reached in the diff header (`PanelUI/Diff/DiffHeader.swift`) and the Studio filters |
| `narrow` + `menu_chevron` | CLOSED | `IconMenuButton.narrow(_:)`/`.chevron()` (`IconMenuButton.swift:38-42`, 12 pt glyph, hit's height, turns while its menu is tracked, cut under Reduce Motion or Animate icons off via `menuChevron`); Open in…'s chevron uses it with `band.hover` (`PanesUI/Title/OpenInButtons.swift:21-32`), as `buttons.rs:186-198` |
| popover rows' hover | CLOSED | the switcher's hit rows and both footer buttons are the shared `Row` (hover `state.hover`, press `state.active`; `PanesUI/Tabs/GroupSwitcherView.swift:64-85`, `DesignSystem/Row.swift:44-60`) |
| profile avatar letter contrast | CLOSED | `PanesUI/Tabs/ProfileChip.swift:90-111`: the window's colour below 4.5:1 on the profile's colour turns to black or white, whichever contrasts more, as `style/profile.rs:18-24` (`text_on`); the colour itself untouched |
| status bar items clickable across 28 pt | CLOSED (a Mac check remains) | `WindowUI/StatusBar.swift:166-177`: the content shape and hover reach `(statusBar - statusItem)/2` above and below, the layout stays 22 (negative padding). Whether SwiftUI hit-tests a shape past its laid-out frame here is the lane's own open check |
| palette `detail_color` | NOT REAL, holds | no `command_palette` code sets it (`grep detail_color`: only the quick panel `status_item/quick_panel/view.rs:260-282`, the hotkey preview and the gallery); the Swift quick panel colours its details (`PlatformUI/QuickPanel/QuickPanelRowView.swift:75-82`) |
| draggable note grip | CLOSED | `PanelUI/Notes/ReorderDrop.swift:40-48`: a faint grip under the pointer, first line tall, takes no clicks, hidden from VoiceOver, as `style/section.rs:131-146`; on notes and next steps (`NotesCard.swift:35`, `StepsCard.swift:40`) |
| status glyph's kinds | CLOSED | `DesignSystem/StatusDot.swift:8-15,49-71` adds running (working drawing, muted), bell (amber), shell (faint terminal); chosen by `told` exactly as `StatusGlyph::of` (`style/status.rs:46-57`) in the palette (`PaletteUI/ResultRow.swift:96-108`, `told` from `Wormhole/Platform/PaletteSourcing+Listed.swift:21`) and the quick panel (`QuickPanelRowView.swift:114-127`). `Terminal { unread }`, the ninth, is never chosen by `of`, so these lists cannot show it |
| `row_keys` ⌃↩ and `own_menu` | PARTLY | ⌃↩ on any control of a row with keys opens Copy Key, Copy as JSON, Reset to Default at the pointer, each copy announced and shown as Copied (`SettingsUI/Rows/SettingRow.swift:83-108`); whether AppKit-backed controls and the field editor pass ⌃↩ up to the row's `onKeyPress` needs a Mac. `own_menu`: see the gap above |
| tint and neutral chip colours | look, not counted | unchanged |

## Ten more `style/` files, state by state (not compared by audit 5 or 6)
Audit 5 took: row, icon tab and title tab, chip, count badge, icon button, segmented, card band, section label, key cap, fields, focus ring, banner, find bar, palette row, status tile (plus ten header reads). Audit 6 took the twenty in its table. I took these ten.

| GPUI file | Swift | verdict |
|---|---|---|
| `choice_card.rs` | `SettingsUI/Rows/ThemeCard.swift`, `ThemeCards.swift`; `AuxWindows/Onboarding/ThemeCard.swift` | INCOMPLETE (small): rest, chosen, hover, Increase Contrast edge (`contrastOutline` card edge, primary when chosen) and the arrow keys across a shelf (`ThemeCards.swift:136`, kept through the waves' refactor) are there; chosen as a ring and check, hover as an edge (looks). VoiceOver hears a selected button, not a radio button; the keyboard ring is the system's, not `focus_outline` |
| `folder_colors.rs` | `PanelUI/Project/ColorChoice.swift` | INCOMPLETE (small): 28 pt hit, 16 pt dot with hairline, chosen ring with gap, none as outline with a slash, `band.hover` only when enabled, held back to 50 % but the chosen one: all there. Radio role and the shared ring missing (selected button, system focus) |
| `rows/card.rs` | `SidebarUI/Group/CardSurface.swift`, `Row/TerminalRowView.swift` | right: card fill and edge, wash even or faded, solo card taking the selection's fill and edge, hover not on the selection, lifted card with shadow (`liftedFace`), the shared keyboard ring. `nav_row` is the system's list (owner's choice) |
| `rows/session.rs` | `SidebarUI/Row/RowLines.swift`, `SecondLineView.swift`, `RowMarks.swift`, `NameText` | right: name weight (semibold selected or unread), selected text colour, tone colours (selected muted for plain), marks, ⌘ keycap in place of the trail, name fade with room for the × under the pointer; a compact row's "needs input" word never shows in GPUI either (`sidebar/row/second_line.rs:34-36` gives compact rows a plain tone). The ⌘N cap is a second copy of `KeyCap` (`SidebarUI/Row/Keycap.swift`), same look |
| `tabs.rs` | `PanesUI/Tabs/TerminalTabView.swift`, `GroupSwitcherButton.swift` | right: open fill 14 %, closed 5 %, hover not on the open one, muted name on closed tabs, one weight, ⌘ cap (shared `KeyCap`), × on hover hidden under ⌘, VoiceOver "Close terminal" action (new in wave 9); switcher `band.hover`, alert dot, spoken label |
| `submodule.rs` | `PanelUI/Changes/SubmoduleRowView.swift` | INCOMPLETE: selected colours, icon, name, folder, words, kind letter there; the enter button only under the pointer and inside the row's button, no other way to it (see X01) |
| `setting.rs` | `SettingsUI/Rows/SettingRow.swift`, `EntryRowView.swift`, `Controls/ValueStepper.swift`, `ShortcutRecorder.swift` | INCOMPLETE: changed dot, reset arrow on hover or always, (i), chip, visible hint, found marks, keys and menu, stepper (system, owner's choice), recorder states there. Project-set rows still show the changed dot and the reset arrow; the Disabled state does not cover the control (see X01) |
| `setting_group.rs` | `SettingsUI/Rows/SettingGroup.swift` | right: head with aside, blurb, one card with `settings.group` fill, dividers drawn by the group, Increase Contrast `card.edge` |
| `tab_selection.rs` | `WindowUI/PanelTabBar.swift` | right: one shape springing tab to tab (400/30), at once under Reduce Motion (`:37-38`), no fill of its own on the selected tab, hover not on the selected, the ring on the selected tab while the strip has the keyboard |
| `rows/detail.rs` | `SidebarUI/Row/SecondLineView.swift` | right: the old words roll up out of the clip in their own colour, the new come from below, none under Reduce Motion. A change of tone with the same words does not roll in Swift (`.id(line.text)`), where GPUI rolls on either (`sidebar/row/face.rs:167-171`): minor |

4 of 10 incomplete, 6 right.

## New (waves 9 and 10 broke or left half-wired)
- Nothing found broken. Every fix above is reached from an app screen; the only gallery-only additions are specimens (`ChromeSpecimen.swift:48-55`).
- Half-wired or unverified, for a person on a Mac: ⌃↩ through AppKit-backed controls and text fields (`SettingRow.swift:84`); the status bar's hit past the face (`StatusBar.swift:170-173`); `Segmented`'s nameless radio group; `IconMenuButton`'s turned chevron depends on the pointer being over it when AppKit starts tracking the menu (`IconMenuButton.swift:65-66`), so a menu opened from the keyboard never turns it.
- Found in passing, not new: the submodule enter button, the project-set and disabled setting rows, and the radio roles above.

```
id,written
X08,95
X09,95
X10,85
X11,100
X12,90
X13,95
X14,90
X15,90
```

Tree `swift-audit7` at `f578f1527`, waves 9 and 10 = `9db1b1da6..HEAD`. Read only; nothing built or run. Swift paths under
`macos/swift/Sources/`, GPUI under `crates/wormhole/src/`, bridge under `crates/wormhole-ffi/src/`. No wave-9/10 commit touched
the bridge's control socket (`hook_server/`), `HookBridge`, the perf guards, `Isolation.swift`, the font folder or the legacy
launch code (diff stat empty for each).

## Rows below 100

### X12 (dev tooling): 90
Audit 6's list, item by item:
- The control folder is compiled into every build: a release build opens it for anyone who sets `WORMHOLE_DEVCTL` (GPUI: `devctl/mod.rs:1`, cargo feature `devctl` only) → Swift: unchanged, `Wormhole/Options+Parse.swift:42` reads the variable, `Wormhole/AppController+Open.swift:31-36` starts `DevCtl`; safe only while `Isolation.swift` refuses non-`Wormhole-swift*` data folders. OPEN (must close before the app runs as the product).
- `a11y` shows the tree VoiceOver gets (GPUI `devctl/mod.rs:450-453`) → Swift: wave 9 added `wakeSwiftUI` (`Wormhole/Driving/AccessibilityDump.swift`, sets `AXEnhancedUserInterface`/`AXManualAccessibility` on the app, then lays out); whether SwiftUI then vends its nodes is unknown (its own comment says so). PARTLY. `a11y live` is not a gap: it replays GPUI's kept views.
- `front` (GPUI `devctl/mod.rs:458-459`, `commands.rs:228`) → Swift: none. Small: Swift's injected keys already act as the key window (`Driving/InFront.swift`), and `window-front` exists; only a check that needs the real key window outside an injection (wave 9's own doubt about the field editor in `rename-edit`/Esc, TO-VERIFY wave 9 line "Least sure") has no escape hatch. OPEN.
- `studio ... modal=` names the Versions sheet (GPUI `devctl/mod.rs:423`) → Swift: `modal=update|-` only, `Driving/AppController+ControlStudioState.swift:22-25` (its comment: the Versions sheet is not seen). OPEN, small.
- `grow-hold <t>` holds a torn-off terminal's window part-way through its growth for a shot (GPUI `devctl/mod.rs:385-388`, `tear_off.rs`) → Swift has the growth (`Wormhole/TearOff/TearOffGrow.swift:64`, a 60 Hz timer) and nothing to hold it. OPEN, small (a look check).
- `pointer <x> <y>` sets the pointer's screen place so a row dragged out with `mouse-*` lands there (GPUI `devctl/mod.rs:352-355`) → Swift `pointer ROW COL` is a terminal cell (`Wormhole/AppController+Control.swift:49-54`); `tear-off-at ID X Y` lands a terminal at a screen point but skips the drag gesture (`AppController+ControlWindows.swift:82-87` → `land`). PARTLY: the drag-out path through the sidebar gesture cannot be driven.

Not gaps, checked against GPUI's source:
- `refresh`: drops the views GPUI caches between frames to catch a stale replay (`devctl/mod.rs:330-333`, `window/regions/`); Swift has no view cache of that kind.
- `reveal-test`: hands GPUI's view `displayLayer:` by hand because GPUI draws only when AppKit asks a visible view (`platform/macos/window.rs:228-235`); a GPUI workaround.
- `scroller overlay|legacy|system`: stands in for `NSScroller.preferredScrollerStyle` for GPUI's own drawn scroll bars (`devctl/settings.rs:249-257`, `ScrollerPrefs::stand_in`); Swift uses the system's scroll bars (owner's decision), which follow the preference themselves.
- `notice-material glass|frosted|solid|system` (`devctl/notices.rs:110-120`): the Swift chip has two surfaces, glass and a solid one under Reduce Transparency (`TerminalUI/Notice/NoticeChip.swift:49-56`), and `reduce-transparency on` reaches the solid one; frosted is GPUI's fallback for systems before Liquid Glass. (The file comment "always Liquid Glass", `AppController+ControlNotices.swift:4-5`, is slightly wrong.)
- `settings-preview <density>`: drives GPUI's hover preview of one density; the Swift Terminals scene shows all three at once (`AppController+ControlPages.swift:5-6`), so there is nothing to drive.
- `scaling`, `settle`: `geometry` prints increments, cell, grid, strip and `live` (`AppController+Control.swift:87-96`), and `live-resize-sim` runs a live resize to its end with `windowDidEndLiveResize` (`AppController+ControlWindow.swift:106-135`).
- `paints`: `state` prints `frames_drawn`, `ticks`, `wakes` per terminal view (`AppController+Control.swift:102-104`); a rate is two reads apart.
- `hosts` and `notices` meaning something else: renamed, not missing (`hosts-list` prints the cards GPUI's `hosts`/`hosts-text` print, `Driving/AppController+ControlTooling.swift:62-66`; GPUI's chip listing is `notice chip`, `AppController+ControlNotice.swift:13`). A GPUI script needs the new names.

GPUI commands with no Swift counterpart that audit 6 did not list: none. I went through the whole header of `devctl/mod.rs` (lines 16-483, 244 names) against every string in `Wormhole/`; besides the list above, each missing name has a counterpart under another name (audit 3's table, all present: `size`, `split`, `join`, `drop`, `ingroup`, `fold`, `tab`, `history-file`, `pasteboard`, `question`, `ambient-run`, `notice-log`, `quick-*`, `window-shot`, `settings-pick`, `settings-size`, `probe off`, `project-approve`, `action`, `scroll`, `sidebar`, `panel`, `select`, `note-add`, `task-add`), or: `hosts-text` ↔ `hosts-list`; `settings-project-menu` ↔ `settings-press project.search|project.settings.<name>` (the Swift page is a list with a search field, not a select: `SettingsUI/Projects/ProjectsPanel.swift:37-103`).

Readouts of wave 9 that do not read what the screen shows (the `title-menu` kind), from a sample of 20 of the lane's commands:
- `comment-at <row>` opens no dialog. It sets `ChangesStore.commenting` (`Driving/AppController+ControlChromeActs.swift:109-118`), but the dialog on screen is the row's own `@State editing` sheet (`PanelUI/Diff/DiffRowView.swift:95-113`), which nothing outside the row can open; `comment` then saves through the store (`AppController+ControlNotes.swift:53-58`). GPUI's `comment` acts on a dialog a real click opened (`devctl/mod.rs:302-303`). `ChromeActsTests.swift:106` passes without any dialog. WRONG.
- `trust-state` builds a fresh trust backend for the front window's project and reviews it (`AppController+ControlChromeActs.swift:70-76`); it does not read the open project commands window (`TrustWindow`, opened by `Wormhole/Aux/AuxController.swift:37-41`). It answers with no window open and can differ from a window opened earlier or on another root. A copy.
- `status-items` says the hosting word at every width (`WindowUI/Driving/WindowHub+Readouts.swift:50-52`); the bar shows the word only at `detail >= .numbers`, an icon alone below (`WindowUI/StatusBar.swift` `StatusHostingItem(word: detail >= .numbers)`, `WindowUI/StatusHosting.swift:51`). Already drifted. Its `stateWords` re-states `StatusBar.stateText` (`:61-67` against `StatusBar.swift:118-124`).
- `recents projects` names each root with `lastPathComponent` (`Driving/AppController+ControlUsageRecents.swift`), the menu with `RecentProjects.name` = `DiskName.shown(...)` (`Wormhole/Menus/RecentProjects.swift:51`): differs for exactly the non-text names wave 10 worked on.
- `theme-cards [filter]` and `switcher [query]` compute the listing for the argument (`SettingsUI/Driving/ThemeCardsReadout.swift:18-19`, `PanesUI/Driving/TabBarReadout.swift:46-69`); they never read the filter field or the switcher's search on screen. The listings themselves are shared (`ThemeShelves`, `TabSets.search`).
- `title-menu` (the lane's fix) reads `TitleKeyMenu.rowMenu`, the ⌃↩ NSMenu (`PanesUI/Title/TitleKeyMenu.swift:23-28`); the right-click shows the SwiftUI `TitleRowMenu` (`PanesUI/Title/TitleRowMenu.swift:11-21`). Two builds of one menu, the same items today.
- Same sources, guards re-stated (no drift found today): `settings-page` banners and row notes (`SettingsPageReadout.swift:60-74, 134-174` against `SettingsPage.swift:18-28`, `RowExtras.swift:39-157`); `files-viewer`'s card words (`FilesViewerReadout.swift:60-78` against `FileViewerBody.swift:18-59`); `sidebar-tip` (`ChromeActs.swift:92-106` against `WindowUI/SideColumns.swift:30-32`); `step-draft` (`:64-69` against `PanelUI/Notes/StepsCard.swift:26`); `TabBarReadout.currentGroup` (`:41-43` against `TabBarView.swift:80`); `diff-header`'s counts guard (`DiffReadout.swift:41` against `DiffHeader.swift:46`).
- Faithful: `settings-press` (a real click at the recorded frame), `changes-menu`/`history-menu`/`files-row-menu` (a real right-click at the row's recorded frame), `diff-spans` (`session.built`, what the view draws), `diff-header` badges (`DiffHeader.badges`, shared), `onboarding-state` (session plus the views' static functions), `tab-bar`, `usage`, `panel-kept`, `find-state`, `title-words` (`TitleFactsMaker`, the call `CentreTitle` makes), `exited-words` (`ExitedBar.words`).
All the wave-9 families are reached (`runExtra`, `AppController+ControlMore.swift:8-21`, from `run(line:)`), no command name is claimed twice, and the members they call exist (`TabBarActions.loudest/groupKeys`, `keptTerminals`, `isWatching`, `probeItems`, `DiffHeader.badges`, `LookStep.offered`, `recentFoldersJSON`).

### X08 (CLI and control socket): 95
- `events`, `read`, `blocks`, `wait`, `run`, `mouse`, `send-keys`, read flag by flag (`app/control/{read,stream,wait,input}.rs` against `hook_server/control/{read,stream,wait,input}.rs`; every flag is parsed by core's `ReadSpec::capture_spec`, `BlocksSpec::blocks_spec`, `EventsSpec::filter`, the wait plan, `RunSpec`, `MouseSpec`, the strokes): the same checks in the same order, the same grants, budgets, `read_check`, `run_check`, ⌃U then paste then Return after 60 ms, `claim_run`, output watched from each terminal's cursor. CLOSED by reading. Two differences, neither a wrong answer: `events` subscribes first and then takes the listing with `listing_and_rebase` (GPUI syncs the windows, subscribes, then lists, all on the UI thread); `wait` adds `core.kick()` for the moment between its look and its subscription.
- `run` showing a command's output only the first time and `blocks` listing nothing in the marking-shell fixture (features.csv X08) → still untraced; the handlers are the same, so a fault, if real, is below them (the reader's marks). OPEN.
- `send-text`/`send-keys` in GPUI also drop the view's selection and block cursor (`terminal_view/mod.rs:520-523`, `terminal_view/control_input.rs:48-53`); the bridge writes to the PTY and feeds `Typed` only (`hook_server/terminals.rs:53-65`). Small; whether the Swift view drops its selection on new output was not read.

### X09 (events, not polling): 95
- Audit 3-6's one named gap, notes sync on a repeating `Timer` (`Wormhole/Panel/NotesCoordinator.swift:116-118`): NOT REAL. GPUI syncs on its tick too, every third tick (`features/project/detect.rs:155-161`).
- The other loops read: `GitBridge/RepoChanges.swift:24` looks every 3 s only until the folder is a repository, then watches; `Hooks/HookHost.swift:53` and `RustWorkspaceBackend+Hosting.swift:227` are 60 s backstops (GPUI's own save interval); `UsageStore.swift:39` is a one-shot to the next word change; `HostsViewModel.swift:39` a slow look behind a watch; the status item animates only while working. Not compared: the quick panel's one-second clock while it is open (`PlatformUI/QuickPanel/QuickPanelController.swift:94-100`).

### X10 (performance guards): 85
No code changed (`FrameWorkGuardTests.swift`, `perf-guards.sh`, `swift-perf.json` untouched; the baseline still holds 9 nulls). Still missing: "a screen without blinking text asks for no clock" (GPUI `terminal_view/render/guards.rs:234`), half blocks without shaping (`:164`), and a stored Swift baseline that fails on a regression (`scripts/perf.sh`).

### X13 (fonts): 95. Not touched. No named gap; audit 3 held 5 for the unread evidence (`evidence/s3-themes-fonts/X13.md`).

### X14 (theme files, overlay, icon themes): 90. Not touched in its own code (wave 10 changed `files_panel/project/theme.rs` for non-text folder names, P07's). Still not followed: version-1 files opened in place by the Studio, an icon theme's light/dark family; one app-wide project theme (audit 4).

### X15 (legacy upgrades): 90. Not touched. Written and called, but never run (`LegacyLaunch.applies` is false for `Wormhole-swift*`). By the brief's "running as the product is a later step" this may count as no gap; the number is audit 6's.

## Audit 6's gaps found CLOSED, per row
- X12: 9 of 15 (all nine as not real or reached another way, checked against GPUI's source: refresh, reveal-test, scroller, notice-material, settings-preview, scaling, settle, paints, hosts/notices; none closed by a wave-9 change), 2 PARTLY (a11y, pointer), 4 OPEN (control folder in release builds, front, modal=versions, grow-hold).
- X08: 1 of 2 (the flag-by-flag read); the marking-shell fault stays untraced.
- X09: 1 of 1 (notes timer NOT REAL).
- X10: 0 of 2.
- X11, X13, X14, X15: no gap was listed as closable; untouched.

## New (waves 9 and 10 broke or left half-wired)
- `comment-at` opens no dialog on screen; the scenarios over it (`ChromeActsTests`) prove the store, not the dialog. While nothing sends `comment`, `ChangesStore.commenting` stays set with no dialog up, and `ProjectActionKeys.hasActiveDialog` (`Wormhole/Panel/ProjectActionKeys.swift:45`) holds back a project's keys.
- `trust-state`, `recents projects`, `status-items` (hosting) read copies, not what the screen shows (above); scenarios built on them can pass while the screen differs.
- `title-menu` still reads a second build of the right-click menu (the ⌃↩ one), not the menu a right-click opens.
- Nothing in the tooling the waves touched removed something that worked: `ThemeCards` moved its listing to `ThemeShelves` and reads it (a clean move); `key` gained `key-held` and view-owned keys go the keyboard's way (`AppController+ControlKeys.swift`); `press` now makes the selected terminal first responder when nothing in the window has the keys, which only changes where an injected key lands when the window itself was first responder.
