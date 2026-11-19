# Regrade 12: how much of the Swift app is proven (2026-11-19)

Four readers who wrote none of it read every tracker row against the end-to-end suite as it stood at `3dafdb9e5`
(316 scenarios, one full pass: 312 passed, 0 failed, 4 disabled), the unit tests and the bridge's tests. The rule was
stricter than in earlier regrades: only what a repeatable test proves today counts as proven. A recorded check from an
earlier round (a driven transcript, a visual comparison against the GPUI app in `evidence/`) is listed apart, and so
is what was taken off only because no proof exists for a terminal that runs in a session host. Nothing was built or
run for the reading. Per row: `proven.csv` (shares of the row, adding up to 100: `proven`, `manual`, `unproven_auto`;
`recorded` and `hosted_cut` are parts of `unproven_auto`).

**Proven by the suite: 60.2 %** of the app by the tracker's weights. Needs a person (a look, VoiceOver's voice, a
real drag): 16.9 %. Testable and not proven by the suite: 22.9 %, of which 7.3 points were shown once by a
recorded check and 1.3 points are proven for a terminal in the app but not for one in a session host. The coarse
grade (`works_real` in `features.csv`: 1 only when a row is proven or manual throughout and at least 80 % proven)
gives 51.5 %; it fell because of the stricter rule, not because something stopped working.

| Area | Share of the app | Proven by the suite | Recorded once | Needs a person | Not proven |
|---|---|---|---|---|---|
| crosscut | 21.5 % | 50.7 % | 0.3 % | 25.1 % | 23.8 % |
| panel | 15.4 % | 77.4 % | 8.0 % | 7.0 % | 7.7 % |
| terminal | 13.4 % | 61.0 % | 14.2 % | 0.6 % | 24.2 % |
| studio | 9.6 % | 52.2 % | 1.9 % | 35.1 % | 10.7 % |
| window | 9.2 % | 60.3 % | 15.4 % | 11.2 % | 13.1 % |
| sidebar | 7.9 % | 54.8 % | 2.7 % | 24.6 % | 17.9 % |
| aux | 7.8 % | 67.9 % | 8.1 % | 19.3 % | 4.7 % |
| settings | 7.3 % | 54.6 % | 15.1 % | 12.7 % | 17.7 % |
| overlays | 4.3 % | 67.1 % | 8.4 % | 12.2 % | 12.2 % |
| platform | 3.6 % | 59.4 % | 6.2 % | 29.6 % | 4.8 % |

Heaviest of what no test and no recorded check shows, in points of the whole app: X01 1.97, X12 1.70, SD01 0.65, S08 0.53, P04 0.46, SD02 0.37, ST02 0.35, X10 0.35, S01 0.33, T01 0.32, W01 0.28, T27 0.27.

# Regrade 12, set A: area `terminal` (T01 to T31, 15,479 weight)

Read from the snapshot `3dafdb9e5`: the scenario files in `Tests/EndToEndTests`, the Swift unit tests (`TerminalInputTests`, `TerminalCoreTests`, `TerminalUITests`), the bridge crate's tests, `e2e-status-pass25.md`, `MANUAL.md`. Per row: `regrade12-A.csv`.

Weighted proven share of the 31 rows: **61 %** (manual 0.6 %, the rest unproven_auto). Grades: 1 for T30; 0.5 for 29 rows; 0 for T26.

How the grade was cut: 1 needs proven + manual = 100 with proven >= 80, so a row with any named unproven part cannot be 1. Only behaviours MANUAL.md names count as manual (the bell, the native menu, notification banners); a real drag, the blink flip in a key window, Option sides on a real keyboard and the like are not in MANUAL.md, so they stand as unproven_auto.

Hosted terminals: almost every scenario and every unit test runs an in-app terminal. Hosted ones are shown for: wheel, selection, copy, Select All and reattach (HostedPointer), Clear Buffer reaching the host (ClearHistory), typed text, restored text, Restart, host death, detach and reattach, fallback, switching the flag (Hosting*, HostsReattach, RestartedShell, HostFallbackRestore). Not shown for a hosted terminal: mouse reporting, paste and bracketed paste, keyboard modes (kitty, modifyOtherKeys), links, images, find, blocks, folds, minimap, Probe rules, program notices (OSC 9/99/777/7501), colour answers and mode 2031, window operations, text zoom and resize reaching the PTY, shell/environment/login at spawn. Where a row's behaviour is engine or PTY facing, its proven share was cut roughly in half for that part and the rows say so in `unproven`. T20, T16 and T04 have real hosted proof. Persistent terminals is off in a release build (`FeatureFlag::released`), which is why the cut is not deeper.

## Rows raised
- T30 0 to 1 (400): savedText, textAfterCrash and Lifecycle/restore prove every named case in real shells (quit, crash after the minute's save, ⌘K at once, "Don't keep", setting off). The row names in-process terminals, so hosted does not apply. The audit note on the row is out of date.
- T27 0 to 0.5 (700), T28 0 to 0.5 (500), T29 0 to 0.5 (400), T31 0 to 0.5 (400): the audit notes ("settings ignored", "texts dropped", "FFI drops WindowOp") are out of date; ShellChoice, ShellEnvironment, TerminalPrograms, ProgramColours, WindowReports and ProgramWords/TerminalBlocks prove the in-app behaviour. They stay at 0.5 because no scenario starts the terminal in a host.

## Rows lowered (concrete reasons)
- T01 1 to 0.5 (3,700): no test draws bold/italic/dim/inverse, the underline styles, SGR 58 or strikeout; what is proven is background runs, glyphs, blocks, wide/emoji, cursor and row rebuilding, plus drawn text read by many scenarios. The visual-run table against GPUI is a recorded harness run, not in the suite; if the owner accepts it as proof, T01 and the other drawing rows come back up.
- T02 1 to 0.5: fallback is asserted by a counter, not a glyph; ZWJ, flags and Indic marks are only parsed, never shaped.
- T04 1 to 0.5: autoscroll is checked as "a timer exists" on a mock; word, line and block are not shown hosted.
- T05 1 to 0.5: only 1000 + 1006 press/release proven; 1002/1003/1005/1015/1016, wheel and motion are not.
- T06 1 to 0.5: alt-screen wheel is unproven.
- T08, T09, T10, T11, T12, T13 1 to 0.5: each had parts that only the visual harness or a mock covered (OSC 8 and file links through the app; Unicode placeholders; the minimap strip, its clicks and ticks; a real scroller drag; the find bar typed into, next/previous and options through the app, because FindModel is tested on a MockSearching; gutter, hover and readout drawn).
- T14 1 to 0.5: nothing shows bell settings reaching terminals or a BEL ringing the view; the glow is a unit test on a mock source. `foldGuards` only proves the BEL reaches the engine.
- T16 1 to 0.5: the host problem banners (not responding, unavailable, ended unexpectedly words) were seen live only; no scenario stops a host.
- T20 1 to 0.5 (1,275): proven 82 but the checkpoint after a killed host (app level), the setting that stops checkpoints, and old-host history (disabled scenario) are not shown.
- T21, T22 1 to 0.5: a real file drop and the Print panel are not driven by any scenario; hosted paste unproven.
- T23 1 to 0.5: tone, shortening and reveal are proven in the Rust frame and in Swift's painting rules separately, never drawn in the app by a scenario; the pointer to `setReveal` wiring is untested.
- T26 1 to 0: no scenario or test sends OSC 9;4; only the one-shot deadline is unit-tested.

Kept: T03, T07, T15, T17, T18, T19, T24, T25 (all 0.5).

## Scenarios that pass and prove less than their name says
- `PointerTests` (TerminalInputTests) drive a `MockInteraction`: they prove the view's wiring, not the core. Rows T04, T05, T08 cited them; the real-engine proof is `LiveTests` and the ffi tests.
- `FindTests` is the find model over a `MockSearching`.
- `CursorTests` reads `cursor`, what the scene builds, not pixels; "blinking=true" is the flag, never the blink.
- `FontChangeTests/aFallbackFamilyIsAskedForWhatTheFontLacks` asserts a rasterised-glyph counter only.
- `ProgramColours` "drawn" reads `window-look`, the window's own colour state.
- Disabled (prove nothing): `OldHostHistory/clearBufferKeptByAnOldHost` (hosted ⌘K against an old host), `UsageRecents/recentFolders`, `Files/filesRawName`, `ProfileFolderAtLaunch/unopenable...`. The first touches T15/T20.
- The hosted scenarios are `.enabled(if: hostProgram() != nil)`: they skip silently without a built host. In pass 25 they show `pass`, so they ran.

## Ten behaviours most worth a new scenario (weight x unproven_auto, heaviest first)
1. T01 (3,700 x 45): SGR styles drawn in the app (bold, italic, dim, inverse, underline styles and colour, strikeout, 256 and truecolor) with a drawn-run read-out, hosted too.
2. T25 (900 x 40): a fold marker clicked by a real pointer; fold guards (alt screen, secret read, limits); a Probe's fold label; match count in a closed fold.
3. T02 (500 x 65): ligature, ZWJ/flag/skin-tone and a fallback glyph really drawn (read the glyph runs, not a counter).
4. T27 (700 x 45): a hosted terminal's shell, login flag, environment, profile and start folder (`hosts: true` with `shell: .configured`).
5. T12 (574 x 45): the find bar typed into, Return/shift-Return stepping, case/regex/word toggles and Esc, through the app on the real engine, on an in-app and a hosted terminal.
6. T28 (500 x 50): a palette slot, foreground and cursor as drawn; image memory and `allow_*` switches; colour answers from a hosted terminal.
7. T16 (798 x 30): a host stopped with SIGSTOP (Session host not responding: Wait / End host), a host that did not answer at launch, a clipboard prompt answered Allow.
8. T20 (1,275 x 18): kill -9 a host, relaunch: the terminal restarts above its checkpoint; Restore after restart off removes the checkpoint files; the disabled old-host scenario made runnable.
9. T18 (407 x 50): `live-resize-sim` snapping, strip, gate and the chip, with a hosted terminal's PTY told the size.
10. T05 (280 x 70): mouse modes 1002/1003/1005/1015/1016, wheel and motion reported to a tracking program, in-app and hosted.

Next in line: T29 raise and full screen, T23 tone and reveal drawn, T19 a real agent, T13 gutter and hover readout.

## Added columns: `recorded` and `hosted_cut`

Weighted over the 15,479: proven 61.0 %, recorded 14.2 %, hosted_cut 9.6 %. So the earlier way of counting (strict + recorded + hosted not cut) would be 61.0 + 14.2 + 9.6 = about 85 % before overlap; the three are disjoint by construction.

- `recorded` counts only a part of `unproven_auto` that I excluded from `proven` for lack of a suite test and that a file in `macos/parity/evidence/` shows passing on the real core. I opened each file named in the row's `evidence`. Where the recording shows a behaviour the suite already proves (cursor shapes, cell selection painting, images in the visual table, the context menu lists in T15.txt), nothing is added: it was never in `unproven_auto`. Where a recording does not show passing it counts 0: the find bar (95.8 % / 84.2 %, "same: no"), ⌘-hover links (83.75 %, no valid GPUI pair), iTerm2's edge column, `r-verify/T16.txt` (it holds only an exit bar and `host new`, no banner), the print panel and the engine-lane rows T27 to T31 (no evidence files).
- `hosted_cut` is the part I took off `proven` only for a missing hosted proof (0 where I cut nothing: T11, T14, T19, T20, T22, T26, T30).
- Where both apply (T01, T02, T10, T13, T18, T21, T23, T24, T25) they are different parts of `unproven_auto`: `recorded` is the in-app behaviour a recording shows, `hosted_cut` the hosted copy of behaviours the suite proves in-app. I kept recorded + hosted_cut <= unproven_auto in every row (the file checks it). The recordings are all of in-app terminals, so none of them lifts a hosted cut.
- T26 is the clearest case: proven 25, recorded 70 (a transcript of the whole report-to-idle run), unproven 5.

# Regrade 12, rows B (panel, sidebar, overlays: 24 rows)

Read only, on `swift-regrade` at `3dafdb9e5`. Weighted proven share of these rows: **69.3 %**. Weighted `works_real` goes
from 0.615 to 0.476. Shares count a visual residue as `manual` only where the code is there and only a person can judge it;
a behaviour a script could drive and none does is `unproven_auto`.

Recorded (ninth CSV column): weighted **6.5 %**, counted only where I opened a driven transcript that shows the behaviour (named in `evidence`).
`visual-run.sh` covers the terminal grid and Settings pages only, so none of these rows has a harness comparison. Evidence PNGs for S01, S03, S04 and S08
are not in the tree, so those rows get 0. S08's transcript is prose, not data. O02's project block and O03's recents appear in no file.

## Kept (grade unchanged)
- S02 (1), S05 (1), P09 (1): every named behaviour has a passing scenario that reads the real thing.
- At 0.5 and staying: S01, S03, S07, S09, P02, P03, P04, P05, P06, P07, P08, O04, O05 (each has a named, automatable gap, see the CSV).

## Raised
- S10 0 -> 0.5. The row text in `features.csv` ("none of the four exists") is stale: the no-agents tip with Don't show again and Reset tips
  (`ChromeActs/noAgentsTip`), the + menu with the shells (`plusMenu`, `RowMarks/tabBarPlusOffersTheShells`), the lock and the fork
  (`RowMarks`) all pass. Not 1: the + menu is picked through `plus-menu`, never opened as a SwiftUI menu.

## Lowered, with the reason
- S08 1 -> 0 (weight 2050, the biggest swing). No scenario or test catches a frame mid-animation, none sets Reduce Motion and
  looks for the cut. The evidence is PNG strips and the core's pose tables (FFI tests, about 15 %). MANUAL.md has icon motion only,
  no sidebar motion step.
- S04 1 -> 0. No scenario sets Density; `SidebarTests` only writes a PNG per density and asserts nothing; the settings tests never touch the key.
- S06 1 -> 0 (weight 1). Only GPUI's own behaviour (Cmd-F from the list opens Find) is proven; the Swift search field is not.
- P01 1 -> 0.5. Ctrl-Cmd-1, -3, -4 are pressed; -2, -5, -6 are not; the panel's collapse is only seen under Zen.
- O01 1 -> 0.5. Only the Actions scope (run by keys) and the Commits scope `#` are driven; Terminals `@`, Settings `,`, Folders `~`,
  Tab stepping, recents, project group, Cmd-Return are evidence files, and the Swift grouping and recents tests run on sample lists.
- O02 1 -> 0.5. The project's own keys block of the sheet has no test.
- O03 1 -> 0.5. `UsageRecents/recentFolders` is disabled, so recents through the app (kept over a restart, emptied with the flag off) are unproven.

## Trust, notes, notes.md sync, pull-request chip (read hardest)
Counted proven because the scenario would fail if trust were wrongly kept:
- Actions: `Project/trustedProject`, `actionKey`, `ProjectKeys`, `ProjectFoldersPage/trustedListRevoke` (the shell prints e2e-42 only when it ran;
  after the revoke the count stays 1, so a kept trust adds a second line), `AmbientReplies/ambientProjectRows` (run before trust did nothing).
- Revoke by another program: `TrustOutside/trustRevokedOutside` (tab, chip, key, action by id, and gh not started again, all within 10 s).
- Pull-request chip: `PullRequest/pullRequestOnlyInATrustedProject` (call log empty untrusted; fixed arguments; scrubbed environment; chip
  gone on revoke) plus the ffi test `a_cached_pull_request_is_not_shown_once_its_project_is_revoked`.
- notes.md sync: `NotesSync/notesSync` both ways and once each; `TrustOutside/trustRevokedOutsideStopsNotesSync` (a line added and a step made
  after the revoke stay out; a kept trust would have synced within 2 s); `ProjectWindows` across two windows; ffi `project_notes.rs`.
- Environment and profile: `ShellChoice/projectTrustFollowsWindow` (the other profile's window gets no project environment).
- Worktree: `WorktreeNotes/worktreeRefusedUntrusted`. Rust: `project_trust.rs` (a change asks again, what was not shown is not approved, a link out is never read).

Counted NOT proven (a wrongly kept trust would pass):
- `Project/projectTheme` reads "the person's theme first" before the project's terminal exists; an untrusted theme applied is not caught.
- `trustedProject` reaches `status=changed` and then revokes: no check that the action is refused while changed (only the Rust test says so).
- An untrusted project's `config.json` overrides (font_size) and its first terminal's env/shell are not read in the app while untrusted.
- The Project commands window with a real project (only `trust-state none`), so audit 2's "approve re-reads the disk" is untested; `project-send` of a prompt into the terminal.
- Notes side: the merge into the shown terminal is never checked after a quit (audit 2's data-loss finding has no scenario).

## The ten behaviours most worth a new scenario (weight x unproven_auto, heaviest first)
1. P04 (about 704): the PR chip in its other states (draft, merged, closed, checks passing or pending, no PR, gh missing), `history-rev stashes`,
   keyboard walk over commits, log paging.
2. S08 (615): fold, insert and leave frames mid-animation (`after <ms> window-shot` + `colour-count`) and `reduce-motion on` cutting them.
3. O01 (600): the palette scopes `@` (windows, agent question), `,` (open row, Cmd-C copies key), `~`, Tab stepping, recents, Cmd-Return.
4. P03 (527): new-file and deleted-file notes, LFS card, too-large with Load diff, mode-change badge, "No longer changed".
5. P06 (519): the notes editor's keys typed live, the close question about discarded notes, a notes.md merge still there after a restart.
6. S01 (380): the tile drawn per state (working, needs input, done, failed) by `colour-count`, the branch/dirty line, the group wash.
7. P07 and P08 (about 287 each): P07 raw non-UTF-8 names (fixture) and the tree by real keys; P08 untrusted theme and overrides checked BEFORE approving,
   refusal while `changed`, `project-send`, the Project commands window with a project.
8. S07 (280): tear-off at a point and into another window, drag out of a group, Tear off flag off (and add a MANUAL step for the real drag).
9. P01 (225): Ctrl-Cmd-2, -5, -6 and View > Panel off drawing no panel, width kept across a restart.
10. S04 (180): set Density and read `marked sidebar.row` heights; S03 (195): the group colour drawn on its band by `colour-count`.

## Findings
- Scenarios that pass and prove little: `PlatformTests/noticesAndBanners` reads `notice 12` and `notice sticky` back from the command's own input;
  `NoticeBanner/noticeBannerWhenNoticesOff` posts its warning with `notice sticky`; `PanelState/trustStateNoProject` and `unknownCommandLine` assert a refusal word only.
- Drag scenarios (`DropLanding`) send the action the drag sends, not a drag; the gesture is manual, but `MANUAL.md` has no step for it, nor for sidebar motion.
- Three disabled scenarios sit in these rows: `Files/filesRawName`, `UsageRecents/recentFolders`, `OldHostHistory/clearBufferKeptByAnOldHost` (and `ProfileFolderAtLaunch`).
- Stale text: `features.csv` S10's notes say none of it exists; S07's older notes say "no tear-off".

# Regrade 12, checker C: rows with area window, platform, aux (27 rows)

Method: read every scenario body of the files these rows lean on, the commands they read from (`AppController+Control*.swift`),
the bridge crate's and the Swift unit tests, and the manual list. Rule applied strictly: 1 only when proven + manual = 100 and
proven is at least 80; a named behaviour with no passing scenario keeps a row at 0.5.
Weighted proven share of the 27 rows: **63 %** (weights sum 23,641).

## Recorded checks (new column `recorded`, part of `unproven_auto`)
Weighted `recorded` share: **11 %** (strict proven 63 % + recorded 11 % = 74 % by the earlier way of counting). Grades above are the strict
ones and do not change. Counted only where I opened a file in `macos/parity/evidence/` that shows the behaviour working; named per row in
`evidence`. The PNG screenshots and pairs the tracker cites (`png:rw/...`, `pairs/*.png`) are not in this tree, and `visual-run.sh`
covers only the terminal grid and Settings pages, so no row got credit for a picture. Largest: W11 50, W10 40, W04 25, W05 20, PL01 20,
W02 18, A01/A06/W06/W07/W08 15, A03 9. Nothing recorded (a file is missing or does not show it): W01, W03, W09, W12, A04, PL07, PL09,
A07 (`s3-launch/` holds only a README, no transcripts). Two recorded files are prose summaries with quoted values (W11, A06), not raw logs.

## Raised (0 to 0.5): 4
- **PL07**: the open handler is proven (folder, `.command` run in its folder, web address and missing file refused). Default-terminal role, Info.plist document types and usage strings have no test.
- **PL09**: SIGTERM (two scenarios) and the second launch (e2e verb plus the real lock in HookBridgeTests) are proven. Only SIGTERM with a hosted terminal rests on composition.
- **A07**: the widened mode held back and asked about, and the minute's timeout, both have passing scenarios and core tests. The profile-follow clause has no test.
- **W12**: theme, window colour, font, terminal colours, status bar, profile tag text, live follow of the other profile's file and per-profile writes are read from what draws each window. The colour edge and interface font are not.

## Lowered: 10 (each with the named, concrete reason)
- **W01 1 to 0.5**: zoom is only a core call in a bridge test; no scenario zooms a pane in the app, drags a divider or a pane header, or picks a layout from the menu.
- **W02 1 to 0.5**: Open in menu and launch, the path card and breadcrumb clicks are driven by nothing. (Title facts, ports, branch chip, path menu and Copy path are proven.)
- **W04 1 to 0.5**: the fit rules are proven on the real core (bridge test), but no scenario drags an edge, quits and relaunches to see the widths kept, or reads the sliding; `WindowGeometryTests` pins Swift copies of the numbers.
- **W05 1 to 0.5**: restore of terminals, split, text, a second window and a hosted two-window relaunch are proven; the frame applied to a live window and a two-window relaunch with groups and widths are not. Full screen is simulated in a test instance.
- **W06 1 to 0.5**: switch, new, delete, open and move window are proven. A hidden profile's terminals still running and the menu or Settings buttons are not. `ProfileFolderAtLaunch` is disabled for a real defect (see below).
- **W09 1 to 0.5**: minimise, zoom and (simulated) full screen are read; the collapse buttons are never pressed and real chrome is unread.
- **W10 1 to 0**: no scenario or unit test reads the backdrop (`backdrop` command exists, called by none). Only Settings captions are proven.
- **A01 1 to 0.5**: the first run's steps and its look writes are proven; the welcome program playing in a terminal, the agents step with a detected agent and the Import press are not (core tests only).
- **A06 1 to 0.5**: the approval path and revoke are proven on the real core; the trust window itself (opening, its list, the changed state) is never driven.
- **W11 1 to 0**: the only evidence for hook effects (bell status item, notify, offer, paused chips) is a transcript; no scenario reaches it.

## Kept: 13
W03, W07, W08 (0.5); PL01 to PL06 (0.5, the rest is manual); A03, A04 (0.5); A02 and A05 (1; A05 is the heaviest row at 2,089, its agent-access gate is proven by scenarios and by core desk tests).

## The ten behaviours most worth a new scenario (weight x unproven share)
1. **A01 (823)** The welcome terminal program played through the app: token only in that terminal's environment, the page walk, hooks installed under the scratch agent home, finish remembered; plus the agents step with a stand-in agent on PATH and the Import button.
2. **W04 (460)** Drag a column edge (`edge sidebar 420`), quit, relaunch, read the same width back; collapse and reopen each column and read the state.
3. **W10 (450)** Read `backdrop` per setting: blur on, off, Reduce Transparency, Increase Contrast, glass on, and that its appearance follows the theme.
4. **W02 (438)** Open in: pick an app from its menu and read the recorded launch with the folder; the path card; a click on a breadcrumb folder.
5. **W01 (326)** Zoom a pane by the key and by the pane button and read the zoomed pane drawn; drag a divider and a pane header; pick a layout from View > Layout.
6. **W03 (288)** Click the attention button and read the terminal and window it jumps to; read the state item's elapsed time and exit code; the size item against the real grid after a column opens.
7. **W11 (273)** A fixture Probe with bell, notify, offer and a failing hook: read the status item's tone, the banner log, the offer chip and the paused chip.
8. **W05 (250)** Two windows with groups and widths, SIGKILL, relaunch: each window back at its frame (read from the NSWindow) with groups, columns and selection.
9. **PL01 (175)** Run every menu item through `menu-pick` and read an effect (the S2 `menu-keys.py` is a transcript, not a scenario).
10. **W12 (150)** Read the profile-colour edge and the tag's dot in the other profile's window, and that profile's interface font and radii (window-look does not print them).

Not in a row's weight but asked about: no scenario kills the app with SIGKILL while terminals are hosted and checks the next launch re-attaches them (and a host started in the last minute, before any save, is adopted as an orphan). Core tests cover the plan (`hosting/tests` gone_hosts_replay_and_unnamed_hosts_are_orphans; core store sessions test); the app's adoption is unrun. `textAfterCrash` proves the crash case only for unhosted terminals.

## Things noticed while reading
- `ProfileFolderAtLaunchTests/unopenableShownProfileAtLaunch` is disabled for a real defect: the Swift app exits with code 2 and no window when the shown profile's data folder cannot be opened (GPUI runs with no store and says so). Not covered by the passing sticky-warning scenario, which only switches into such a profile while running.
- Passing but weak: `AuxWindows/hostsLicensesUpdates` checks `licenses-select serde` against the words it was given; `firstRunAndWelcome` checks replies begin with the command sent; `toolingPictures` counts frames and files. `AgentAccess/accessAsked` does not assert that the request is held before the answer (`accessRefused` and `accessWidenedOutside` do, by checking that nothing opened or typed).
- Full screen in a test instance is simulated (`WindowFrames.simulatesFullScreen`), so `fullScreenComesBack` proves the save and restore of that state, not macOS's transition.
- The host-related scenarios were enabled in the pass (they would show as skipped otherwise), so session-host evidence stands.

# Regrade 12, lane D: crosscut, studio, settings (25 rows)

Read only, against the snapshot `3dafdb9e5` and `e2e-status-pass25.md` (312 passed, 4 disabled; none of the four is in these rows).
Weighted over the 25 rows (weight 44,227): proven 51.8 %, manual 25.2 %, unproven_auto 22.9 %, of which recorded 3.5 % (ninth CSV column `recorded`: a driven transcript in `macos/parity/evidence/` that I opened and that shows the behaviour on the real core). Weighted works_real was 0.71, is 0.54.

How Settings and the Studio were counted. `settings-press` and `settings-record` are real injected pointer events at the control's
recorded frame, so a switch, segment, stepper or button pressed by them and read back in `config.json` and in the app counts as
clicked. `set key=value`, `settings-search`, `settings-section`, `settings-reveal`, `settings-reset-confirm`, `font-picker`, `profile`,
`import-*`, `settings-cli-tool` and every `studio-*` line are commands into the model: the value and its effect are real (real core),
the control is not clicked, so those count for half. A SwiftUI pop-up menu never opens under an injected click (MANUAL.md), so a
pick from a select is manual. A row that is only rendered is unproven_auto. The Studio gallery has 32 specimens; `colour-count` over
the window's own picture reads three of them (sidebar, Diff, Terminal ANSI slots); the rest is by eye.

## Raised

- X14 0 to 1 (800). Its notes are stale: user theme by name and by file with live recolour, a relaunch keeping a card pick, version 1 files
  (old text kept, a newer file never written), the project `theme.json` while its terminal is in front, and icon themes (offered, picked,
  drawn, None, flag off) each have a passing scenario. The icon drawing and the pop-up pick are manual.
- SD03 0.5 to 1 (1728). Shelves, filter, hidden themes, the newer mark, card click with a relaunch, a user file and Import theme / Move to Trash
  all run. The Studio's own theme menu is counted under SD01.
- X13 0 to 0.5 (250). One unit test over the real core and CoreText registers a font at start and follows the folder, but no scenario
  draws in a font from the folder, and that test skips itself on a Mac that has JetBrains Mono installed.

## Lowered (each with its reason)

- X01 1 to 0.5 (11,363, the heaviest row). Tokens are proven (Swift equals Rust's dump, derive, increase contrast). Rows, pills, badges,
  key caps, banners, icon buttons, tabs, chips and their states are read by no scenario; the six snapshot tests only write PNGs.
- X12 stays 0.5 (5,609). The control line is the suite's own harness and about 316 of 466 command words are used, but inspect nudge,
  shot PNGs, perf reader lines and a dozen commands have no scenario.
- ST09 1 to 0.5 (900). `font-picker` sets the model's `isOpen` and query; the popover was never opened, nothing was picked by click or key.
- ST01 1 to 0.5 (1,200). The per-row reset (arrow or Reset to Default) is only listed, never used; Reset all and search go through
  commands, not the button or the field.
- STX1 1 to 0.5 (1,047). No scenario opens the Extensions page with a Probe in it; approve and off are proven in a Rust test only.
- STX3 1 to 0.5 (450). No scenario presses a skill or MCP row; the unlock key is a command.
- X05 1 to 0.5 (520). Reduce Transparency is read in Settings; no scenario sets Reduce Motion or Increase Contrast and reads a window.
- X08 1 to 0.5 (1,378). The bridge tests cover list, status, send-text, keys, read, new, split, worktree, events, wait, run over the
  real socket and the real executable runs new-terminal, list, notify; `wh mcp` and the executable's events/wait/run have no test.
- X09 1 to 0 (300). Nothing counts an idle window's wake-ups (`--idle` is manual and stops the terminal source first); audit 8's
  unseen-window repository reads are a known deviation.
- X10 1 to 0.5 (807). The counting tests cover the terminal renderer only; the speed baseline file is all nulls.
- X11 1 to 0.5 (117). Isolation has a test; "opens behind, never takes focus" has none.

## Kept

ST02 0.5 (about 22 of 62 rows clicked, 15 by command, 25 only listed), ST03 0.5, STX2 0.5, SD01 0.5, SD02 0.5, X03 0.5, X04 0.5,
X07 0.5, X15 0 (the launch path is unreachable in any test instance); X02 and X06 stay 1.

## Ten behaviours most worth a new scenario (weight x unproven_auto, heaviest first)

1. X01/SD02 (about 2,300 + 640): read a token edit's pixels in the other 29 gallery specimens and the component families (rows, pills,
   count badges, key caps, banners, icon buttons, tabs, chips) with `colour-count`, the technique StudioGallery already has.
2. X12 (about 1,960): scenarios for `window-shot` (a non-blank PNG), `inspect nudge/reset/hover/pick`, `perf-profile` reader and lock
   lines on a perf-counters build, `reduce-motion`, `increase-contrast`, `window-front`, `keyboard-navigation`, `menubar-icons`.
3. ST02 (about 910): click the roughly 25 unclicked rows: density, cards, scrollback stepper, bell sound and visual, clipboard-read,
   default shell pick, line height, interface font, opacity slider, plus the Profile page's fields and Delete.
4. SD01 (about 750): the Studio through its own controls: expression field with completion, colour well, token list and filter
   clicks, close with unsaved edits (Save sheet), Revert To, Update from, New Theme From, Move to Trash.
5. ST03 (about 530): the remaining Notifications rows through `notify-sim` (agent-done, bell, only-unseen, in-front, dock-badge), the
   Import card clicked, the Agents access list Revoke, the Preview page.
6. STX1 (about 420): the Extensions page over a real Probe folder: list, approval sheet, approve with `WORMHOLE_AUTH=allow`, off, Forget.
7. X10 (about 400): counting guards for the sidebar, panel and diff frames, and one `perf-compare.sh` run to fill the baseline.
8. ST09 (about 315): the font picker driven by keys: arrows, Return writes `font_family`, the monospace/all toggle, recents after a relaunch.
9. ST01 (about 300): per-row reset (right-click, Reset to Default picked with `menu-pick`), the Reset all button, Esc ladder, nav click.
10. X04/X09 (about 230 + 210): arrows inside Notes, Project and Usage and one Tab stop per panel section; an idle window's wake-ups
    counted, and that a window nobody sees reads no repository.

## Noticed while reading

- Six snapshot tests (StudioSnapshotTests, GalleryRenderTests, SidebarSnapshot, ThemeSwatchSheet, PlatformSnapshotTests,
  PaletteTests/SnapshotTests) have no assertion: they pass whatever they draw.
- `FontFolderTests` is `XCTSkipIf` on a Mac with JetBrains Mono installed: a pass there can be a skip.
- `features.csv` notes for X13, X14, SD03 and the Settings rows are stale (they still say "not built"); `built.csv` and the tracker disagree.

## Recorded (not proven by the suite, shown in `macos/parity/evidence/`)

Counted only from files I opened. The screenshots the evidence names (`*.png`) are not in the tree, so no row counts on a picture alone.
No `visual-run.sh` table covers these rows (the two visual tables are the terminal's).

- ST02 21 and ST03 20, ST01 10: `s3-settings/pages-run1.txt`, the rerunnable `drive.py`: 310 ok, 15 FAIL. It presses every toggle, segment,
  slider and checkbox of Appearance, Fonts, Terminals & sidebar, Git, Notifications, Menu bar, Updates, Hotkey and Advanced with the injected pointer, reads
  the value in the model and in `config.json`, and presses the reset arrow (present, puts the key back, gone from the file). Gaps in that run: the Terminal
  page is not in it, pop-ups are picked with `settings-pick` (the binding, not a click), fonts, recorders, actions and panels are "driven in their own run"
  (not saved here), Import lists no row, and the Preview/Motion page fails (14 FAIL) as does one Appearance checkbox (`studio_translucent`).
- STX1 25: `r-verify/STX1.txt` shows the Probe list, sheet, approve, off and a broken Probe refused, by `probe` commands.
- X13 30: `s3-themes-fonts/X13-run-a.txt` shows `resolved=Inter` at launch, `Helvetica` after removal, `Inter` again, and a replaced file re-read.
- STX2 5 and SD02 5: small (`r-verify/STX2.txt`; `s3-themes-gallery/SD02.md` is a ticked list with its pictures missing).
- Looked at and not counted: `s3-themes-motion/` (Reduce Motion, Increase Contrast: only `window-shot` lines, pictures missing), `r-verify/X07.txt` (no
  pseudo-language word in the transcript), `r-verify/SD01.txt` (model state lines only), `s-tooling/keys-a11y-perf.txt` and `pointer.txt` (what the suite
  already proves).
