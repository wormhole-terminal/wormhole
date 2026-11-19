# Audit 8: the rows audit 7 left open (2026-11-14)

Audit 7 found the Swift app 98.0 % written and left 20 rows below 100. Wave 11 (three lanes: panel, parts, tooling)
worked its lists; wave 12 added scenarios and control commands; build rounds 8 to 10 compiled, ran and fixed. Two readers
who had written none of it then read those 20 rows again, each gap to where the app reaches it, on a snapshot of
`exp/swift-s5` at `2193da74b`, and looked in the same areas for behaviour no audit had listed. Nothing was built or run
for it. Line numbers are of that snapshot. A row the readers did not take keeps audit 7's number.

**Written and reachable: 99.1 %** by the tracker's weights (audit 7: 98.0 %); 96 of 107 rows with no gap left.
Verified by a test or a driven check is a separate figure (`features.csv`, `macos/scripts/built.py`).

What is left, in points of the whole app (the row's number in brackets): X12 0.24 (95), P07 0.12 (95), P08 0.12 (95), X10 0.11 (85), S08 0.09 (95), PL03 0.07 (90), X08 0.06 (95), W12 0.03 (95), W06 0.02 (95), X15 0.01 (90), X13 0.01 (95).

A 100 here means written and reachable by reading; what only a run or a person can settle is listed apart in each
part and is not in the number. The gaps the readers list as new (a project's settings after a profile switch, Reply's
bytes, a deleted profile's saved-text key, `wh --help`, the read-outs that were copies, fonts left registered) were
handed to three code lanes the same day; their fixes are not part of what this audit read.

| Area | Share of the app | Audit 7 | Audit 8 | Rows with no gap |
|---|---|---|---|---|
| crosscut | 21.5 % | 93.7 % | 98.0 % | 10 of 15 |
| panel | 15.4 % | 97.1 % | 98.4 % | 7 of 9 |
| terminal | 13.4 % | 100.0 % | 100.0 % | 31 of 31 |
| studio | 9.6 % | 100.0 % | 100.0 % | 3 of 3 |
| window | 9.2 % | 99.5 % | 99.5 % | 10 of 12 |
| sidebar | 7.9 % | 98.9 % | 98.9 % | 9 of 10 |
| aux | 7.8 % | 100.0 % | 100.0 % | 7 of 7 |
| settings | 7.3 % | 100.0 % | 100.0 % | 7 of 7 |
| overlays | 4.3 % | 100.0 % | 100.0 % | 5 of 5 |
| platform | 3.6 % | 99.0 % | 97.9 % | 7 of 8 |

```
id,written
S08,95
W06,95
W12,95
P03,100
P06,100
P07,95
P08,95
PL03,90
```

Tree `swift-audit8` at `2193da74b`, read only, nothing built or run. Swift paths under `macos/swift/Sources/`, bridge under
`crates/wormhole-ffi/src/`, GPUI under `crates/wormhole/src/`, core under `crates/wormhole-core/src/`. Each line says how sure:
(F) read and followed to where the app reaches it, (R) read only, (S) suspected.

## Rows below 100: gaps still PARTLY / OPEN / WRONG

##### PL03 (90)
- NEW, WRONG (F for the bytes, S for what an agent's prompt does with them). GPUI's Reply (a banner's, and the quick panel's)
  trims the text, sends nothing when it is empty, pastes it through the terminal's paste encoding (bracketed where the program
  asked, ESC and C1 removed) and presses Return 60 ms later, "the program reads the paste first" (notifications/respond.rs:61-65,
  82-85; window/control/terminals.rs:231-239; window/feature_terminals.rs:19-33; terminal_view/mod.rs:558-562;
  wormhole-term input/mod.rs:14-21) → Swift: writes the raw text and `\r` in one write, no paste encoding, no pause, no trim,
  no empty check (Wormhole/Notices/NoticeFeed.swift:155-156; Wormhole/Ambient/AppController+Ambient.swift:21; the bridge hands
  the text on as it came, notify/mod.rs:128-131). A coding agent's prompt that takes a burst of characters as a paste gets the
  Return as a line break inside it, so the reply is typed and not sent; a reply of spaces alone presses Return in a waiting
  prompt (its default answer). The app has the right tools and uses them elsewhere (`source.paste` then Return after 120 ms,
  Wormhole/Panel/PanelEffects.swift:31-38). The end-to-end proof was run against `cat`, which shows neither.
- NEW, PARTLY (R: the write is read; that a terminal runs with no view was not traced to the core's start). GPUI types the reply
  into the terminal wherever it is (`pane_terminal`, control/terminals.rs:233) → Swift: `terminals.existing(id)` is nil for a
  terminal not shown since launch (views are made on first show, WindowUI/TerminalViews.swift:1-3, 48), and the reply is dropped
  without a word (same two lines as above).
- OPEN, unchanged (dev builds only). Without a bundle GPUI posts through `osascript` (platform/macos/notifications.rs:55-58,
  174-187) → Swift only logs (PlatformModel/Notify/NoticeRoute.swift:15-19), while Settings still says "cargo run: osascript,
  no actions or grouping" (SettingsUI/Panels/NotifyControl.swift:34).

##### P07 (95)
- NEW, WRONG (F). GPUI lays the project of the terminal in front over the settings of the profile Wormhole is in now, every
  tick, whichever profile that is (features/project/detect.rs:86-104; `SettingsStore::swap_in` on a switch,
  app_state/profiles.rs:285) → Swift: `ProjectSettingsFollower` is made once at launch and keeps that moment's Settings host
  and that moment's front window's project service (Wormhole/Settings/ProjectSettingsFollower.swift:26-31;
  Wormhole/AppController+Open.swift:60-62, the only place it is made). A profile switch makes a new Settings model and host
  (Wormhole/Settings/ProfileChipBinding.swift:222-229; SettingsBridge/SettingsBridge.swift:23-35), so from the first switch on
  `host.follow(project:)` (:61, :68) writes into a handle nobody reads: a trusted project's `config.json` settings are no
  longer applied, and no row says "Set by …", until the app is started again. Its theme still arrives (the shared
  `ThemeStore`), judged by the launch window's profile's switches, not the shown one's.
- Left, shared with GPUI (core): `project::features::root_key` keys approvals by the root's lossy text.

##### W12 (95)
- NEW, WRONG (F; when it shows depends on the next change in the window watched before). GPUI: only a window of the shown
  profile puts a project's settings and theme on, "a project another profile trusts does not change the look and behaviour of
  this one" (features/project/detect.rs:83-89) → Swift: the follower reads `app.hub`, the window in front, which may be a
  window of another profile opened beside (Wormhole/AppController.swift:141-151; ProjectSettingsFollower.swift:55), and lays
  that terminal's project over the shown profile's Settings and over the shared theme: the shown profile's windows take the
  font size and colours of a project that a window of another profile is in.

##### W06 (95)
- NEW, OPEN (F). Deleting a profile throws away the key its saved terminal text is sealed with, so "any copy of the old file
  left anywhere (a snapshot, a backup) can't be opened any more" (app_state/profiles.rs:138; core
  storage/store/scrollback.rs:71-88; saved_text/platform.rs:21-23) → Swift: `settings_delete_profile` removes the list entry
  and the folder only (settings/profiles.rs:76-80, 104-109; core config/profile/list.rs:254-275, 335-348); nothing in the
  bridge or the app calls `forget_text_key_for` (Wormhole/Settings/ProfileChipBinding.swift:124-142;
  SettingsBridge/RustSettingsHost+Lists.swift:27-31). The key stays in the Keychain and a backup of that profile's saved text
  stays readable.

##### S08 (95)
- OPEN, unchanged, not worked. GPUI plays a group card's row in by the person's `motion.insert` pose
  (sidebar/motion/insert.rs:60-88) → Swift: the code is there and reached (`.transition(.listRow(…))` in the keyed `ForEach`,
  SidebarUI/Group/GroupRows.swift:45, under the list's `.motionTurn`, SidebarUI/SidebarView.swift:110); whether SwiftUI plays a
  transition set inside the `if moving … else` branch cannot be told by reading. Waves 11 and 12 added no run that looks.

##### P08 (95)
- WRONG, unchanged, the lane left it as the owner's call. GPUI runs a project key again on every repeat of a held key
  (app/handlers.rs:254-276, no held check; terminal_view/input.rs:43-49) → Swift: the repeats are taken and run nothing
  (Wormhole/Panel/ProjectActionKeys.swift:38, 72). If one run per press is accepted, P08 is 100.

## Audit 7's gaps found CLOSED

- S08: 0 of 1.
- W06: 1 of 1, as NOT REAL, and the reason holds: GPUI's Import page writes into the profile Wormhole is in, not "the
  window's" (`active_profile::paths()` and `update_settings`, settings/import.rs:51-56; active_profile.rs:11-17). Swift's page
  writes through the Settings handle of the profile Wormhole is in, made again on a switch (settings/import_review.rs:232-251;
  settings/handle.rs:116-129; ProfileChipBinding.swift:222-229). The same rule. (F)
- W12: 1 of 1. The windows left are dressed in their own look first, then the settings and theme move and the app's terminal
  palette is taken at once (`followThemeNow`), and only then are the returning windows dressed, once
  (ProfileChipBinding.swift:84-97; Look/TerminalLook.swift:80-97; `ThemeSync.apply` is synchronous inside `applier.start()`,
  Settings/SettingsApplier.swift:37-61). What the applier no longer reaches at that moment (no window wears the app's look
  yet) is given by `dress` afterwards (Settings/HubSettings.swift:14-19). (F)
- P03: 1 of 1. The comment being written is `ChangesStore.commenting`, with the terminal it is for
  (PanelUI/Stores/ChangesStore.swift:48-53, 225-251); the sheet hangs off the window's root
  (PanelUI/Diff/CommentEditor.swift:12-25; WindowUI/WorkspaceRootView.swift:72); a line's click only sets it, one at a time
  (PanelUI/Diff/DiffRowView.swift:95-98; PanelUI/Changes/ChangesView.swift:55-59). A reload, a closed diff or another terminal
  leaves it up; saved, it goes to the terminal it was opened for, as GPUI's `Commenting::session`
  (features/git/diff/review.rs:35-44, 212-225). (F)
- P06: 2 of 2. (F)
  - The merge that could not be laid is kept (`unlanded`) and laid before the next merge is sent, once the terminal reads
    (Wormhole/Panel/NotesCoordinator.swift:38, 161, 195-203). Left over: a quit, or a move to another profile (:49), while the
    database still cannot be read brings the file's deleted line back once; a failed save of the laid merge does the same.
  - The read of a terminal not on show is off the main actor (`Task.detached`, :224-228); the main actor reads it only when a
    merge changed something (:191-195; `applying` returns nil otherwise, FilesBridge/Notes/NotesMapping.swift:44-45).
- P07: 1 of 1. The follower asks with `selectedPath` (the `readPath` key), the bridge opens it through `names::folder`, and the
  record's root comes back as a key (ProjectSettingsFollower.swift:55, 64-73; files_panel/project/overrides.rs:36, 50;
  project/theme.rs:19; project/service.rs:254). The worktree close question was moved to `readPath` too
  (Wormhole/AppController+WorktreeClose.swift:25-31). (F)
- P08: 0 of 1.
- PL03: 0 of 1.

## New

Broken or half-wired by waves 11 and 12 in these rows: nothing found. Read whole: `ChangesStore`, `CommentEditor`,
`DiffRowView`/`DiffReview` (the old per-row sheet is gone, nothing else used it), `NotesCoordinator`, `ProjectSettingsFollower`,
`ProfileChipBinding`, `ProfileLooks`, `TerminalLook`, `SettingsApplier`, `HubSettings`, `ProfileFlags` (the per-profile wake
relay), `WakeStream` (re-armed with every wake, one more element per 250 ms), `FileViewerBody` (a faithful move of the card's
words), `RevPicker`, `FolderColorCard`/`ColorChoice`/`RadioGroupSpeech`, `SubmoduleRowView`, `SecondLineView`, `TrustWindow`,
`AppController+WorktreeClose`, and the bridge's `project/service.rs`, `overrides.rs`, `workspace_extras.rs`, `workspace_pump.rs`.

Never listed by an audit (the first four are the lines above):
1. PL03: Reply is written raw with its Return, not pasted and then sent. (F / S)
2. P07: no project settings after the first profile switch. (F)
3. W12: a window of another profile in front lays its project over the shown profile. (F)
4. W06: a deleted profile's saved-text key stays in the Keychain. (F)
5. PL03: a reply to a terminal with no view yet is dropped. (R)
6. P06, narrow (S): the off-main read is new in wave 11 and can now fall inside "Move Window to" (`rehome` deletes the rows
   from the old database, windows/rehome.rs:126-169, before `follow(profile:)` swaps the store,
   FilesBridge/Notes/RustNotesPersistence.swift:14, 22-25): it reads an empty list, not nil. If that terminal has an old merge
   base in the profile it moves into (it lived there before), one merge sends "everything deleted" and notes.md loses those
   lines for about two seconds, until the landing's rebase and the next merge put them back (ticks made in the file meanwhile
   go). Needs a move back to a profile within the millisecond of a read; before wave 11 the read was on the main actor and
   could not interleave.
7. P04's, seen in passing (R): `gh` is allowed by the project at the repository's root (`ProjectFolder::open(root)`,
   workspace_extras.rs:160-167), GPUI by the terminal's own project (`programs_approved(session.id)`,
   window/title/extras.rs:183-190): they differ where the project folder sits above or below the git root.

Not found, with where I looked: GPUI's diff, files, notes and project folders walked against the Swift panel for a behaviour
missing altogether (features/git/diff/{mod,header,review}.rs, features/files/menus.rs, features/notes/mod.rs,
features/project/{panel,detect}.rs, sidebar/motion/{mod,insert}.rs, notifications/respond.rs, app_state/profiles.rs): the path
menu, pins, Follow Terminal, Reveal Current File, the no-newline mark, the refresh error, note folds, the Motion flag and the
banner's Open / Show Changes are all there.

```
id,written
X01,100
X02,100
X03,100
X06,100
X07,100
X08,95
X09,100
X10,85
X12,95
X13,95
X14,100
X15,90
```

Method: read only, tree `swift-audit8` at `2193da74b`, changes since audit 7 = `f578f1527..2193da74b` (106 commits, 132 files under
`macos/swift/Sources` and `crates/wormhole-ffi`). Nothing built or run. Swift paths under `macos/swift/Sources/`, GPUI under
`crates/wormhole/src/`, core under `crates/wormhole-core/src/`, bridge under `crates/wormhole-ffi/src/`. Sureness: (F) read and
followed to where the app reaches it, (R) read only, (S) suspected. A 100 here says "written and reachable by reading"; what
only a person with VoiceOver or a run can settle is listed under New and is not in the number.

## Rows below 100

### X08 (95)
- `wh --help` and `wh -h` print the control command line's usage: a leading `--help`/`-h` is a control verb (core `ipc/client.rs:266-269`, `ipc/control/cli/parse.rs:21`), and `wh` is `exec <binary> "$@"` (core `terminal/cli_tool/mod.rs:57`) → Swift: `main.swift:14` hands the command line to the core only when the first word has no dash (or is `--json`), so `--help`/`-h` fall to `main.swift:20-23` and print the Swift app's own launch options ("--shell … Needs WORMHOLE_DATA_DIR=…"). `wh help` and `wh list --help` are right. New, never listed. (F)
- `run` shows a command's output only the first time per terminal and `blocks` lists nothing in the marking-shell fixture (features.csv X08) → Swift: still untraced; TO-VERIFY wave 11 answers NOT DONE ("needs a run") and names where to look (`hook_server/control/wait.rs` `first = reader.current_prompt()`). No scenario runs `wh run` twice or `wh blocks` (the scenarios' `blocks` is the control folder's, another reader). OPEN. (R)

### X10 (85)
- No code changed (`Tests/TerminalCoreTests`, `macos/scripts`, `Driving/PerfProfile.swift`, `TerminalMetal/AllocationCounter.swift`: empty diff). Audit 7's line stands: no guard for "a screen without blinking text asks for no clock" (`terminal_view/render/guards.rs:234`) or half blocks without shaping (`:164`), and no stored Swift baseline that fails on a regression (`scripts/perf.sh`). Wave 12 added only a scenario that `perf-profile` names its spans (no time asserted). Number copied.

### X12 (95)
- The control folder exists only in a `devctl` build (`devctl/mod.rs:1`) → Swift: unchanged, every build reads `WORMHOLE_DEVCTL` (`Wormhole/Options+Parse.swift:42`) and starts it (`Wormhole/AppController+Open.swift:33-40`); safe only while `Isolation.swift` refuses other data folders. OPEN; must close before the app runs as the product. (F)
- `a11y` prints the tree VoiceOver gets (`devctl/mod.rs:450-453`) → Swift: `wakeSwiftUI` unchanged; whether SwiftUI then vends its nodes is still unknown, so every wave-11 VoiceOver fix (radio groups, the submodule action) has no read-out. PARTLY. (R)
- `pointer <x> <y>` + `mouse-*` drive a row dragged out of the list (`devctl/mod.rs:352-355`) → Swift: new `sidebar-let-go <id> <x> <y>` calls the two closures the list's drag end calls (`Wormhole/AppController+ControlWindows.swift:136-143` against `SidebarUI/Drag/SidebarView+Drag.swift:97-102`); the gesture up to its end still cannot be driven. PARTLY, small. (F)
- Read-outs that still answer for their argument, not the screen: `theme-cards [filter]` (`Driving/AppController+ControlReadouts.swift:32`; the filter is `ThemeCards`' own `@State`, `SettingsUI/Rows/ThemeCards.swift:15`), `studio-complete <text>` (`Driving/AppController+ControlStudioFamily.swift:28-35`), `studio-versions select` (does not move the sheet's selection; TO-VERIFY says so). NOT DONE, as the lane says. (F)
- New read-outs that restate a rule instead of reading it: `cursor` works out "drawn" itself (`TerminalMetal/CursorReadout.swift:15-18`, the comment names `TerminalScene`/`RowBuilder` as the rule it copies); `profile-chip` lists what the chip is given, not the chip (`Driving/AppController+ControlChromeActs.swift:104-111`; that it is drawn is `marked profile.chip`); `review-paste`/`review-clear` and `diff-layout` call the store behind a guard written a second time (`Driving/AppController+ControlPanelActs.swift:38-49`). Small. (F)

Command lists as they stand: GPUI's header (`devctl/mod.rs:16-483`) did not change since audit 7 (empty diff); Swift gained 20 command
names (`front`, `grow-hold`, `sidebar-let-go`, `group-drop`, `group-plan`, `profile-chip`, `colour-count`, `marked`, `changes-rows`,
`panel-tabs`, `submodule-enter`, `diff-drawn`, `diff-layout`, `review-paste`, `review-clear`, `comment-remove`, `task-move`,
`note-move`, `cursor`, `flags`) and lost none (every `case "…"` of audit 7's list is still there). All are reached (`Wormhole/AppController+ControlMore.swift:8-22` from
`run(line:)`). GPUI names without a Swift counterpart now: none beyond audit 7's renames (`front` and `grow-hold` are in).

Twenty read-outs, real thing or copy:
| command | reads |
|---|---|
| `trust-state` | real: the open window's model, `none` while closed (`ChromeActs.swift:81-87`, `AuxWindows/Trust/TrustWindow.swift` `shown`) |
| `comment-at` / `comment` | real: the one state the window's sheet is shown from (`PanelUI/Diff/CommentEditor.swift:17-26`, `WindowUI/WorkspaceRootView.swift:72`); `comment` refuses without `window.attachedSheet` (`AppController+ControlNotes.swift:57`) |
| `status-items` | shared rule: `StatusBar.state` and `StatusHosting.showsWord`, the functions the bar draws from (`WindowUI/StatusBar.swift`, `Driving/WindowHub+Readouts.swift`) |
| `recents projects` | shared: `recents.names(of:)`, what Open Recent is handed |
| `switcher` | real for `open` and `query` (the popover reports itself, `PanesUI/Tabs/GroupSwitcherView.swift` `switcherShown`); the cards from the shared search |
| `title-menu`, `path-menu` | shared: one entry list for the right-click's menu and the keys' (`PanesUI/Title/TitleMenuEntries.swift:20-49`, `TitleMenuItems.swift`, `TitleKeyMenu.swift:30-44`) |
| `settings-page` banners | shared: `PageBanner.of` draws and is read (`SettingsUI/Pages/PageBanner.swift:20-33`) |
| `files-viewer` | shared: `FileViewerBody.card` |
| `sidebar-tip` | shared: `hub.agentsTipStage` (`WindowUI/Hub/WindowHub+AgentsTip.swift`) |
| `changes-rows` | shared: the build and cut the list uses (`PanelUI/Driving/ChangesRowsReadout.swift:22`, `PanelUI/Changes/ChangesListView.swift:36`) |
| `panel-tabs` | shared: `badge(for:)` and `spokenTab`, what `WindowUI/RightPanel.swift:26,46` draws from |
| `diff-drawn` | real: what `DiffView` decided from its width (`PanelUI/Diff/DiffView.swift:59`) |
| `marked` | real: frames the views report (inspect mode on) |
| `colour-count` | real: the window's own picture (`Driving/AppController+ControlLooks.swift:33-36`) |
| `studio … modal=` | real: `session.showsUpdate/showsVersions`, bound to the sheet (`StudioUI/Window/StudioView.swift:58`) |
| `pull-request` | the title row's facts (`hub.state.title?.pullRequest`), then the chip's own click function |
| `cursor` | copy: rule restated |
| `theme-cards [filter]` | copy: for the argument |
| `studio-complete` | copy: for the argument |
| `profile-chip` | copy: the chip's input, not the chip |
16 read the real thing or the function the view draws from, 4 a copy.

### X13 (95)
- The fonts of the profile Wormhole is in are the ones registered: on a profile switch the fingerprint of the new profile's folder differs, so the old profile's files are unregistered and the new one's registered (`fonts.rs:75-103`, `active_profile::paths().fonts_dir()`) → Swift: a switch makes a new settings backend and with it a new `FontFolder` (`Wormhole/Settings/ProfileChipBinding.swift:222-229`, `SettingsBridge/RustSettingsBackend.swift:22-27`); the old one is dropped without unregistering (`SettingsBridge/FontFolder.swift` has no `deinit`, `unregister` runs only for a removed file). A family only in profile A's folder keeps drawing in B, and a custom JetBrains Mono of A stays registered beside the bundled one B registers. New, small. (R: the drop was read, CoreText's choice between two registered families was not run)
- Code text follows the Ligatures setting where GPUI gives it `code_font_features` (`fonts.rs:219-226`; used for a project's commands `features/project/panel.rs:516`, the project commands window `features/project/trust_window.rs:219`, a git problem `features/git/problem.rs:52`) → Swift: `DesignSystem/CodeFont.swift:7-24` carries family and size only; `font_ligatures` reaches the terminal and the Fonts page's specimens alone (`grep ligatures:`). With ligatures off those three still join `=>`. New, small. (F)

### X15 (90)
- Not touched (`legacy.rs`, `Wormhole/Platform/LegacyLaunch.swift`, `Isolation.swift`: empty diff). Written and called, never run (`LegacyLaunch.applies` is false for `Wormhole-swift*`). Number copied.

## Audit 7's gaps found CLOSED, per row
- X01: 4 of 4. Submodule button (F): a VoiceOver action "Open a terminal in …" on the row (`PanelUI/Changes/SubmoduleRowView.swift:42-57`, root and host from `PanelUI/Shell/PanelContentView.swift:27-28`). Project-set and disabled rows (F): `EntryRowView.isChanged` is false and the keys are empty while a project sets the row (`SettingsUI/Rows/EntryRowView.swift:34,52-54,92`), a disabled row's control is `.disabled` and faint with no key and no menu (`SettingsUI/Rows/SettingRow.swift:51,80,95`), `faint` is the bridge's `Missing && != Notes` (`files_panel/project/settings_page.rs:228`, as `settings_window/sections/projects.rs:512-516`). Radio role and shared ring (F for the code, a person for the hearing): `DesignSystem/RadioGroupSpeech.swift`, used at `SettingsUI/Rows/ThemeCards.swift:138-142`, `AuxWindows/Onboarding/LookStep.swift:30-33`, `PanelUI/Project/FolderColorCard.swift:41-44`; `keyboardFocusRing` + `focusEffectDisabled` on all three cards. `own_menu` (F): closed by construction, and the reason holds against GPUI: the only text fields in setting rows are the Profile page's, in rows without keys in both apps (`settings_window/sections/profile.rs:30-36` `Setting::custom`, core `config/settings_index/table/profile.rs:13-21` `&[]`); a Swift row without keys has no `.contextMenu` at all (`SettingRow.swift:54-65`), and no keyed control kind is a text field (`SettingsUI/Rows/EntryControl.swift:17-61`). Also closed: the second line rolls on a change of tone (`SidebarUI/Row/SecondLineView.swift:28,37,44`).
- X02: 1 of 1 (the standing question). One app-wide project theme is GPUI's too: `settings/mod.rs:127,463-470` holds one `project_theme` in the one `SettingsStore`. NOT REAL, holds. Wave 11's change of order at a profile switch (`Wormhole/Settings/ProfileChipBinding.swift:85-96`, `Wormhole/Look/TerminalLook.swift:72-80`) read: nothing lost.
- X03: 2 of 2 (the submodule action and the radio groups, above).
- X06: 1 of 1 (the unread caveat since audit 1). `setTheme` writes the pick through the core's rule and never moves the mode (`DesignSystem/Theme/ThemeStore.swift:109-118`, core `config/picks.rs:11`); `show`, which does move it, is called only by `--theme` and the control folder (`Wormhole/AppController+Look.swift:19,41`). (F)
- X07: no gap listed. Checked the one claim: the shortcut sheet's and palette's action labels stay English under `en-XA` in GPUI as well (core `keys/keymap/palette.rs:78-83`, `sheet.rs:24` are static labels), so `08927d59b`'s correction of the scenario holds. The wave's new words all have catalog entries (`submodule-enter`, `project-folder-color`, `settings-appearance-hide-named`, `settings-appearance-trash-named`, `review-remove`, `files-viewer-empty` found in `crates/wormhole-text/locales/en`).
- X08: 1 of 2. `send-text`, `send-keys` and `run` now forget the program's selection and tell the view, which drops the block cursor and holds the blink, as `terminal_view/mod.rs:520-523` and `control_input.rs:48-53` (`hook_server/terminals.rs:67-73`, called at `control/input.rs:61,108`, `control/wait.rs:149`; `TerminalMetal/MetalTerminalView.swift:374-377`, `WindowUI/Overlay/TerminalOverlay.swift:66`). (F) The marking-shell fault stays.
- X09: 1 of 1. The quick panel's one-second clock is GPUI's too (`status_item/quick_panel/view.rs:102`). NOT REAL, holds. No new repeating timer in the diff but the tear-off growth's 60 Hz clock (as before) and the notes sync (GPUI's too).
- X10: 0 of 2 (no code changed).
- X12: 3 of 6 CLOSED: `front` (`Wormhole/AppController+ControlWindows.swift:96-101`), `grow-hold` (`:102-108`, `Wormhole/TearOff/TearOffGrow.swift:43-99`), `studio … modal=versions` (`StudioUI/Window/StudioSession.swift:17`, `StudioView.swift:24,58`, `Driving/AppController+ControlStudioState.swift:26`); 2 PARTLY (a11y, the drag-out gesture); 1 OPEN (release builds). Of audit 7's six wrong or copied read-outs, 5 CLOSED (`comment-at`, `trust-state`, `status-items`, `recents projects`, `title-menu`; `switcher` too), `theme-cards` not.
- X13: none listed.
- X14: 3 of 3 (the "not followed" items). Version-1 files (F to the bridge call): opened in place under GPUI's own rule (`StudioModel/Document/StudioDocument+Open.swift:62` = `studio/opening.rs:131-133`), a built-in copied with its base through the core (`StudioDocument+Open.swift:44-48` → `StudioBridge/FfiStudioFiles.swift:26` → `studio_files.rs:120`), ids kept on save (`StudioDocument+Save.swift:53,77`). Icon theme's light and dark family (F): `icon_theme.rs:34-40,63-67` opens both and falls back to the other, `Wormhole/Look/CoreIconTheme.swift:25-31`, set at `Wormhole/Settings/SettingsApplier.swift:107-113`. One app-wide project theme: NOT REAL (X02). `2f76c55a2` (a theme file edited on disk) read: the wake now reaches the observers whatever moved (`SettingsBridge/RustSettingsBackend.swift:58-73`). Not read line by line: the Studio's save of a version-1 pair.
- X15: none closable without a run.

## New
Broken by the waves: nothing found. The refactors read whole (`PageBanner`, `TitleMenuEntries`/`TitleMenuItems` replacing `PathMenuItems` and `TitleRowMenu`, `SettingRow`, the status bar's `state`, `CommentSheet`) keep what was there.

Never listed, most noticeable first:
1. `wh --help` / `wh -h` print the app's launch options, not the command line's usage (X08, above). (F)
2. A profile switch leaves the left profile's custom fonts registered (X13, above). (R)
3. Code text ignores the Ligatures setting in the project's commands, the project commands window and a git problem (X13, above). (F)
4. A repository in a window nobody sees is read on every file event: GPUI reads it only when somebody works in it and owes one read when it comes back into sight (`window/repo/state.rs:104-147`, `Sight::Unseen`); the bridge has no notion of sight (`grep -i sight crates/wormhole-ffi/src`: none; `workspace_watch.rs`). Work, not something seen; not counted in X09's number. (R)
5. `WORMHOLE_LANGUAGE=en-XA` is honoured by every build (`Wormhole/Platform/PlatformSetup.swift:28-29`), where GPUI says a release build cannot show the pseudo-language (`app/language.rs:7-8`). Same kind as the control folder in release builds; not counted in X07's number. (F)

Half-wired or for a person on a Mac (not in the numbers):
- Theme cards: the shelf's radio group is an `accessibilityRepresentation`, and the cards' menus (Hide, Move to Trash) are `accessibilityActions` put on after it (`SettingsUI/Rows/ThemeCards.swift:138-142`). Whether VoiceOver lists actions added outside a representation is not settled by reading; if not, VoiceOver lost the cards' menus it had before wave 11. (S)
- `IconMenuButton`'s chevron from the keyboard, ⌃↩ through AppKit-backed controls, the status bar's hit past its face, the tab strip's "n of 6": as audit 7 left them.
- `a11y` still cannot show SwiftUI's part of the tree, so `themeCardsAreRadioButtons` asserts only "where the tree lists SwiftUI's part".
- `TearOffGrow.hold` is a static that stays set until `grow-hold off`: a scenario that forgets it leaves every later tear-off part-grown (dev tooling only).
