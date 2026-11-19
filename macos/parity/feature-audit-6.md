# Audit 6: what wave 8 closed (2026-11-14)

Audit 5 found the Swift app 93.8 % written and listed what was open. Wave 8 (five lanes) worked those lists; the branch
built and its tests passed (433 XCTest, 404 Swift Testing, the bridge's 372 Rust tests), and the first end-to-end
scenarios ran against a real instance. Five readers who had not written the fixes then read each one to where the app
reaches it. Nothing was built or run for this audit. Line numbers are of the branch `exp/swift-s5` at `9db1b1da6`.

**Written and reachable: 95.9 %** by the tracker's weights (audit 5: 93.8 %); 79 of 107 rows with no gap left.
Verified by a test or a driven check is a separate figure (`features.csv`, `macos/scripts/built.py`) and has not moved.

What is left, in points of the whole app (the row's number in brackets): X01 1.48 (85), X12 0.49 (90), P06 0.36 (80), P08 0.25 (90), SD01 0.22 (95), SD02 0.19 (95), P07 0.12 (95), P03 0.11 (95).

The readers found three faults that lose or re-grant something of the person's, all in what two windows of one profile
share: two windows writing one `notes.md` over each other, a switch turned off in one window coming back when the other
saves, and a project revoked in one window trusted again when another window approves any project. Wave 9's project lane
is written against exactly these (one store per profile in the process); they stay as audited here until read again.

| Area | Share of the app | Audit 5 | Audit 6 | Rows with no gap |
|---|---|---|---|---|
| crosscut | 21.5 % | 86.2 % | 88.6 % | 1 of 15 |
| panel | 15.4 % | 91.0 % | 94.5 % | 5 of 9 |
| terminal | 13.4 % | 100.0 % | 99.1 % | 29 of 31 |
| studio | 9.6 % | 91.6 % | 95.8 % | 1 of 3 |
| window | 9.2 % | 97.2 % | 99.5 % | 10 of 12 |
| sidebar | 7.9 % | 98.2 % | 98.9 % | 9 of 10 |
| aux | 7.8 % | 96.4 % | 100.0 % | 7 of 7 |
| settings | 7.3 % | 97.5 % | 100.0 % | 7 of 7 |
| overlays | 4.3 % | 98.6 % | 98.6 % | 4 of 5 |
| platform | 3.6 % | 97.0 % | 98.6 % | 6 of 8 |

## Right panel (P01 to P09), audit 6

Tree `swift-audit6` at `9db1b1da6`. Swift paths are under `macos/swift/Sources/`. Bridge paths are under `crates/wormhole-ffi/src/`. GPUI paths are under `crates/wormhole/src/`.

id,written
P01,100
P02,100
P03,95
P04,100
P05,100
P06,80
P07,95
P08,90
P09,100

### Gaps still PARTLY / OPEN / WRONG

##### P06 (80)
- **WRONG (two windows, data loss).** In GPUI, one UI thread reads, merges and writes notes.md one terminal at a time (features/project/notes_sync.rs:95-123). In Swift, each window has its own `ProjectService`, and so its own `Hub` mutex and its own `NotesFile` (FilesBridge/FilesBridge.swift:72; files_panel/project/service.rs:30,55). `sync_notes` holds only that window's lock across read → merge → write (files_panel/project/notes_sync.rs:81-99, 138-163). `write_notes` has no stamp check (core project/mod.rs:383-392). The exact sequence:
  1. Windows A and B are in the same profile. A shows T1 and B shows T2. Both terminals are in trusted project P. notes.md holds F0, and both bases hold F0.
  2. The person adds step "a" in T1 and ticks "b" in T2 within the same 2 s.
  3. Both timers fire within a few ms of each other. A reads F0 and writes F0+a. B has already read F0, so it writes F0+b✓ over A's write. The bases are now T1 = F0+a and T2 = F0+b✓.
  4. On A's next merge, "a" is in T1's notes and in its base but not in the file. It reads as deleted in the file and is removed from T1's screen and database. It never reaches notes.md.
  - Wave 8 shared the bases (hub.rs:171-186) but nothing serialises the file across windows.
- **WRONG (two windows).** GPUI has one `ProjectHub` per profile (features/project/hubs.rs:54). In Swift, the project prefs and the trust store are a copy per window. `prefs` is loaded once (hub.rs:42) and never re-read. `sync_notes` judges trust without `reload_trust` (notes_sync.rs:82-84).
  - Sequence 1: switch Notes sync off for P in window A. Window B keeps merging P's notes.md and its tab still shows the switch on. Then switch any project feature in B: `set_feature` → `hub.save()` writes B's stale prefs whole (service.rs:155-163; core project/features.rs:186-191), which turns P's switch back on in the file.
  - Sequence 2: revoke P in another window (or in Settings, which uses the first window's service, Wormhole/Settings/SettingsPlatform.swift:47). A window whose only terminal is the shown one in P never calls `project()`, so it never reloads trust, and it keeps reading and writing P's notes.md. A window with more terminals reloads within about 10 s (through `root(of:)`).
- **Possible, not shown to happen: a merge against the wrong project after a terminal switch.**
  - GPUI keeps a root per terminal (notes_sync.rs:58). Swift takes the shown terminal's root from the panel's project: `found = id == shown ? hub.panel.project.state?.root` (Wormhole/Panel/NotesCoordinator.swift:127).
  - The panel follows the selection from a SwiftUI `onChange` (WindowUI/WorkspaceRootView.swift:60-62). `shown` follows through an observation `Task` (NotesCoordinator.swift:46-50). Nothing checks `hub.panel.project.folder == terminal.readPath`.
  - The sequence: the person switches from X (project P1) to Y (project P2). The coordinator's Task sets `shown = Y`. A sync continuation runs before SwiftUI's update calls `panel.follow`. Y is then merged with P1's notes.md.
  - The base's root does not match, so the merge keeps everything of both sides. Y's items are written into P1's notes.md and P1's lines go into Y. Y's base now names P1, so the next merge with P2 is also a union, and P1's lines reach P2's notes.md.
  - Whether this ordering happens depends on main-queue scheduling. I could not prove it either way without running the app.
- **WRONG (rule differs from the core).** A step the person deletes during the merge, while the file ticks it:
  - Swift: the deletion wins. `rebase` skips every answer item that was in `sent` (PanelModel/Notes/NotesRebase.swift:74). The next merge then deletes the line, and the file's tick, from notes.md.
  - Core and GPUI: "deleting a task on one side removes it unless the other side changed it" (core project/notes/merge.rs:41-42, test at 225-234).
  - Notes are not affected: a note the file edited has new words, so the rebase adds it back.
- **PARTLY (narrow race).** A terminal moved to another window during a merge.
  - The answer is dropped and the base is forgotten at landing (NotesCoordinator.swift:142-148). Because the bases are shared now, the destination window B can run its step between A's step and A's landing.
  - In that gap, B merges its model from before A's merge against the base A just set (which includes the file's new lines). Those lines read as deleted by the person and are removed from notes.md. The window is a few ms.
- **Data loss, minor.** The shown terminal's notes could not be read when it was shown (`unread`).
  - What the person types is not saved (NotesCoordinator.swift:59, 70). It is only merged back in `reread()`, which runs only from `sync()` (line 124).
  - With the sync timer off (project folders or notes off, line 82), switching away loses what was typed.
- **Minor.** The database cannot be read at landing for a terminal that is not shown. The base is forgotten (lines 158-163), and the next merge brings back, once, a line the file had deleted. This is the lane's stated trade.
- **Cost, not correctness.** Every terminal that is not shown is read from SQLite on the main actor every 2 s (`holding`, line 179). With 500 items that is a few ms per terminal. GPUI keeps the notes in memory.

##### P03 (95)
- The comment form is a dialog in GPUI (features/git/review.rs:92-178). Swift: still a popover (PanelUI/Diff/DiffRowView.swift:100, Diff/CommentEditor.swift). Untouched by wave 8.

##### P07 (95)
- **PathBuf throughout** (GPUI). Swift (wave 8) closed the reads: the terminal's folder is a key (`workspace_cwd.rs:10-15` ← `WorkspaceBridge/TerminalsMapping.swift:29` → `TerminalSummary.readPath`, `WindowUI/Chrome.swift:75` → `PanelController.follow`), decoded by `names::folder` in git (service.rs:71-87, 358, watch.rs:54), the project service (service.rs:99, 140, 169-172, 232, 241, 254; overrides.rs:34; theme.rs:32) and worktrees (worktrees.rs:154, 168, 176, 279).
- Still text or lossy, inside the panel:
  - `repo_file_resolve` does `Path::new(root)` on a key (open_file.rs:71-73). In a repository whose root is not text, Changes, Diff and History's Open, Reveal and Copy Path silently do nothing (PanelUI/Stores/ChangesStore.swift:181, Diff/DiffHeader.swift:75, Changes/FileRowView.swift:50).
  - `repo_file_resolve` also returns `to_string_lossy`. A link that resolves to a name that is not text is handed on as U+FFFD text.
  - Project actions and layouts open their terminals in `to_string_lossy` folders (files_panel/project/effects.rs:48-49).
  - A worktree's `repo`, `folder` and plan are lossy (git/worktrees.rs:37-39, 99, 114).
- Still text or lossy, outside the panel: the workspace's `Opened { cwd }` (workspace.rs:92). On the Settings page, roots go through `Path::new` on keys (settings_page.rs:197) and come back lossy (84, 241).
- **Crafted sibling, low risk.** `theme_target` returns a lossy root and `write_theme` opens it again (theme.rs:45, 50-51). In a repository that has both `caf\xE9/.wormhole` (trusted) and a planted `caf\u{FFFD}/.wormhole`, the person's theme is written into the planted, untrusted one. Only volumes that allow names that are not text (exFAT, FAT, SMB) are exposed.
- **Keys themselves: no escape found.**
  - A key decodes only bytes ≥ 0x80, so no `/` or NUL can be smuggled in (names.rs:46-52, DiskName.swift:35).
  - A broken key names `/\0`. Every system call on it fails. Its only parent is `/`, where `find_project` would look for a `/.wormhole`, which only root can create.
  - Swift draws and copies keys through `DiskName.shown`, and refuses them for Copy Path and Insert (WindowUI/SystemPanelHost.swift:57-60; PanelUI/Files/FileViewerView.swift:95).
  - A NUL inside a cwd reported by a program (OSC 7 `%00`) passes `path_key` unescaped and decodes to one high byte. That reads a folder the program could name directly, so it adds no reach.

##### P08 (90)
- **One project hub per profile** (features/project/hubs.rs:54). Swift: a trust store and prefs per window.
  - `approve` and `revoke` change the window's in-memory trust list and save it whole, without `reload_trust` (files_panel/project/trust.rs:47-97). The project commands window does re-read first (aux_backends/trust.rs:130-148).
  - Sequence: revoke P in window B (or in the Settings page). Then, in window A, whose tab has not re-read trust since, approve Q with Touch ID. A's save writes P back: **a revoked project is trusted again** without being asked.
  - The Project tab's switches are lost between windows the same way (P06, second item). Not listed by audit 5. This predates wave 8, but wave 8 shared only the bases.
- The key filter reads the panel's cached `state.actions`, not the actions of the terminal of the pane the key was typed in (Wormhole/Panel/ProjectActionKeys.swift:29-31).
- Repeats (`isARepeat`) are passed on to the view instead of being claimed (ProjectActionKeys.swift:53). Unchanged from audit 5.

### Audit 5's gaps closed
- P01: 1 of 1. The tab role is a segmented `accessibilityRepresentation`, reached in the strip (WindowUI/PanelTabBar.swift:45-52).
- P02: 1 of 1. A chip click on a folder already read as no repository leaves no request (WindowUI/Hub/WindowHub+Branch.swift:11-12). A later read clears it (ChangesStore.swift:145).
- P03: 0 of 1.
- P04: none open (looks-only change to the filter chip).
- P05: 2 of 2.
  - The window that is gone has its sheet mark taken down (Wormhole/Hooks/HookHost.swift:107-113).
  - A stored list that cannot be read is never written (git/worktrees.rs:220-234; test at line 15 of its tests).
- P06: 6 of 7.
  - Closed: the duplication; ticks, adds and deletes during the merge; reordering; an editor opened after the send; a window moved to another profile; `departed`.
  - Partly: a terminal moved to another window during a merge.
  - New gaps above.
- P07: 0 of 1 fully. Partly: every panel read is closed; the gaps listed above are left.
- P08: 1 of 3 of audit 5's differences (`hasActiveDialog`, ProjectActionKeys.swift:27, 36-39). One new gap (per-window trust and prefs).
- P09: none open.

### Orders of events for the notes merge (P06)
- **(a) A terminal not on show, synced twice with nothing changed.** Nothing grows.
  - `sent` and `now` are two reads with new ids, so `rebase` runs. Every item of `now` finds its words in `sent` and in the answer and keeps its own id. Nothing in the answer is new.
  - Then `merged == now` → nothing is saved (NotesCoordinator.swift:165-166).
  - On the second sync, `local == base` and `theirs == base` → `plan_sync` returns None (core sync.rs:56-58).
  - Screen, database and notes.md are unchanged.
- **(b) Two items that say the same, one ticked during the merge.** Kept as two, and the tick stays on the one ticked.
  - They are told apart by their place among equals (NotesRebase.swift:43-51), as `match_exact` does.
  - Edge case: the person deletes the first "x" while the file ticks the second. The keys shift, so the remaining "x" is matched with the first, and the file's tick is lost. The core has the same blindness across edits made between syncs.
- **(c) An item edited by the person and by the file at once.** Both versions are kept, once each (the rewritten note's id is renewed, line 76).
  - The next merge writes the person's version into notes.md. This matches the core's rule.
- **(d) An item deleted by the person that the file changed.**
  - A note the file edited comes back with the file's words, as in the core.
  - A step the file only ticked stays deleted, and the next merge deletes it from notes.md. This differs from the core (see P06 above).
- **(e) Switched away, closed or moved mid-merge.**
  - Switched away: the switch saves the panel's model. The answer is laid over the database's copy by words and stored for that terminal. The terminal shown now is untouched. Correct.
  - Closed: the base is dropped and nothing is stored. Correct.
  - Moved to another window: nothing is stored and the base is forgotten. The next merge keeps both sides. A narrow race can delete the file's new lines (see above).
- **(f) Two windows of one profile.** The bases are shared now and correct. The notes.md read and write is not serialised, which loses a step (exact sequence above). Prefs and trust are stale per window.
- **(g) The store unreadable once, then readable.**
  - Not shown: it is not sent. At landing, the base is forgotten and nothing is stored. The next merge may bring back, once, a line the file had deleted.
  - Shown: the panel shows nothing and saves nothing. What was typed is merged in on the next sync. It is lost if the person switches away first or if the sync timer is off.
- **(h) An empty line, or an item that is only white space.**
  - Steps cannot be blank: TaskList trims them, and the core drops `- [ ]` with no text.
  - Blank notes stay out of the document and are kept locally, moved after the others (as GPUI does). Their key is "", told apart by place. Nothing is doubled.
  - Inner whitespace in a step is normalised by the core, which renews that step's id once.
- **(i) 500 items.** Correct.
  - `rebase` is O(n²) on keys, a few ms. The core's `match_exact` and `emit` are O(n²) too.
  - The cost is the per-2-s main-thread SQLite read for each terminal that is not shown.

### Worktree store (brief item 3)
A SQL read error is now an error: nothing is written back (worktrees.rs:201-234, tested). JSON that does not parse is still read as an empty list (core git/worktree/mod.rs:199-201 `unwrap_or_default`), and the next `create` writes a list of one over every remembered worktree. GPUI does the same (app_state/profiles.rs:232), so this is not a parity gap, but the rule is not met. A `create` that follows a read error makes the worktree without recording it: Wormhole will not offer to remove it later, and later terminals in it get no `WORMHOLE_PORT` (`env_for` reads the store, line 239-250).

### New
- Wave 8 shared the notes bases per profile (hub.rs:171-186) but left half-wired the rest of the window's project hub:
  - Prefs and trust are a copy per window and are saved whole. This allows a revoked trust to come back and switches to be lost (P08, P06).
  - The notes.md read → write has no lock across windows (P06).
- The rebase rule written in wave 8 makes the person's deletion win over the file's tick of the same step. The core keeps the step (P06).
- The shown terminal's merge root comes from the panel's project state with no check that it is that terminal's folder (pre-existing, possible; P06).
- Nothing found broken elsewhere in wave 8's panel changes. Sampled: BandButton/IconButton, the CountBadge swap, the History filter chip, PanelScroll's scroll edge, `notes_store.read`, the HookHost and the action-key dialog checks.

## Sidebar, window and terminal (audit 6)

id,written
T01,100
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
T16,100
T17,100
T18,100
T19,100
T20,90
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
S09,100
S10,100
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
W11,100
W12,95

Tree read: `.claude/worktrees/swift-audit6` at `9db1b1da6` (wave 8 = `937014da2..HEAD`). Swift paths under
`macos/swift/Sources/`, bridge paths under `crates/wormhole-ffi/src/`. Read only; nothing built or run.

T07 and T20 were 100 in audit 5 and wave 8 did not open them as gaps; they drop because the brief asked me to read the
quit path and the ⌘-key routing, and both turned up a difference from GPUI (T20's is older than wave 8, see New).

#### Rows below 100

T20 (90)
- Quit and End All keeps every terminal's row (its name, folder, notes, tasks, saved text) and only drops the host id, so
  the terminals come back next launch as new shells (`persisted_host` returns None while `ending_hosts`,
  window/session.rs:179-186; core ipc/hosts/launch.rs:96; `finish_quit` saves first, window/windows.rs:413-421) → Swift:
  `HostedTerminals.finish(end_hosts: true)` calls `registry.forget_all()`, which DELETES every session row that names a
  host (hosting/registry.rs:117-119 → `delete_sessions_except` → core store/sessions.rs:147-158 deletes notes, tasks,
  saved text, the tab and the session), and each `terminal.close()` runs the slot's `on_end` → `registry.forget(tab)`
  (hosting/desk.rs:281-285, hosting/slot.rs:27-31, desk.rs:408-413). `saveAllWindows()` ran just before
  (Wormhole/AppController.swift:170-173), so its rows are deleted after it. Result: with Persistent terminals on, Quit and
  End All (or the quit question with the box left ticked) loses every hosted terminal of every window, with its notes
  and tasks. No test checks the rows (`EndToEndTests/HostingTests.swift:30-41` checks only that no host is left; the ffi
  test `ending_at_quit_ends_a_host_still_starting` asserts "no row names it" as intended). Not wave 8's: `forget_all` in
  `finish` predates it.
- The End-All quit runs on the main thread in `applicationWillTerminate`, window after window: each desk waits up to 6 s
  for its own starts under way (hosting/quit.rs:62-70), then `end_all_live` (5 s per host, in chunks); GPUI ends the hosts
  on a background thread and quits when they answered (window/windows.rs:413-433). Bounded (no hang), but the app is
  unresponsive for up to ~6 s × windows with a slow host. Minor.

T07 (95)
- A ⌘ key with ⌥ and/or ⌃ that no table row uses falls back to the same key with ⌘ alone (terminal_view/input.rs:105-110:
  exact `terminal_action`, else `terminal_action(⌘+key)` unless an entry of any scope uses the key) → Swift: `shortcut(for:)`
  returns nil for any ⌘ key that also has ⌥ or ⌃ before it asks the table (TerminalMetal/Pointer/TerminalShortcuts.swift:32);
  the menus have nothing for those keys and `keyDown` passes platform keys to `super` (TerminalMetal/Keyboard/KeyDown.swift:17),
  so they beep. CONFIRMED. Keys that differ (base table, `keys/keymap/table.rs`): ⌥⌘ / ⌃⌘ / ⌃⌥⌘ / ⇧⌥⌘ / ⇧⌃⌘ with C (copy),
  V (paste), A (select all), K (clear), ⌫ (delete line); ⌃⌘ / ⌃⌥⌘ / ⇧⌥⌘ / ⇧⌃⌘ with ← / → (line start / end; ⌥⌘←/→ are
  pane focus in both); ⇧⌘ / ⇧⌥⌘ / ⇧⌃⌘ / ⌃⌥⌘ with ↑ / ↓ (previous / next prompt; ⌃⌘↑/↓ and ⌥⌘↑/↓ are app/pane keys in both).
  The ⇧⌘↑/↓ case is Swift's for another reason: `TerminalShortcut` has no prompt case (TerminalShortcuts.swift:13-23), so only
  the exact ⌘↑/⌘↓ work, through the Edit menu. Keys that agree: every ⌥/⌃ key a row uses (⌥⌘←→↑↓, ⌥⌘L, ⌃⌘S, ⌥⌘I, ⌃⌘1-6,
  ⌥⌘J, ⌃⌥⌘0-9, …) goes to the app in both; ⇧⌘ letters behave the same (Swift's `isAppKey` checks only `.app` scope where
  GPUI's `action_for(key, App)` matches every scope, but no base-table key makes that differ).
- Smaller: wave 8 greys the six block items without shell integration (WindowUI/Menu/MenuStateMaker.swift:99-100), so ⌘↑/⌘↓
  without blocks reach `keyDown` and beep; GPUI takes them as `Prompt` and swallows them (input.rs:122-125).

S08 (95)
- Unchanged by wave 8 (only a TO-VERIFY line): the group card's insert pose is `.transition(.listRow(...))` in the `ForEach`
  keyed by id (SidebarUI/Group/GroupRows.swift:45), the same as a loose row (Group/SidebarSectionView.swift:24). Code is
  there; whether SwiftUI plays it needs a Mac. Kept at audit 5's number.

W06 (95)
- Not worked in wave 8: the Imports desk (Settings') follows the shown profile, not the window's; audit 5 left it as the
  one open item. Unchanged.

W12 (95)
- The engine's per-profile look is CLOSED for the main case: `LOOKS` per profile (engine_look.rs:30-71), `launch::options`
  reads `current(profile)` (launch.rs:47-48), every start path passes its profile into `from_parts` → `follow(terminal,
  profile)` (terminal.rs:190-196, 208-216, 363-372, 393-397, 429-432; hosting/desk.rs:414-421 with the desk's profile, which
  `follow_profile` moves), `set_colors` takes a dressed terminal out of the profile's list (engine_look.rs:78-84). Swift:
  `TerminalLook(engineProfile:)` shares for the shown profile (AppController.swift:78-79) and each `ProfileLook` for its own
  (Settings/ProfileLook.swift:20-21, `start` → `followTheme`), each look's `terminals()` are only its own windows
  (ProfileLooks.swift:37, 65-67). Traced: B's terminal in a B window with A shown starts in B's colours when B's look exists;
  A's theme change reaches only A's windows and viewless A terminals; B's change reaches B's views and viewless B terminals;
  a move to A re-dresses the views (`dressForProfile` → `dress` → `setPalette` → `setEngineColors`, MetalTerminalView.swift:168-170).
- Residue 1: the FIRST window of a profile opened in a run starts its terminals before that profile's look exists:
  `WorkspaceHost.make` restores and opens a terminal (Wormhole/WorkspaceHost.swift:72-74) at AppController+Windows.swift:32,
  and `dressForProfile` (which makes the `ProfileLook` and shares its colours) runs at :46. So `current(B)` is None and an
  in-process start gets the engine's built-in defaults (not even A's) until its view is made; a hosted start races it.
- Residue 2: parked windows of a profile Wormhole left keep the app's look (ProfileLooks.swift:65-67 says so), so when the shown
  profile switches, `followTheme` → `setPalette` tells their out-of-sight terminals the NEW profile's colours and scheme
  (Look/TerminalLook.swift:85-89); GPUI's hidden windows keep their own profile's palette (`settings_in(&self.profile)`).
  Programs in them (mode 2031) flip light/dark while hidden.
- Residue 3: `switchProfile` calls `terminalLook.shareEngineColours()` synchronously (Settings/ProfileChipBinding.swift:90)
  while the palette is still the profile left's (the theme follow is an async `Task`, TerminalLook.swift:73-75), so for a
  moment the new profile's `LOOKS` entry holds the old colours; and its saved windows were restored just before
  (`openSavedWindows`, :84), starting with `current(new)` = None or stale. Transient.
- Residue 4: a window moved to another profile: terminals with no view yet stay under the old profile in `TERMINALS`
  (answer its colours, hear its changes) until a view dresses them. Minor.

#### Audit 5's gaps closed, per row
- S08: 0 of 1 (a Mac-only check, not worked; not counted against beyond audit 5's 95).
- S09: 1 of 1. Sets, current set, per-set alert and the switcher's dot are the core's: `WorkspaceModel::tab_sets_view`
  (core workspace/snapshot/tabs.rs, using `tabs::tab_sets`, `current_set`, `others_need_you`) → ffi `Workspace.tab_sets`
  (workspace_tabs.rs:28-33) → `TabSetFacts` (WorkspaceBridge/TabSetsMapping.swift) → `WindowHub.tabBar`
  (WindowUI/Hub/WindowHub+Sidebar.swift:41-68: sets, `current`, `alerts:`), `TabBarModel.make(alerts:)` uses them
  (PanesModel/Tabs/TabBarModel.swift:85-104). `TabSets.of` and the own-alert fallback run only when `tabSets()` is nil (the
  sample). "Ungrouped" is the catalog's word for `group == nil`.
- W01: 1 of 1. Only `.splitRight/.splitDown` record the new terminal (`opensSplit`, WindowHub+Panes.swift:33-38; `send`,
  WindowHub.swift:116-119); the tracker plays only when the one new pane is that terminal and was not weighed before
  (SplitOpeningTracker.swift:43-48), so a drop (`.movePane`) or a join (`.joinSplit`, WindowHub+Panes.swift:47-49) is a cut,
  including a pane that left and is dropped back. Every split entry (⌘D/⇧⌘D via `split(_:)`, title buttons, pane menu, More
  menu, terminal menu, row menus `.splitRight(id)`) goes through those two actions.
- W03: 1 of 1. Who is named and how many are counted are the core's (`Summary::needs_you_named`, `NEEDS_YOU_NAMED = 3`,
  view/layout/status_bar.rs:128-193 → ffi status_counts.rs `named`/`more` → CoreStatusRules.swift:12-15 →
  StatusSummary.swift:68-69); Swift only words them from the catalog ("then", "and N more", "go there"/"go to the first",
  the last chosen by whether anyone besides the first is named, as the core's `rest.is_empty()`).
- W06: 0 of 1 (Imports desk, not worked).
- W12: 1 of 1 PARTLY (per-profile engine look written and reachable; residues above).
- T20 (wave 8's quit work, read in full): `QuitGate` is sound. `quit()` sets the fate before `finish` reads `made`, and
  `taken` pushes into `made` before it reads the fate, so a start that ends on either side of the quit is seen once
  (desk.rs:427-439, 276-280); an ending quit waits up to 6 s for starts/attaches under way (quit.rs:56-71), `taken` closes
  what comes after (`Fate::Ends`), then `end_all_live` ends every live host of the folder. Cases: host still starting → ended
  (or, if it takes over 6 s, ended by `end_all_live` once it listens, or by its own 10 s no-Spawn timeout); host that never
  answers → start gives up at 5 s, the host is signalled by `wormhole_term`, and HostConnect then starts a local shell in the
  exiting app (dies with it); host that dies during the quit → `end_all_live` skips or fails on it, nothing left; two windows
  → each desk waits for its own under-way starts in turn, the first `end_all_live` ends all hosts; second ⌘Q during the wait
  → the main thread is blocked in `applicationWillTerminate`, nothing re-enters, the process exits after. A plain quit does
  not wait (as GPUI): a start under way dies with the process; a host already given its program runs on unnamed and is
  adopted into Recovered next launch (same as GPUI). No `wormhole-host` is left that should be gone, none ended that should
  stay; the app cannot hang (all waits bounded). The data loss above is the gap.

#### New
- T20 (pre-existing, not wave 8): Quit and End All deletes every hosted terminal's row with its notes, tasks and saved text
  (above). Highest priority in these areas: it loses the person's notes.
- T07 (pre-existing, confirmed for the scenario writer): ⌥⌘C / ⌃⌘C and the other ⌥/⌃ variants of the terminal's ⌘ keys beep
  instead of copying, pasting, clearing or moving; ⇧⌘↑/↓ do not jump prompts.
- W12 residues 1-3 are what wave 8's per-profile look left: start order of a profile's first window, parked windows taking
  the shown profile's colours, and the switch's synchronous share of the old palette.
- Wave 8's block-menu greying (MenuStateMaker.swift:99-100) makes ⌘↑/⌘↓ beep without shell integration (GPUI swallows them).
- Not counted (a look): the closed switcher's dot is red when only a failure is elsewhere (TabBarModel.swift:103 →
  GroupSwitcherButton.swift:29); GPUI's is always amber (style/tabs.rs:116-124).
- Nothing else broken in what wave 8 touched here that I sampled: the title-row and find-bar buttons moved onto the shared
  `IconButton` (the wanted-panel dot kept via `.dot`, TitleButton.swift; the find toggles via `.toggled`), the panel tab
  strip's VoiceOver radio group, `readPath` from the core's cwd (workspace_cwd.rs; Chrome.swift `selectedPath`), the
  `pickBranch` non-repository guard, `ExtraKeys` refusing keys while a sheet is up, `WorkspaceWindow` holding a devctl size.
- Everything read for logic, not compiled.

# Audit 6: Settings and Theme Studio (ST01 ST02 ST03 ST09 STX1 STX2 STX3, SD01 SD02 SD03)

Tree: `.claude/worktrees/swift-audit6` at `9db1b1da6`, wave 8 = `937014da2..HEAD`. Read only; nothing built or run.

```
id,written
ST01,100
ST02,100
ST03,100
ST09,100
STX1,100
STX2,100
STX3,100
SD01,95
SD02,95
SD03,100
```

## Rows below 100

### SD01 (95)
- GPUI's Studio reads its contrast from the store's tokens, which follow Increase Contrast the moment it changes (`studio/mod.rs:584` `tokens_of`, `settings/derive.rs:51-67` `contrasted`). → Swift: Increase Contrast is read only when the draft is resolved. `StudioDocument.refreshContrast` runs from `init` and `apply` only (`StudioModel/Document/StudioDocument.swift:57-90`), and nothing re-resolves the open document when `ThemeStore.increaseContrast` changes: its `didSet` refreshes the store, not the document (`DesignSystem/Theme/ThemeStore.swift:24`), and `StudioView` watches only `shownPick` (`StudioUI/Window/StudioView.swift:64`). So if Increase Contrast is turned on or off while the Studio is open, the row marks, the Low count, the Contrast section and the card stay on the old variant until the next edit. Low importance, and older than wave 8 (the document's `tokens` had the same lag), but the lane's "they follow the high-contrast variant" holds only when the setting was already on at the last edit.

### SD02 (95)
- Status tiles are held part-way through every variant, a third of the way from working to needs input (`studio/gallery/tiles.rs:123, 200-240` `status_changes`, `rolling_counts`). → Swift: flare and nudge are still held at a third (`TileStatesSample.swift caught`). For cross-fade, badge pop, wash and line roll there is now a "before and after" row (`SidebarUI/Samples/TileStatesSample.swift:41-62` `beforeAfter`), but its four pairs are identical: the same `StatusTile(.working)` → `StatusTile(.needsYou)` at `phase: 0`, with only the caption changing. None of them shows what that variant draws partway (the wash's tint, the badge mid-pop). The counts' before/after (`StudioUI/Gallery/TilesSpecimen.swift:40-56`) is an acceptable stand-in: the same tokens, at both ends.

## Audit 5's gaps found CLOSED
- ST01: 4 of 4 (wave 8 added only the InfoButton hover face and VoiceOver label; nothing removed).
- ST02: 13 of 13 (wave 8: `PartsList` hover fill, `PreviewStrip` Increase Contrast edge; additive).
- ST03: 3 of 3 counted as closed. (1) The permission is read again on events: `NoticeFeed.lookOnFocus` re-reads on `didBecomeActive` (back from System Settings) and when the Settings window becomes key (`Wormhole/Notices/NoticeFeed.swift:87-99`), and the page's own `.task` now calls `rereadPermission` (`AppSettings.swift:21` → `NoticeFeed+Test.swift:37-43` → `SystemNotifier.readPermission` → `center.notificationSettings()`). No timer; the only sleep is the old one-shot while `.checking`. (2) Send test reads the permission fresh before deciding (`NoticeFeed+Test.swift:53-56`), so notifications turned off in System Settings give `.denied` in `finishTest`, never "Sent". (3) A symbolic link as the final `settings.json` of a login folder: still written through. GPUI does the same through core's installer (`LinkPolicy::Follow`), and the confinement `resolved_within` is the Swift app's own extra hardening, so this is not a gap against GPUI. It is still an open question for the owner (TO-VERIFY 1240), and `is_real_home` now catches `<home>/../<name>` and a link to the home (`ffi settings/agents.rs:137-141`).
- ST09: 1 of 1. STX1: 1 of 1. STX3: 2 of 2 (untouched).
- STX2: 2 of 2. Trust is now followed as an event: core's watch on the profile's `trusted-projects.json` (`ffi settings/project_watch.rs watch_approvals`, `files_panel/project/settings_page.rs watch_trust`), reached through `RustProjectsBackend.followTrust` → `ProjectsModel.followTrust` (an AsyncStream that lives as long as the page's `.task`, `ProjectsPanel.swift:50-53`). `trusted_projects` and `project_page` call `reload_trust` before they read, so the page reads the file as it is now. The window coming forward is still the fallback.
- SD01: 3 of 3. (1) Contrast comes from one core report: `studio_contrast` gives `contrast_checks` (every pair, the same `measure` that `contrast_warnings` uses) plus `contrast_issues`, with `high_contrast()` when Increase Contrast is on (`ffi studio_edit.rs:94-136`). The row marks (`TokenListView.swift:18`), the Low filter and its count (`StudioDocument+List.swift:21-36`), the Contrast section (`ContrastSection.swift:14` → `ContrastReport.checks(of:)`, the same `fg==t||bg==t` as `contrast_checks_of`) and the card (`LowContrastCard.swift:14`) all read `document.contrast`. Pairs, thresholds and the `passes` rule are core's, so it matches GPUI (`studio/contrast.rs:24`, `token_editor.rs:109, 171`). The Swift pair table and its maths are gone, and nothing refers to them. Without the core the report is empty (no marks, no card), but the one `StudioEnvironment` in Sources is the launcher's real one (`StudioLauncher.swift:71`), so only tests run that way. (2) "Used by", a metric's "n uses" and the area-chip count come from `uses` = core's `TOKEN_USES` (`FfiTokenUses.all()`, passed in at `StudioLauncher.swift:72`; `UsedBySection.swift:14`, `MetricEditor.swift:46`, `GalleryView.swift:22`). The generated `GalleryUses` feeds only the gallery outlines (`GalleryView.swift:20` `drawn(with:)`) and the inspector note, so "Used by" never shows a person the generated table in the app. `TokenUse.draws` handles `terminal.*`/`syntax.*`, and core's only wildcard is `syntax.*`, so they match. (3) `assumeIsolated` from a background thread now hops to the main thread (`StudioLauncher.swift:66-69`).
- SD02: 3 of 4 counted as closed (lazy gallery; core's `TOKEN_USES`; tooltip and dialog specimens are the owner's decision, since macOS draws tooltips and alerts, and "Used by" still names them). Status tiles are improved but still PARTLY (above).
- SD03: 7 of 7 (untouched).

## The shipping-app change (`saved_versions::{file_id, version_id, save_id}`)
I read it as correct, and it does what the CHANGELOG line says.
- First save: a new or untitled theme gets `save_id(name, replaced)`. With nothing by that name, this is the old rule (an id made from the name that no other theme or built-in has). Duplicate gets `new_id(copy, None)` (unchanged).
- Rename and save (`write_own` → `file_to_write`, `studio/convert.rs:36-52`): for a v1 file with no `id`, `save_id(..., Some(path))` returns the file's current slug (`file_id`, read from disk, where it still has the old name). That slug is written into the file, and `keep_old`/`versions_in_profile` look up the same slug, so the earlier saves stay listed. Before, the id came from the new name and the versions were left under the old one.
- Older-format file converted by the save (also `pair_of`, `commit.rs:157`): the converted file gets the file's existing slug, a stored id included. Before, it got one made from its current name, which could drop the stored id and orphan its versions. Now it doesn't.
- Can two themes share an id: no new way. `file_id` returns the slug `assign_ids` gave, which is unique in the folder at that moment (stored ids first, then derived ones with `_n`). Save-as over a same-named theme writes to that same file (`save_theme_in` → `named_in`), so taking its id cannot duplicate one. Only the old ways remain: a Finder copy of a file, or files added later.
- Existing versions: for a single theme, `version_id` is the slug, as before. For a file with several versions it is now the slug's prefix instead of `file.id`/derived name, which differs only when the family's id clashed. A theme renamed before this release keeps its versions hidden (no migration). The CHANGELOG describes future renames, so that is fair.
- Someone who never renames: every save of a v1 theme file without an `id` now writes `"id": "<slug>"` into it. The id is pinned to the current slug, possibly a clash suffix like `mine_2`. Save-as replacing a same-named theme now keeps that file's id (before: a new one from the name), which matches its comment and keeps that file's versions together. I see no harm in either.
- Callers: GPUI `themes.rs:218-219, 256-270` and `convert.rs`. FFI `studio_files.rs studio_new_id`, `studio_folder.rs version_id`. Swift `StudioDocument+Save.swift:51-53` does the same as GPUI's `file_to_write`. Imports read consistently (`slug`/`read_theme_file` still used where kept, no module cycle: `registry` does not import `saved_versions`), but nothing was compiled by the lane or by me.

## New
- Parked agent access (`Wormhole/Settings/ParkedAccess.swift`), outside my rows (A05/A07). `followParkedAccess` runs only after a profile switch and a profile ending (`ProfileChipBinding.swift:88, 127`). On a switch back to A, A's parked follower is dropped in the same pass that `followProfileSettings` starts A's own, so a switch back never follows A twice. Two edges remain. (1) If a window of the parked profile is opened while another profile is shown, its `ProfileLook` and the parked follower both settle each change. That is harmless, because core's `settings_changed` acts once per mode. (2) If a later switch drops A's parked follower because A has a `ProfileLook`, and then that window closes (`dropUnusedProfileLooks`, `ProfileLooks.swift:61-63`, `AppController+WindowEnding.swift:64`), nothing calls `followParkedAccess`. A's parked terminals then have no follower until the next switch, so a change to its `agent_access` does not settle them. That is rare, but it is half-wired.
- Profile switch with a draft open: Swift now keeps a draft's versions next to its own file (`FfiThemeFolder.home(of:)`). GPUI's `overwrite_in_profile` calls `keep_old(&dir(), ...)`, which puts them in the new profile's themes folder. Swift is the better of the two, but the lane's "as in the GPUI Studio" is wrong for this one point. `convert` and `completions` still use the live (new) profile's folder, as GPUI does.
- `StudioLauncher` `live`: `DispatchQueue.main.sync` would deadlock if the main thread ever waits on a background caller. I found no such caller.
- Nothing that used to work is gone: the Swift contrast table (`ContrastPair.swift`, `ContrastCheck.all/lowTokens`) is deleted with no reference left, and `GalleryUses` is regenerated (the `Chip` rule puts `metrics.radius_chip` back on the chip rows) and is used only for outlines.

# Audit 6: Secondary windows, overlays, platform (A01-A07, O01-O05, PL01-PL07, PL09)

Tree read: `.claude/worktrees/swift-audit6` at `9db1b1da6`. Swift paths are under `macos/swift/Sources/`; GPUI paths under `crates/wormhole/src/`.

```
id,written
A01,100
A02,100
A03,100
A04,100
A05,100
A06,100
A07,100
O01,100
O02,100
O03,100
O04,90
O05,100
PL01,100
PL02,95
PL03,95
PL04,100
PL05,100
PL06,100
PL07,100
PL09,100
```

## Gaps still open (rows below 100)

O04 (90). Read item by item this time: terminal grid, sidebar row (mouse and ⌃↩ copies), group head, pane header and its ⋯, folder crumb, title row, Changes file and folder rows, History commit row and details block, a commit's file rows, Files rows, and the + button's shells.
- WRONG: the title row's own menu writes the setting. Each part's toggle and Reset title row call `update_settings` (`window/title/parts.rs:82,89`), so the change reaches `config.json`, every window of the profile and Settings' Title row list, and it survives a restart. Swift: `setPart` and `resetRow` change only this window's in-memory `hub.titleRow` and the core model's `cfg.title_row` (`WindowUI/Hub/WindowHub+Title.swift:28-29,37`, core `workspace/model.rs:239-242`). Nothing writes `title_row`, so other windows, Settings and the next launch all keep the old row. The ⌃↩ copy has the same fault (`PanesUI/Title/TitleKeyMenu.swift:52-56`).
- Small: in Zen mode, GPUI's title-row menu ticks the person's own row (`settings_without_zen`, `parts.rs:64`). Swift ticks the row on screen, which is Zen's preset (`hub.titleRow`, set by `HubSettings.applyTitleRow`). Both grey the items in Zen.
- Tiny: "Show beside…" lists terminals in the list's order in GPUI (`session_order`, `window/sidebar_drops.rs:111-118`). Swift lists them in `state.all` order (`Wormhole/Menus/RowMenuKeyboard.swift:29`; the SwiftUI copy uses `context.everyTerminal`).
- Tiny: the terminal menu works out Split Right / Split Down before it selects the terminal that was right-clicked (`Wormhole/AppController+Context.swift:18-21`). The greying is therefore the previously selected terminal's; it differs only when the two panes differ in room.
- Everything else matches GPUI item for item: present, enabled, checked, worded, and what choosing each item does (list below).

PL02 (95). New, not in audit 5:
- GPUI's Settings key recorder sees every key first: the page's `on_key_down` calls `record_key` and stops propagation (`settings_window/window.rs:449-453`). So any key can be recorded.
- Swift: `ExtraKeys` is a local keyDown monitor, and local monitors run before the recorder's `performKeyEquivalent`/`keyDown` (`SettingsUI/Controls/KeyCapture.swift:31`). It takes the key in Settings too, because Settings can become main (`Wormhole/Actions/ExtraKeys.swift:28-33`).
- Effect: while recording a shortcut in Settings, ⌘1-9, ⌃⌥1-9, ⌃⌥⌘0-9, ⇧⌘N, ⇧⌘], ⇧⌘[ and ⌘= are not recorded. Each runs its action on the front window instead (new terminal, select group, and so on).
- `onRecording` pauses only the global hotkey (`Wormhole/Settings/AppSettings.swift:22`), never `ExtraKeys`.

PL03 (95)
- Unchanged from audit 5: a dev build without a bundle posts through the platform's script; Swift only logs. It is dev-only and arguably a packaging matter; it was not touched.

## Audit 5's gaps found CLOSED, per row

- A01: 1 of 1. `FFIOnboardingBackend.followTrust` (`AuxBridge/OnboardingBridge.swift:105-120`) follows the desk the card is read from. It is called from init (l.71) and whenever the project changes (l.98). `TrustDesk.follow` (`crates/wormhole-ffi/src/aux_backends/trust.rs:153-177`) watches the project's `.wormhole` folder and the profile folder, filtered to `trusted-projects.json`. On each wake `makeCard` reads again (which re-arms the waker) and `cardChanged` bumps `OnboardingSession.trustRevision`, which the view reads (`AuxModel/Onboarding/OnboardingSession.swift:35,40-43`). Reached: `AuxController.showOnboarding` → `backends.onboarding` → `OnboardingWindow` → `OnboardingSession`.
- A02, A03, A04: none open; untouched.
- A05: 1 of 1. `ParkedAccess` (`Wormhole/Settings/ParkedAccess.swift`) follows each parked profile's settings through its own `SettingsBridge.make(profile:)` backend, which follows the file. On a change of `agent_access`, `control_write`, `feature_flags` or `never_hide_lines` it calls `hooks.settingsChanged(profile:)`. That leads to `hook_server/core.rs:287-315` `settings_changed`, which re-reads the seat and settles by `mode_in_force`. `followParkedAccess` runs after a profile switch and after a profile ends (`ProfileChipBinding.swift:88,127`). It excludes the shown profile and profiles that have a `ProfileLook`.
- A06: untouched apart from `TrustDesk.follow` gaining the approvals file, which also serves the trust window; read, nothing broken.
- A07: 1 of 1 (same as A05; the widening goes through `mode_in_force` in `settings_changed`). A parked profile's mode question has no shown window, so `window(ofProfile:)` is nil (`Aux/AuxController.swift:87-90`). GPUI's would sit on a hidden window. Nobody sees it in either app; not counted.
- O01, O02, O03, O05: untouched (no palette or picker files in the wave-8 diff).
- O04: audit 5 listed no gap (not re-read). Both of the lane's O04 claims are menu-bar items and are confirmed (see PL01). The context menus were read in full this time (see above).
- PL01: 1 of 1 written, plus 1 that is not a gap.
  - Show Project is greyed with project folders off: `RecentProjects.swift:62-64`, where `featuresOff` is the core's `FeatureMask` for the window's profile (`settings/flags.rs:53-56`). This is GPUI's `project_folders_on` (`features/project/def.rs:100`).
  - Open Recent's in-memory list: GPUI also reads its store once (`menu_state.rs:90`). Not a gap.
- PL02: audit 5 had none open; one new (above).
- PL03: the permission re-read is on events only (app active, Settings key, before Send test: `Notices/NoticeFeed.swift:84-99`, `NoticeFeed+Test.swift:34-57`), read and fine. The dev script stays open.
- PL04: 1 of 1. The jump key is followed now: `bindJumpKey` installs it, then calls `ambient.pushStatus()` (`Platform/JumpKey.swift:9-12`), and the snapshot reads `Keymap.shared.key("goToWaiting")` (`Ambient/AmbientController.swift:75-76`). A hotkey change calls `pushStatus` from `AmbientController+Settings.swift:15`. `.reply` not bringing a window forward is what GPUI does too, so it is not a gap.
- PL05, PL06, PL07, PL09: untouched.

## Readings asked for

O04, context menus, item by item (GPUI file → Swift file: verdict):
- **Terminal grid** (`terminal_view/context_menu.rs:91-226` → `PlatformModel/Context/TerminalMenu.swift`, `Wormhole/AppController+Context.swift`): same.
  - Items, order and icons: Open/Copy Link over a link; Copy (needs a selection), Paste (needs text); Select All; Find…; then the block's Copy Command / Copy Output / Put Command at Prompt (needs command plus prompt) and Fold Output (folds on and the block has output); Split Right/Down (greyed by room); Clear Buffer.
  - Keys come from the keymap.
- **Sidebar row**, both copies (`sidebar/row_menu.rs:155-244` → `SidebarUI/Menu/RowMenu.swift` for the mouse, `PlatformModel/Context/RowMenu.swift` plus `Menus/RowMenuKeyboard.swift` for ⌃↩): same.
  - Rename…, Add to new group, Add to group ▸ (greyed when there are no groups; current ticked; `allowed` greys folder-only groups), Remove from group, Split right, Close terminal (through `actions.closeTerminal`, which asks), Show beside… (by room, greyed when empty), Remove from split, Pin/Unpin, "Don't keep this terminal's text" (ticked, greyed without restore), Hide from Agents (offered with Automation or while hidden), Reveal in Finder, Copy path (greyed without a folder; posts Copied).
- **Group head** (`sidebar/group_popover.rs:215-242` → `PlatformModel/Context/GroupMenu.swift`, `Menus/GroupMenuKeyboard.swift`): same.
  - Rename…, the swatch row, Other colour… (ticked when custom; opens the system Colours window), No colour (ticked when none), folder line (greyed) / Set folder… / Clear folder / Only terminals in this folder (ticked), then New terminal in group, Ungroup, Close group, Pin/Unpin group, Move group to new window.
  - The switcher card uses the same menu (`PanesUI/Tabs/GroupSwitcherView.swift:29`).
- **Pane header and ⋯** (`window/panes/mod.rs:370-405` → `PanesUI/Panes/PaneMenuItems.swift`): same items, greying and focus-first.
  - Move out of split shows no key, but it has none by default (`table.rs:155`).
- **Folder crumb** (`window/title/path_menu.rs:34-60` → `PanesUI/Title/PathMenuItems.swift`): same.
- **Title row** (`window/title/parts.rs:70-100` → `PanesUI/Title/TitleRowMenu.swift`): the items and their greying (Zen, another profile's window) are the same. What choosing a part or Reset does is WRONG (above). Customize reveals the setting (`WindowHub+Title.swift:30`).
- **Changes file and folder rows, a commit's file** (`features/git/path_menu.rs:26-73` → `PanelUI/Changes/PathMenu.swift`): same.
  - Open in <editor> (left out with Finder), Reveal (greyed when not on disk), Copy Path(s), Copy Relative Path(s), File History (only with History offered); several marked files share one menu.
- **History commit row and details block** (`features/git/history/menu.rs:36-73` → `PanelUI/History/CommitMenu.swift`, `CommitDetailsView.swift:52-70`): same. The row has the five copies; the block adds Show more/less, the copies and Open Parent for each listed parent.
- **Files row** (`features/files/menus.rs:172-225` → `PanelUI/Files/FileRowMenu.swift`): same.
  - Open in ▸ (editors only for a file, every app for a folder, chosen ticked, icons, greyed for a link), Reveal, Copy Path, Rename…, Pin/Unpin Folder.
- **+ button** (`sidebar/shells.rs:22-55` → `PanesUI/Tabs/ShellsMenu.swift` via `WindowHub+Sidebar.swift:88`): same heading, shells, default ticked. The Swift list is a snapshot from the last draw rather than read when the menu opens.

Menu bar, ten actions followed from `WindowUI/Menu/MenuStateMaker.swift` and `Wormhole/Menus/RecentProjects.swift:53-77` against `window/menu_state.rs` and the features' `menu` hooks:
- ToggleSidebar: off when hidden. Same.
- PinTerminal: checked. Same.
- HideFromAgents: checked, or hidden by the window profile's Automation flag. Same.
- SplitRight: disabled without room. Same.
- ZoomPane: off and disabled outside a split. Same.
- RevealInFinder / CopyPath: disabled without a folder. Same.
- ShowProject: disabled without a folder or with project folders off. Same now.
- ShowUsage: hidden with no agent on screen. Same.
- NewTerminalInWorktree: disabled without a repository; GPUI leaves it unbound without git status. Same.
- LayoutGrid: ticked and greyed by room. Same.
- QuitAndEndAll: follows the shown profile's persistent-terminals flag. Same.
- ToggleStatusBar / ToggleMinimap / Zen: Swift reads the window's own look; GPUI reads the shown profile's settings. They differ only for another profile's odd window.

The lane's two fixes, confirmed:
- (1) Edit's Previous/Next Prompt, Select Output, Copy Output, Copy Command and Rerun Command are greyed while `hasBlocks` is false (`MenuStateMaker.swift:99-100,110`).
  - `hasBlocks` is set from the FFI's `blocks_available` (`pty.has_blocks()`, `wormhole-ffi/src/blocks.rs:92-94`), the same fact GPUI binds on (`terminal_view/render/mod.rs:147,227`).
  - It is read after each frame (`WindowUI/Overlay/TerminalOverlay+Folds.swift:16-18`) on an `@Observable` overlay that the menu refresh tracks.
  - Left as is: GPUI also greys them while the focus is in a panel list (the action is bound on the terminal view); Swift goes by the selected terminal.
- (2) `state.persistent` is set from `settingsModel.host.flags()`, the shown profile (`RecentProjects.swift:67`), as GPUI's `persistent_terminals_on` (`window/windows.rs:401-403`). A `feature_flags` change refreshes the menus (`SettingsApplier.swift:79-82`).

Key monitors, one key at a time. AppKit order: local monitors (`ExtraKeys`, `ProjectKeys`, `WindowFocus`, `PaletteField` and others), then the key window's `performKeyEquivalent` (the terminal's `TerminalShortcuts`, the recorders), then the main menu, then `keyDown`.
- **Workspace window, terminal focused:**
  - A project key: `ProjectKeys` takes it first. The terminal's own `windowClaims` → `claimProjectKey` is the backup (GPUI: `claim_window_key` first, `app/handlers.rs:254-276`).
  - ⇧⌘N, ⌘1-9, ⌃⌥1-9: `ExtraKeys`.
  - A menu's first key: AppKit runs the menu item, with its enabled state.
  - ⌘C/⌘V/⌘A/⌘K/⌘⌫/⌘←/⌘→: the terminal's `performKeyEquivalent`, before the menu.
  - The project set and the keymap set are disjoint because the core refuses a project key the keymap uses.
- **Settings or the Studio:** `ExtraKeys` acts on the front workspace window; project keys are not taken; menus work. This matches GPUI, except that it beats the shortcut recorder (PL02 gap).
- **A sheet up** (question, worktree, a Settings sheet): `ExtraKeys` refuses (`sheetParent`/`attachedSheet`, `ExtraKeys.swift:28`), and `ProjectKeys` refuses (the event's window is the sheet, plus `attachedSheet` in `hasActiveDialog`). Menu key equivalents still reach AppKit's menu. Lane claim confirmed.
- **Palette or shortcut sheet open:** `ExtraKeys` still runs (as GPUI's global bindings do); `ProjectKeys` refuses (`hub.state.overlay != nil`, `Panel/ProjectActionKeys.swift:36-39`).
  - GPUI's `has_active_dialog` is gpui-component's dialog stack, which Wormhole never opens, so GPUI would run a project key over the palette. Swift is stricter, and its comment misnames the reason. Not counted.
- **A text field in a workspace window:** both monitors take their keys, as GPUI's `claim_first` and global bindings do.

Trust following:
- **First run's card:**
  - What is watched: one FSEvents watch per desk on the project's `.wormhole` folder and on the profile folder, filtered to the project's paths and `trusted-projects.json` (`trust.rs:153-177`).
  - When it ends: when the project changes (the old stream's `onFinish` calls `desk.unfollow()`), or when the backend is deinited.
  - It outlives its window. `AuxController.onboarding` keeps the `OnboardingWindow`, and with it session, backend, task and watch, after the intro closes, until Help ▸ Welcome replaces it or the app quits (`Aux/AuxController.swift:110`). Only devctl's close clears it (`AuxController+Intro.swift:68`).
  - If the profile folder does not exist when `follow` runs, approvals are never watched for that desk (`trust.rs:161`).
- **Project folders page:**
  - What is watched: `ProjectService.watch_trust` → `watch_approvals` (`settings/project_watch.rs:40-46`) watches the profile folder for `trusted-projects.json`, with a 100 ms settle thread. `trusted_projects` and `project_page` call `reload_trust` (`files_panel/project/settings_page.rs:175,211`).
  - When it ends: the token lives in `ProjectsModel.followTrust`, run from the page's `.task` (`SettingsUI/Projects/ProjectsPanel.swift:50-53`), and ends with that task; dropping `ProjectWatch` ends the watch and the thread.
  - Whether it outlives the window: the Settings window is kept, not released (`SettingsWindowController.swift:30-33`), so whether closing it ends the `.task` depends on SwiftUI's disappear for a closed hosted window. That cannot be settled from reading. At worst one filtered watch stays until the page changes or a profile switch drops the controller.

## New

- The intro's trust watch outlives the intro window (above). There is nothing a person can see, but an FSEvents watch on the profile folder and the project keeps waking a card nobody shows.
- `ExtraKeys` runs before Settings' key recorder (PL02, above). It already did before wave 8; the wave tightened the sheet guard and left this.
- The title row menu's part and Reset choices are never saved (O04, above). This predates wave 8 and no earlier audit read it.
- `ProjectActionKeys.hasActiveDialog` treats the palette, the shortcut sheet and a diff comment as dialogs, which GPUI's `has_active_dialog` does not. It is stricter than GPUI and harmless.
- Nothing wave 8 changed in these folders removed something that worked. Read: `ExtraKeys` guard, `RecentProjects`, `ParkedAccess`/`ProfileChipBinding`, `trust.rs`, `project_watch.rs`, `OnboardingBridge`, `NoticeFeed`/`NoticeFeed+Test`/`SystemNotifier`, `AmbientController`, `JumpKey`, `HookHost` (worktree question for a gone window), `StudioLauncher`.

```
id,written
X01,85
X02,95
X03,95
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
```

Method: read only, tree `swift-audit6` at `9db1b1da6`, wave 8 = `937014da2..HEAD`. Nothing built or run. Swift paths are under
`macos/swift/Sources/`, GPUI paths under `crates/wormhole/src/`.

## Gaps still open, by row

### X01 (shared components): 85 (audit 5: 80)
- Text fields: GPUI's flat field has a well, hover, ring, prefix and clear (`style/text_field.rs:72`), plus `multiline` (`:88`) → Swift: `FlatField` still appears only in the gallery (`StudioUI/Gallery/InputsSpecimen.swift`). The app's fields are the system's or each module's own, and there is no multiline field (OPEN; the lane says it waits on the owner's decision).
- Keyboard rings: GPUI has one `focus_ring` (`style/controls.rs`) → Swift: seven rings are still drawn by hand (settings, studio, onboarding card, find field, sidebar `ringWhileMenuOpen`) and are not on `keyboardFocusRing` (`DesignSystem/FocusRing.swift:10`). The lane says so (OPEN).
- Group head's +: in GPUI it is its own button, named by its tooltip (`style/groups.rs:224, 255`) → Swift: the head is `.accessibilityElement(children: .ignore)` with three actions (fold, Edit group, Rename) and no "New terminal in …", so VoiceOver cannot reach the + (`SidebarUI/Group/GroupHeadView.swift:66-72`). NEW: the table's "same reach" is wrong.
- Pane header buttons: GPUI uses `icon_button(...).in_row()` and `hit_area`, with 28 pt targets and `band.hover` (`window/panes/render.rs:263-300`) → Swift: `PaneButton` (`PanesUI/Panes/PaneButton.swift:17-27`, `PaneMetrics.button` = 22) gives 22 pt targets, and the "…" menu has no hover wash (`PanesUI/Panes/PaneHeader.swift:71-78`). NEW.
- Form segments: GPUI's are one Tab stop, walked with the arrows, roles RadioGroup and RadioButton (`style/controls.rs:279`) → Swift: the shared `Segmented` (`DesignSystem/Segmented.swift:36,70`) still gives each segment its own stop, with no arrows and only `.contain`. Settings uses the system's picker (owner's choice). PARTLY: the hover text is now written (`:57`).
- `IconButton` `narrow` + `menu_chevron` (`style/buttons.rs:186-198`): the shared button does not have them. Open in…'s chevron is a `Menu` of its own (`PanesUI/Title/OpenInButtons.swift:24-42`): it turns while open, but it has no `band.hover` and does not brighten under the pointer. PARTLY.
- Popover rows have a hover fill (`style/popover.rs:48`) → Swift: the group switcher's hit rows and its New group / Show all groups buttons are `.plain` buttons with no hover (`PanesUI/Tabs/GroupSwitcherView.swift:62-86`). NEW.
- Profile avatar: when the window's colour falls below 4.5:1 on the profile colour, the letter switches to black or white (`style/profile.rs:17-23`) → Swift: the letter is always `surfaceWindow` (`PanesUI/Tabs/ProfileChip.swift:85-91`), even though its comment describes the rule. NEW.
- Status bar items: 22 pt drawn, clickable across the full 28 pt height (`style/status.rs:219-234`) → Swift: `StatusItemStyle` frames and hit-tests only the 22 pt face (`WindowUI/StatusBar.swift:161-170`). NEW, small.
- Palette result's `detail_color` (needs input, done, failed: `style/palette.rs:123`) → Swift: the detail is always muted (`PaletteUI/ResultRow.swift:35`). OPEN.
- A draggable note shows a faint grip under the pointer (`style/section.rs:135-146`) → Swift: notes rows have none (`PanelUI/Notes/NoteRowView.swift`). Only `PartsList` has a grip. NEW, small.
- Status glyph: 9 kinds (Running is a muted spinner, Shell is a faint terminal glyph, Terminal unread, Bell: `style/status.rs:46-57`) → Swift: `StatusDot` has 5. In the palette and the quick panel, a plain command running shows the agent's working colour and a bare shell shows the idle ring (`PlatformUI/QuickPanel/QuickPanelRowView.swift:118-128`, `PaletteUI/ResultRow.swift:96`). NEW, small.
- `row_keys.rs`: ⌃↩ on a control opens the row's menu, and `own_menu` lets a field keep its own menu → Swift: only the system context menu (`SettingsUI/Rows/SettingRow.swift:70-78`). OPEN.
- Tint and neutral chip colours (`style/labels.rs:141-177`) → Swift: `textMuted` / `controlBackground` (`FontPickerUI/TintChip.swift:13`, `SettingsUI/Controls/RowAccessories.swift:57`). OPEN, but this is a look only, so it is not counted.

### X03 (VoiceOver): 95 (audit 5: 90)
- Panel task box: GPUI gives it the checkbox role with a checked state (`style/checkbox.rs`) → Swift: a button that says "done / not done" as its value (`PanelUI/Notes/TaskRowView.swift:28-31`). OPEN; the lane did not do it because it would leave a catalog key unused.
- Open-in-editor on file rows: GPUI names the button with the file (`IconButton::spoken`, core `spoken::open_file`) → Swift: every row's button says only the tooltip ("Open in …") (`PanelUI/Changes/FileRowView.swift:48`). `BandButton` takes a `spoken` name, but no caller passes one. OPEN.
- Group head's + is unreachable by VoiceOver (see X01). NEW.
- The right panel's tab strip is now presented to VoiceOver as a segmented `Picker` (`WindowUI/PanelTabBar.swift:45-53`). It was buttons with a selected trait and a "2 of 6" value. Reading the code, the role is closer to GPUI's tabs, but whether the per-segment `.accessibilityValue` survives inside a Picker's representation is not verified.

### X04 (keyboard): 95. Wave 8 did not touch it (FindToggle is still a focusable button); audit 5's number stands.

### X05 (Reduce Motion, Transparency, Contrast): 90 (unchanged)
- In GPUI a disclosure swaps at once under Reduce Motion (`style/controls.rs:170`). In Swift, the right panel's own `Disclosure` still turns: `PanelMotion.turn` is `fade` under Reduce Motion, which is a shorter curve, not no rotation (`PanelUI/Shell/Disclosure.swift:15-16`, `PanelUI/Shell/PanelMotion.swift:16-19,25`). That covers every panel card head and every file-tree folder. Audit 5 and the wave 7 table both called this OK; it is not. Also still turning: the commit row's chevron (`PanelUI/History/CommitRowView.swift:22`, via PanelRow's `select`), the Studio band (`StudioUI/List/BandView.swift:26-29`), import cards (`SettingsUI/Panels/ImportCards.swift:60-61`), the agent list (`SettingsUI/Panels/AgentList.swift:125,156`) and the hosts card (`AuxWindows/Hosts/HostCardView.swift:53,60`). Only `motionTurn` swaps at once, and only `HeadChevron` and `SectionLabel` use it.
- Scroll-to animations are not cut under Reduce Motion: `SettingsUI/Pages/PageScaffold.swift:44`, `StudioUI/List/TokenListView.swift:35,54` (OPEN, unchanged).

### X10 (performance guards): 85. Wave 8 did not touch it; `macos/tools/baselines/swift-perf.json` still holds only nulls, and the two missing guards are still missing.

### X12 (dev tooling): 90
- Everything audit 5 listed is still open: `front`, `paints`, `refresh`, `scaling`, `settle`, `reveal-test`, `grow-hold`, `scroller`, `notice-material`, `settings-preview` (none found in the control dispatch); `pointer` still takes ROW COL; `hosts` and `notices` keep their other meanings; the control folder is compiled into every build; `a11y` is only partly done.
- Wave 8 added the studio `state` line (wired: `Wormhole/AppController+Control.swift` appends `studioStateLine`, written in `Driving/AppController+ControlStudioState.swift:13`), `studio-frames tick|scroll` in GPUI's words, and `studio-scroll` to an offset. `modal=versions` is not done (the lane says so).

### X02, X06, X07, X08, X09, X13, X14, X15
Wave 8 did not touch these, or touched them only beside the row. X07: the new words go through `Words.text` (`settings-more-information` is in `crates/wormhole-text/locales/en/settings.ftl:12`), and the only new literals are gallery samples. Numbers copied from written.csv.

## Audit 5's gaps closed, per row
- X01: 7 of 12 CLOSED (scroll edge; spoken names; find bar hover and toggles; one count badge with chip corners; branch chip width; Row min height, selected text and no hover on the selection; Reduce Motion for the group and section chevrons and for Row/Segmented sliding). 2 PARTLY (IconButton extras: dot, ring, spoken, faces and drawing are done, narrow and menu_chevron are not; Segmented). 3 OPEN (text field, hand-drawn rings, tint and neutral chip colours, which are a look). Of the side items: the (i) is CLOSED (hover, open colour, VoiceOver "More information": `SettingsUI/Controls/RowAccessories.swift:23-48`); the preview's Increase Contrast edge is CLOSED (`SettingsUI/Preview/PreviewStrip.swift:28-29`); `IconButton.selected` now reaches screens (diff wrap `PanelUI/Diff/DiffHeader.swift:53`, find toggles); Chip hover is NOT REAL (GPUI's hovering pill shows only in its gallery); the band's drop target, held-⌘ dim and folder pill are NOT REAL (I checked every GPUI caller: all pass `drop_target: false` and use the default `FolderLook`); palette `detail_color` and row_keys ⌃↩ are OPEN.
- X03: 3 of 4 CLOSED (key glyphs out of button names; the (i) reachable; ChangedPage's "Copied" posted once, `SettingsUI/Pages/ChangedPage.swift:33-42`). 1 OPEN (task checkbox role). "Nothing was run" is not a gap under the brief.
- X04: 1 of 1 (audit 5's), with nothing new.
- X05: 3 of 5 CLOSED (group and section chevrons; Row/Segmented slide; preview Increase Contrast). 2 OPEN (the other chevrons, including the panel's `Disclosure` that audit 5 thought right; scroll-to).
- X10: 0 of 2.
- X12: 0 of the 7 items audit 5 left open (the studio additions answer other lines).

## X01 in detail (the four questions)

### 1. Audit 5's X01 gaps
| gap | verdict | where |
|---|---|---|
| scrolled-list hairline | CLOSED | `DesignSystem/ScrollEdge.swift:13-28` (`onScrollGeometryChange`, 0.5 pt as GPUI's `is_scrolled`, `border` × `borderWidth`); used at `SidebarUI/SidebarView.swift:115` and `PanelUI/Shell/PanelCard.swift:36` (`PanelScroll`), which Changes, History, Notes, Project, Usage and Files use. Same set as GPUI's `scroll_edges` users (`sidebar/list.rs:197`, `features/*/view.rs`) |
| VoiceOver name without the key | CLOSED | `DesignSystem/SpokenName.swift:10-26`, `IconButton.swift:99`; reaches the title row's splits, open-in, sidebar and panel toggles (`WindowUI/TitleButton.swift:24-29`), the find bar (`TerminalUI/Find/FindBar.swift:36`), and band and row buttons (`PanelUI/Shell/BandButton.swift`, e.g. NotesCard's "New note (⌘…)") |
| find bar hover and toggle colours | CLOSED | `FindBar.swift:35-37` (`inTitleRow`), `FindToggle.swift:12-16` (`inRow().toggled`); colours `IconButton.swift:85,161` = GPUI `buttons.rs:379-389` |
| IconButton extras | PARTLY | dot `IconButton.swift:69,165,175-186` (in use: panel toggle `WindowControls.swift:32-35`); ring `:72,166` (TitleButton stops); spoken `:76` (TitleButton; `BandButton` has the parameter, nobody passes it); faces `:57-63,132-144`; drawing in the button's colour `:35-41` (PanelGlyph strips, AppIcon). Not there: `narrow`, `menu_chevron`, `still`, `flush_*` (callers pad) |
| text field | OPEN | gallery only |
| one count badge, chip corners | CLOSED | `DesignSystem/CountBadge.swift:41` (`radiusChip`); `BandCount` deleted; `CardBand.swift:26`, `SubmoduleCard.swift:51` and `FoldedSummary.swift:23` use it; TerminalTabView's ⌘N is now the shared `KeyCap` |
| branch chip width | CLOSED | `Chip.swift:42` frames width with padding inside, so the fill spans it; `BranchCrumb.swift:47-56` passes it |
| tint and neutral chip colours | OPEN (a look) | unchanged |
| Row min height, selected text, no hover over the selection | CLOSED | `Row.swift:28,47,51` |
| Segmented keyboard and role | PARTLY | hover text `Segmented.swift:57`; still one stop per segment |
| seven hand-drawn rings | OPEN | lane says so |
| Reduce Motion: chevrons and selection slide | CLOSED for the named ones | `Motion.swift` `motionTurn`/`slidingSelection`; `HeadControls.swift:17`, `SectionLabel.swift:23`, `Row.swift:53`, `Segmented.swift:63`. Other chevrons still turn: see X05 |

### 2. Twenty lines of the 81-line table that audit 5 did not compare (GPUI file read in full, Swift counterpart read)
| GPUI file | the table's line | my verdict |
|---|---|---|
| status.rs | present | INCOMPLETE: items are clickable only on the 22 pt face (GPUI 28); `StatusDot` has 5 kinds to GPUI's 9 (Running, Shell, unread and Bell are lost in the palette and quick panel). Attention hover and chip-cornered ring are right (`StatusBar.swift:74,77`) |
| rolling_number.rs | present | right (`StatusBar.swift:195-198`, 160 ms, cut without Motion or under Reduce Motion) |
| menu_layer.rs | AppKit menus, left on purpose | right (GPUI's layer is the fallback for no native menu; macOS uses native menus in both) |
| popover_layer.rs | AppKit popovers | right |
| popover.rs | rows are each popover's own | INCOMPLETE: the group switcher's rows and footer buttons have no hover fill (`GroupSwitcherView.swift:62-86`; GPUI `popover.rs:48`) |
| panel_list.rs | scroll edge written | right; scrollbar follows the system's style in both |
| panel.rs | present | right |
| path_crumb.rs | hover written | right (`FolderCrumb.swift:20,41-44`) |
| extras.rs | nothing missing | right (`ExtraChip` on `Chip`, `GaugeChip`) |
| profile.rs | present | INCOMPLETE: avatar letter contrast fallback missing (`ProfileChip.swift:85-91`); choice_row is the system picker (owner's choice) |
| bands.rs | drop target, ⌘ dim, pill NOT A GAP | right: I checked every caller (`studio/token_list.rs:260`, `features/*`: `drop_target: false`, default `FolderLook`) |
| groups.rs | "same reach" for VoiceOver | WRONG: the + is not reachable by VoiceOver (see X01). The + face is now GPUI's `in_row` (`HeadControls.swift:35-38`) |
| panes.rs | present | INCOMPLETE: header buttons are 22 pt targets, and the menu button has no hover (see X01); divider, outline, dim and drop zones are present |
| palette.rs | detail_color missing | right; the rest (surface, search row, footer, label count, marked text, selected/hover) is present |
| messages.rs | present | right (banner, callout, readout `WindowUI/Columns/ResizeReadout.swift`, block readout, key hint) |
| size_readout.rs | present | right (`TerminalUI/Scaling/SizeReadoutChip.swift`) |
| rows/files.rs | open-in-editor spoken left | right (`FileRowView.swift`: kind colours, renamed, moved, semibold selected) |
| rows/commits.rs | present | right (`CommitRowView.swift`); `TwoLineRow` detail is only warning or muted where GPUI takes any colour (minor). Chevron turns under Reduce Motion (X05) |
| section.rs | present but the lift | INCOMPLETE: the hover grip on a draggable note is missing |
| splits.rs | present | right (`SidebarUI/Row/RowMarks.swift:40-42`, tooltip from the catalog) |

14 of the 20 lines are right, 5 are incomplete, and 1 is wrong (groups.rs).

### 3. What the move onto one `IconButton` lost (diff `937014da2..HEAD`, caller by caller)
| caller | before | now | lost? |
|---|---|---|---|
| `PanesUI/Title/TitleIconButton.swift` (deleted) used by SplitButtons and OpenInButtons | 24 pt face on 28 pt hit, `bandHover` on `radiusSm`, disabled = `.disabled` + 40 % opacity, `.help` = VoiceOver label with the key | `inTitleRow()`, `bandHover` on `radius` (GPUI's), disabled = faint, no hover or press (GPUI's), motion kept (`answering(.newPane)`), name without the key plus the key as hint | nothing; the VoiceOver name improved |
| `WindowUI/TitleButton.swift` | own face, `colours.text/muted` (the same tokens), WindowUI `focusRing` round the face, own `MarkDot`, separate label and help | shared button with `.dot(on: terminal)`, `.focusRing` (hidden in a window behind), `.spoken(label)`, hint from help | nothing. The dot is now placed as GPUI's (3 pt in; before it sat in the corner). A press now sinks and fills `stateActive`; GPUI has no press look (a look) |
| `PanelUI/Shell/BandButton.swift` | 24 or 22 face, `bandHover` on `radiusSm`, motion of its symbol | `inBand`/`inRow`, `answering(IconMotion.of(symbol:))`, new `selected`, `spoken` | nothing. Gained: wrap toggled in the selected tint (was 60 % opacity) |
| `SidebarUI/Group/HeadControls.swift` HeadAddButton | 16 pt glyph on a 24 face, `stateHover`, no 28 hit, VoiceOver label | 14 on 22 on a 28 hit (GPUI's `in_row`), `bandHover`, `.quarter` kept | nothing (VoiceOver could not reach it before either: see X01) |
| `TerminalUI/Find/FindBar.swift` steps | plain button, muted always, no hover | `inTitleRow()` with hover and spoken name | nothing |
| `TerminalUI/Find/FindToggle.swift` | a real `Toggle` (`.toggleStyle(.button)`), on text `textPrimary`, hover `stateHover`, change animated with `Motion.press` | `inRow().toggled(on)`: a Button with `.isToggle` and `.isSelected` traits, on text `sidebarRowSelectedText` on `stateSelected` (GPUI's), hover `bandHover` | the AppKit toggle role and the on/off value: VoiceOver now hears a toggle button that is "selected" rather than a toggle that is "on". Equivalent words, not verified on a Mac. The press animation is gone (GPUI has none) |
| Settings and Studio callers (NoAgentsTip, ShortcutRecorder, HeaderAction, AgentList, ProfileControl, UndoButtons, TitleBar, TokenListHeader), BlockReadout, GroupSwitcher card +, TabBarView + | `label: LocalizedStringKey(...)` | `label: String` | nothing: the hover is `bandHover` (was `stateHover`) and the radius is `metrics.radius` (was `Radius.row`). Sizes stay 28 (`Metric.control` = `hitMin` = 28). Tooltips and disabled looks are unchanged |
Not moved (still their own drawing): `PaneButton` (pane header, tab close), `StashPromptButton`, the Open in… chevron, PanelTabBar's icon tabs. Each keeps its own label separate from its tooltip, so none of them speaks a key.

### 4. `SpokenName` against GPUI's `spoken_name` (`style/buttons.rs:409-416`)
- The rule matches: a tooltip ending in `)` is split at its last ` (`; the part inside counts as a key when it holds ⌘⌥⇧⌃ or starts with F (`SpokenName.swift:11-14`). GPUI strips the `)` first and then `rsplit_once(" (")`, which gives the same split.
- GPUI's test has five cases (`buttons.rs:571-579`): Settings (⌘,), Hide sidebar (⌃⌘S), the waiting-panel sentence, Refresh, Compare (main). All five are in `Tests/UITests/SpokenNameTests.swift:12-18` with the same expectations, plus Next match (F3) and Close find (esc).
- Swift goes further than GPUI: the key goes to the hint in words (`KeyCapSpeech.words`, "Shift Command G"). GPUI drops it.
- Every key-bearing label in Swift goes through `tooltipName`: `title-row-split-right/down`, `window-new-terminal-help`, `terminal-find-previous/next`, the sidebar and panel toggle helps, and the `keyed()` band labels. `PaneButton`, `StashPromptButton` and `PanelTabBar` keep a separate label. I found no label with a key glyph that still reaches VoiceOver whole. The onboarding's "Replay intro (R)" is spoken whole, which is also GPUI's rule (no modifier).
- The end-to-end check of the names is `withKnownIssue` (`Tests/EndToEndTests/SpokenNameTests.swift:17`): SwiftUI vends no nodes to the in-process `a11y`, so no scenario proves the names; only the pure rule is tested.

## New (wave 8 broke or left half-wired)
- `BandButton.spoken` (`PanelUI/Shell/BandButton.swift:14,24`) is a seam that no caller uses.
- FindToggle lost the native toggle role (see 3). VoiceOver wording is unverified.
- The panel tab strip's VoiceOver shape changed to a Picker representation (`WindowUI/PanelTabBar.swift:45-53`). This is not a regression on paper, but it is unverified.
- Found in passing, not broken by wave 8: the panel `Disclosure` turns under Reduce Motion, which audit 5 called right; ProfileAvatar's comment claims a contrast rule its code lacks.
- Wave 8's style fixes are all reached from app screens (checked above); none is gallery-only except `Chip` hover, which is not a gap.
