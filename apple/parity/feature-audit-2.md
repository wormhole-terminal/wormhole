# Feature audit 2: P01-P09, O02, W08, O04, GPUI against Swift

Branch `exp/swift` (worktree `wormhole/.claude/worktrees/swift`, at 271894722). Read-only: no file in the
repository changed, nothing built or run, no instance started. Everything below is from reading the source;
where only a run could settle a point the section says "not run".

Method, as in `apple/parity/feature-audit.md`: per row the GPUI feature was read in its source and written down as
behaviours; each behaviour was looked up in `apple/swift/Sources` and `apple/ffi/src` and counted present only with
a named file and function reached from the user's action (a function with no caller is absent). Five readers
worked one group of rows each. These findings were then checked again by hand and hold:
`NotesCoordinator.sync` sets `saved = merged` before the model (Wormhole/Panel/NotesCoordinator.swift:98-99, against
the save test at :57); `AppController.panelUsage` is `sessions.first?.panelUsage` (AppController.swift:116, used by
Hooks/HookHost.swift:28); `RealWorktreeBackend.setOptions` has no caller and `worktreeCreated: { made, _ in` drops
the environment (Wormhole/PanelBackendsFactory.swift:51-59), and nothing in `ffi/src/terminal.rs` names `WORMHOLE_PORT`; the quit, close and window questions read
`state.sections.flatMap(\.rows)` (AppController+Quit.swift:47, 53, 89; AppController+WindowEnding.swift:33);
`TrustBridge.approve` calls `review()` again before `approveReviewed` (AuxBridge/TrustBridge.swift:40-50, against
ffi/src/aux_backends/trust.rs:78-123); `PanelEffects.open` types `run` into a terminal that is already open
(Wormhole/Panel/PanelEffects.swift:44-50); the viewer splits on the Character `"\n"`
(PanelUI/Stores/FilesStore+Viewer.swift:29), which in Swift does not split `"\r\n"`; one `PanelController` per
window (WindowUI/Hub/WindowHub.swift:53, 65). The rest stands on the readers' reading.

Paths: GPUI paths are relative to `crates/wormhole/src/` unless they start with `crates/`; Swift paths to
`apple/swift/Sources/`; FFI paths are written `ffi/src/...`. Sizes: S under 60 lines, M 60 to 250, L more.

## Result

12 rows audited. **None is complete; all 12 have gaps.** P05 and O04 are graded works_real 0.5 in the tracker
today (the other ten 1); their known gaps are confirmed and both have more than their notes name.

| Row | Verdict | Gaps | Row | Verdict | Gaps |
|---|---|---|---|---|---|
| P01 Panel shell | gaps | 6 (all S) | P07 Files | gaps | 16 (3 M) |
| P02 Changes | gaps | 9 (1 M) | P08 Project | gaps | 12 (2 security, 3 M) |
| P03 Diff | gaps | 11 (2 M, 1 L) | P09 Usage | gaps | 3 (all S) |
| P04 History | gaps | 13 (2 M) | O02 Shortcut sheet | gaps | 3 (all S) |
| P05 Worktrees | gaps | 9 | W08 Close and quit | gaps | 10 |
| P06 Notes and tasks | gaps | 9 (1 M) | O04 Context menus | gaps | 18 |

Four findings cut across rows and carry the most weight:
1. **Panel state is per window, not per terminal** (P02, P03, P04, P07). GPUI keys the Changes, diff, review,
   History and Files state by session; Swift has one `PanelController` with one store each per window. The worst
   result: review comments written beside one terminal are pasted into another.
2. **The close and quit questions are decided from the sidebar's visible rows and their wording** (W08, P05).
   Terminals in a folded group or hidden by the sidebar's search are not seen (no question at quit or window
   close; ⌘W on one does nothing); an idle agent closes without a question; a failed command counts as running.
3. **Profiles are ignored below the window** (P04, P05, P07, P08, P09): trust store, pins, worktree records, `gh`
   approvals and usage reports use the Default or the active profile, not the window's.
4. **The folder handed to the panel is the cwd, not the repository root** (P07, P08): the Files tree, the folder
   colour and "Create project folder" act on the subfolder.

The lanes' own checklists: the misses are named per row under "Checklist misses". The pattern is the same in each:
rows marked YES that the source contradicts (per-terminal state, layout reuse, "open is link-safe", the feature
tag hiding actions), and rows marked NO or "wrong" in a checklist that were then dropped from the tracker's notes
(copy notices, ‹ › wrap, the submodule card, the worktree settings).

The per-row sections follow (P01 to P09, W08, O02, O04), then the ranked list of every gap.

---

## Notes from the readers

Reader A: Note on structure: the Swift panel shell is not a second implementation. The open tab, the panel's width, wanted/fits and the
drag rules are core's `workspace` model (`crates/wormhole-core/src/workspace/panel.rs`, `view/layout/mod.rs`) through the FFI;
the notes' merge is core's `plan_sync`; usage parsing is core's. The gaps below are where the Swift side adds its own rule or
leaves one of the GPUI window's out.

Reader B: Paths: GPUI relative to `crates/wormhole/src/` unless they start with `crates/`; Swift relative to
`apple/swift/Sources/`; FFI written `ffi/src/...`. Sizes: S under 60 lines, M 60 to 250, L more.

Reader B: One fact carries several findings in both rows: the Swift window has **one** `PanelController` (one `ChangesStore`, one
`DiffSession`, one `Review`) for the whole window (`WindowUI/Hub/WindowHub.swift:51,63`), and it is told only the selected
terminal's *folder* (`WindowUI/WorkspaceRootView.swift:54` → `PanelController.follow(folder:)`, which returns early when
the folder is the same). GPUI keeps Changes state per terminal (`features/git/changes/mod.rs:78-89` `ChangesOf`, keyed by
session id) and is told when another terminal is selected (`changes/def.rs:123` `selected`).

Reader D: Two findings were checked twice by hand because they carry the most weight: the trust window's approval
(`AuxBridge/TrustBridge.swift` `approve` against `ffi/src/aux_backends/trust.rs` `review`/`approve_reviewed`, and
`AuxModel/Trust/TrustViewModel.swift` `start`), and the folder the two tabs follow
(`WindowUI/WorkspaceRootView.swift:54` hands `selectedTerminal.folder`, which core's snapshot fills from the cwd,
`crates/wormhole-core/src/workspace/snapshot/title.rs:104`; no `find_root` on the way).

Reader D: Not in GPUI's Files tab at all, so nothing to match (the brief asked): hiding dot or ignored files, git marks on rows,
New File / New Folder, dragging a row to the terminal, wrap and find in the viewer, a hover "open" icon on a row
(the tracker's "Left: hover open icon" names something `features/files/row.rs` does not have). ↩ on the path head
opens nothing in GPUI either: `window/focus/panel.rs:117-121` never calls a tab's `press` for a band, so the
`Line::Head` arm of `features/files/keys.rs:74-79` is unreachable; the lane's second reading of this was right.

Reader E: The first audit's ranked gap 4 ("closing without the question") still holds in the source, in four places, and is
referred to below, not re-derived: `SidebarUI/Row/TerminalRowView.swift:33` (the row's ×) and `:68` (the VoiceOver
action "Close terminal"), `SidebarUI/Menu/RowMenu.swift:32` (the mouse menu's Close terminal),
`Wormhole/Platform/PaletteSourcing.swift:75` (the palette's ⌘⌫). All send `.close(id)` to the core. The keyboard's
row menu (⌃↩) does ask (`Wormhole/Menus/RowMenuKeyboard.swift:45` → `hub.actions.closeTerminal`), so the same menu
item behaves two ways.

---

### P01 Panel shell: six tabs; Ctrl-Cmd numbers; collapse — verdict: gaps

GPUI does:
- Six tabs in the order Changes, History, Notes, Usage, Project, Files; a tab of a feature that is off is not listed [WindowUI/Hub/WindowHub+Focus.swift `panelTabs`, from core `tabs()`; WindowUI/PanelTabBar.swift]
- Usage only while an agent is in a terminal on screen; Project only while the terminal has a folder, or while it is the tab on show [WindowHub+Focus.swift `isShown`, `agentOnScreen`]
- Opening Usage remembers the tab it came from; when the agent leaves, back there, else Changes [WindowUI/Hub/WindowHub.swift `send`, `tabBeforeUsage`; WindowHub+Focus.swift `leaveMissingTab`] — PARTIAL (only while the panel is on screen, see table)
- Click a tab, ⌃⌘1..6, View menu lines, the tab on show ticked in the menu [PanelTabBar `select`; Wormhole/Actions/ActionRouter+View.swift `viewActions`; WindowUI/Menu/MenuStateMaker.swift `tabAction`]
- Showing a tab opens the panel; with no room the sidebar steps aside [core `show_panel_tab` → `reveal_panel`, via WorkspaceBridge/ActionMapping.swift]
- Only Changes has a count on its tab [PanelUI/Controller/PanelController.swift `badge(for:)`; WindowUI/RightPanel.swift `badges`]
- The tab's own buttons sit beside the tabs (Notes: stash) [RightPanel.swift overlay, PanelUI/Notes/StashPromptButton.swift]
- Content slides in from the side of the tab left; Reduce Motion keeps the fade [RightPanel.swift `arrival`]
- ⌥⌘I and the panel button toggle; wanted-without-room dot; under Zen the key does nothing and the button leaves Zen [ActionRouter+View.swift "toggleRightPanel"; WindowUI/WindowControls.swift; WindowUI/WindowBody.swift]
- Panel gives way to 240, then hides, comes back when the window widens; width 240..720, default 360 [core `Panes`, WindowUI/Columns/ColumnFit.swift]
- Edge drag with readout, sticks then closes past the minimum, double-click resets, Esc puts it back [WindowUI/Columns/WindowBody+Drag.swift `move`, `reset`, `cancelDrag`]
- Tab, width and open state saved per window (the tab is the window's, not the terminal's) [core `workspace/saved.rs`]
- Tab strip is one keyboard stop: ← → switch at once and wrap, ↩ / Space / ↓ go into the content [WindowUI/Focus/WindowFocus+Keys.swift `walkPanelTabs`]
- The key and menu line of a tab that is not offered are gone (feature off: hidden; Usage without an agent: hidden; Project without a folder: disabled) — ABSENT
- Under Zen, showing a tab switches it without asking for the panel — WRONG
- A tab that is on show but no longer offered is left on the tick, and never drawn — PARTIAL
- Closing the panel gives the keyboard back to the terminal — ABSENT
- Tab tooltips name their key ("Changes ⌃⌘1"), as do the stash button and the Notes "+" — ABSENT
- VoiceOver: tab role, and each tab's words with its counts ("Changes, 3 changed files", "Notes, 2 open next steps, 1 note") — PARTIAL

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Actions of a tab that is not offered: GPUI binds a tab's action only while it is offered, hides the keymap rows tagged with an off feature (menus, palette, sheet), hides Show Usage without an agent and disables Show Project without a folder. Swift hides by `flag` only: `showNotes`, `newNote`, `stashPrompt`, `showHistory`, `showProject`, `showFiles` carry `feature`, not `flag` (keymap.json), and `keymapHiddenActions` has no caller outside tests. So the lines stay, and ⌃⌘4 without an agent (or ⌃⌘3 with Notes off) shows Changes and opens the panel instead of doing nothing | window/feature_host.rs:288-296; crates/wormhole-core/src/config/feature_table.rs `hidden_actions`; features/usage/def.rs:46-50; features/project/def.rs:100-103 | WindowUI/Menu/MenuStateMaker.swift `make` (hide by feature tag, hide `showUsage`, disable `showProject`); WindowUI/Hub/WindowHub.swift `send` (drop the request instead of `.changes`) | S | yes |
| Under Zen mode `show_tab` switches the tab but does not `reveal_notes` (it would change what the window comes back to). Swift sends `.showPanelTab` to core whatever Zen says, and core's `show_panel_tab` sets `panel_wanted` and may close the sidebar for room: leaving Zen then shows a panel that was closed, or no sidebar | window/layout.rs:63-73; window/panel.rs:147 | WindowUI/Hub/WindowHub.swift `send` (under `look.zen` set the tab without revealing; needs a core action or flag for "tab only") | S | yes (after Zen) |
| A tab on show that is not offered: GPUI draws Changes in its place and the window's tick leaves it (panel open or not). Swift leaves it only from `RightPanel`'s `.onChange(of: hub.panelTabs)`, which exists only while the panel is on screen and fires only on a change: a layout restored with `usage` and no agent, or an agent that left while the panel was closed, shows the Usage body with no tab selected in the strip | window/panel.rs:31-34; window/tick.rs:80; window/focus/bars.rs:93-104 | WindowUI/RightPanel.swift (also `.onAppear`), better in WindowUI/Hub/WindowHub+Focus.swift (observe `panelTabs` in the hub) | S | yes |
| The panel's toggle, when it closes the panel, focuses the terminal. Swift's toggle only sends `.togglePanel`; nothing calls `focusTerminal` when the panel goes, so a note or list that had the keys leaves the window without a key view | window/layout.rs:93-100 | Wormhole/Actions/ActionRouter+View.swift "toggleRightPanel"; WindowUI/WindowControls.swift:35 (then `hub.focus.focusTerminal()` when the panel is now hidden) | S | yes |
| Tooltips with the key from the keymap: each tab, "Stash prompt into a new note ⌥⌘S", "New note ⌥⌘N". Swift's are the bare words | window/chrome.rs:278; features/notes/view.rs:31-33, 427 | WindowUI/PanelTabBar.swift:87; PanelUI/Notes/StashPromptButton.swift:26; PanelUI/Notes/NotesCard.swift:24 | S | yes (hover) |
| VoiceOver: `Role::Tab` in a `TabList`, selected, position in set, and the feature's own words with counts. Swift: a button with the bare title and the value "2 of 6, 3" | window/chrome.rs:279-286, 317-318; features/notes/view.rs:345-352; crates/wormhole-core/src/view/spoken.rs `panel_tab` | WindowUI/PanelTabBar.swift:30-33, `spoken` | S | VoiceOver only |

Checklist misses: `s-panel/checklist-files-usage-shell.md` 1a marks "Key of a tab whose feature is off is ignored, its menu line hidden" YES by pointing at the flag filter, which does not see the `feature` tag (the Usage menu line it did list as NO is still open, and the row's notes never mention it). It has no row for Zen mode and `show_tab`, none for the tab being left on the tick while the panel is closed (its fix went into the panel's own view), and none for the toggle giving the keys back. The tooltip keys and the VoiceOver words it had, and the row's notes pass them to "a person" although the source already answers both.

### P02 Changes: list or tree; sections; count badge; submodules — verdict: gaps

GPUI does:
- Tab body by state: "Reading git status…" until git answered for this folder; "This terminal's folder isn't inside a git repository."; the git problem card (headline, hint, detail well, Retry, Copy error) [PanelUI/Changes/ChangesListView.swift `body` phases; Shell/GitProblemView.swift; ffi/src/git/problem.rs]
- A failed refresh keeps the last list under "Couldn't refresh: <first line>"; a deleted compare branch says "<branch> no longer exists · showing vs HEAD" [ChangesListView `loaded`; Stores/ChangesStore.swift `refresh`]
- Clean tree: the head stays (picker in reach) with "No changes. Working tree is clean." [ChangesListView `loaded`]
- Sections "Changes" / "Untracked" (one "Changes" without grouping, and always one when compared with a branch), count on the band, click folds; a section over 200 files starts folded with the note saying where the files are [PanelModel/Changes/ChangeRows.swift `build`, `foldedNote`; ChangesStore `layout` getter]
- List or folder tree, chains of single folders as "a/b/c", folders fold [ChangeRows `appendTree`; ChangesStore+Walk `toggle`]
- File row: icon, name cut in the middle, folder cut at the front, old name and moved mark of a rename, "+N −M" / bin / LFS, kind letter, tooltip with the whole path [Changes/FileRowView.swift]
- Click opens the diff over the terminal and drops the marks; ⌘-click marks or unmarks; the open or marked row is selected; marks of files that left the list are pruned [ChangesStore+Walk `click`; ChangesStore `open`, `refresh`]
- Right-click a file: Open in <editor> (not with Finder chosen; off for a deleted file), Reveal in Finder, Copy Path, Copy Relative Path, File History; several marked: Copy Paths, Copy Relative Paths, History of These Files; a folder row has the same menu; a submodule row none [Changes/PathMenu.swift; FolderRowView `.pathMenu`]
- Hover icon on a row that opens the file in the chosen editor (none on a deleted file) [FileRowView `body`] — PARTIAL (not with Finder chosen)
- Head controls on the first band: compare picker ("vs HEAD" / branch), view options (List, Tree, Group untracked: the same settings as Settings → Git), Refresh [ChangesListView `head`; Shell/RevPicker.swift; Changes/ViewOptionsMenu.swift; Stores/PanelPreferences.swift `persist`]
- Picking a branch drops the open diff and reads the status against it; Return in the search picks the first match [ChangesStore `setCompare`; RevPicker `onSubmit`]
- Keys through core's walk: arrows over bands, folders and files, ← → fold, ↩/Space, one ⇥ stop per section, an open diff follows the cursor [Shell/SelectableListSplit.swift `handle`; ChangesStore+Walk]
- Submodule row: its words from core, a button that opens a terminal in its folder (checked out, not deleted); click opens its card [Changes/SubmoduleRowView.swift; WindowUI/Hub/WindowHub+PanelHost.swift `terminal.open`]
- Count of changed files on the tab [Controller/PanelController.swift `badge`; WindowUI/RightPanel.swift `badges`]
- Reading: file events through core's `git::watch` rules with a slow poll behind, a folder that becomes a repository is noticed [ffi/src/git/watch.rs; GitBridge/RepoChanges.swift]
- GPUI's Changes tab has no stash UI (stashes are History's picker only) and no ⌘C on a row (`list.rs` `copy: None`): nothing to match.
- Folds, folded folders, marks, the compare branch and the review comments belong to the terminal; selecting another terminal closes the diff and the picker — WRONG (one store per window, see above)
- "File History" shows the History tab with the log filtered — WRONG (the strip is not switched)
- "File History" is offered only while the History feature is on — ABSENT
- ⌘-click does not mark a submodule — WRONG
- An untracked nested repository ("dir/") is a folder row without counts — WRONG
- Picker hints: "Loading branches…", "Changes since this branch split off from it" — ABSENT
- Copies post the notice chip ("Copied path", "Copied error") — ABSENT (the lane lists it)
- Showing the tab reads the repository again — ABSENT (the watch runs whatever tab shows, so rarely seen)

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Changes state is the terminal's: selecting another terminal closes the diff and picker, and each terminal keeps its own compare branch, folds, folded folders and marks. Swift: two terminals in one folder share everything and the diff stays open over the other terminal; a terminal in another folder resets compare to HEAD (`ChangesStore.follow`) but keeps the folds | features/git/changes/mod.rs:78-89, 134-141; changes/def.rs:123; window/repo/state.rs:39-41, poll.rs:66-72 | WindowUI/WorkspaceRootView.swift:54 (pass the terminal id); PanelUI/Controller/PanelController.swift `follow`; PanelUI/Stores/ChangesStore.swift (state keyed by terminal) | M | yes |
| "File History" / "History of These Files": GPUI shows the History tab. Swift `PanelController.showFileHistory` sets only `controller.tab`; the window's tab is `hub.state.panelTab`, which nothing changes, and `PanelBody.sync` writes `controller.tab` back only when the window's tab changes. So the body shows History under a strip that still marks Changes, and a click on "Changes" does nothing (the window's tab is already Changes) | features/git/path_menu.rs:65-72 (`history.show_paths`) | PanelUI/Controller/PanelController.swift `showFileHistory` (ask the window: a callback that sends `.showPanelTab(.history)`); WindowUI/PanelBody.swift `sync` | S | yes |
| The History item is left out of the menu while the History feature is off. Swift's menu always has it (and the window can have the tab off: `WindowHub.panelTabs` filters by `enabled`) | features/git/path_menu.rs:22, 65; changes/view.rs:102 | PanelUI/Changes/PathMenu.swift `body` (an environment flag from `hub.panelTabs.contains(.history)`) | S | yes (edge) |
| ⌘-click on a submodule row opens it like any click (`marking = file.submodule.is_none()`). Swift marks it, and it then rides along in "Copy Paths" / "History of These Files" | features/git/changes/view.rs:241, 247-255 | PanelUI/Stores/ChangesStore+Walk.swift `click` (skip marks for `file.submodule != nil`), `mark` | S | yes (edge) |
| A nested repository git lists as "dir/" is drawn as a folder: folder icon, no line counts, its parent as the folder. Swift `ChangedFile.folder` cuts at the trailing slash ("vendor/lib/" gives folder "vendor/lib" beside name "lib"), the icon is a file's and the counts slot shows "+0" or "bin" | features/git/file_line.rs:38, 83-88, 91-93, 105-110 | PanelModel/Changes/ChangedFile.swift `folder`, `lines` (a `.folder` case); PanelUI/Changes/FileRowView.swift (icon) | S | yes (edge) |
| The row's open icon also shows with Finder as the Open in app (it then reveals; tooltip from core). Swift asks `host?.editorName != nil`, which is nil for Finder, so the icon is gone there | features/git/open_file.rs:80-92; changes/view.rs:236 | PanelUI/Changes/FileRowView.swift `body` (use `openTooltip` alone) | S | yes (edge) |
| The picker says "Loading branches…" until the list is in, and under it "Changes since this branch split off from it" | features/git/picker.rs:294-300 | PanelUI/Shell/RevPicker.swift `RevList` (a hint line; `revs` empty while loading) | S | yes |
| Copy Path / Copy Relative Path / Copy error post the window's notice chip. Swift `SystemPanelHost.copyPath` and `notify` only call `Announcements.post` (VoiceOver); Copy error (`panelCopy`) says nothing | features/git/path_menu.rs:30-34; problem.rs:63-66 | WindowUI/SystemPanelHost.swift `copyPath`, `notify` (`terminal.say(.copied(…))`) | S | yes |
| Showing the Changes tab reads the status again | features/git/changes/def.rs:43-45 | PanelUI/Changes/ChangesView.swift (`.onAppear { store.refresh() }`) | S | rarely |

Checklist misses: `checklist-git-tabs.md` line 143 grades "dropping the diff when the terminal changes" YES; it holds only when the folder changes, and nowhere do the checklist, `driven.txt` or the README say that the store is one per window while GPUI's is one per terminal (`s-panel/driven.txt:177` notes it for the *tab* only). "File History" was listed in the driven menus (`s-panel/driven.txt:69`, `s2-panel/driven.txt:11-12`) and never pressed, so the strip staying on Changes was not seen; the History-off case is not in the checklist. The nested-repository row is in the checklist (line 52, graded "n/a") and in no "Left" list since. ⌘-click on a submodule, the picker's hints (checklist line 67, never taken up) and the Finder case of the open icon are not in any "Left" list of the row's notes.

### P03 Diff: split and inline; syntax colours; hunks; review notes; binary — verdict: gaps

GPUI does:
- The diff covers the terminal; its header is the title row; Esc closes it even while the terminal has the keys, and the list keeps the keyboard [PanelUI/Shell/PanelCoverView.swift, PanelCoverTitle.swift; WindowUI/TerminalColumn.swift:42; WindowUI/Focus/WindowFocus.swift `escape`, `swallows`]
- Header: ‹ ›, path crumb, badges New file / Deleted / Renamed from x / No longer changed / Binary / LFS / Mode a → b / No highlighting / Out of date (tooltip: why), "+N −M" only over lines, open-in-editor, wrap, Split | Unified and "esc close" with room for two columns (560), else × [PanelUI/Diff/DiffHeader.swift; Diff/DiffView.swift `hasLines`, `splitMinWidth`]
- Split and wrap are the settings `diff_split` / `diff_wrap`, written by the header [PanelUI/Stores/DiffSession.swift `split`, `wrap`; Stores/PanelPreferences.swift]
- Rows: hunk bands, two gutters unified or one per half in split, +/− marker, tinted rows, hatched fillers, "no newline at end", tabs to `tab_width`, control and bidi characters spelled out, lines capped at 5,000 characters [Diff/DiffRowView.swift; Diff/DiffCode.swift; PanelModel/Diff/DiffRows.swift; PanelModel/Syntax/VisibleLine.swift]
- Wrap off: only the code scrolls sideways, fades and a position bar where code is cut [Diff/DiffView.swift `lines`; Diff/OverflowCue.swift]
- Notes over a column without lines (New file, Deleted, Binary file + sizes in `size_units`, Stored in Git LFS + pointer and Copy object id, Empty file, No content changes) [ffi/src/git/notes.rs `notes`; Diff/DiffNote.swift `SideNoteView`; DiffView `columnNotes`]
- Too large to load unasked: title, detail, Load diff, Reveal in Finder / Reveal Current File in Finder; the choice holds for re-reads of that file [Diff/DiffNote.swift `TooLargeCard`; DiffSession `loadAnyway`, `forced`; ffi/src/git/service.rs `file_diff`]
- "Loading diff…", a failed first read as its message [DiffView `noteText`] (wording differs: "Reading the diff…")
- Stale answers never land under another file's name [DiffSession `read` cancels the task]
- A working-tree diff follows its file: re-read in place when its status entry or the file on disk changes, "No longer changed" when the status drops it, "Out of date" when a re-read fails; ‹ › follow the list's order [ChangesStore `keepOpenDiffInStep`, `order`; DiffSession `reread`, `markGone`] — PARTIAL (two triggers missing, below)
- Open at line: an icon at a line's right end under the pointer, working-tree diffs of a file on disk only; a removed line opens where it was [DiffRowView `DiffLineView` overlay; DiffView `lineOpen`; ffi `open_line`]
- Review: click a line of a working-tree diff, write / change / remove a comment (⌘↩ saves, empty removes), a primary bar on commented lines, a bar under the rows with the summary, Clear and Paste into Terminal; paste only into a program with bracketed paste, else the clipboard and "Copied the comments instead"; a paste clears the comments, closes the diff, focuses the terminal [Diff/CommentEditor.swift; DiffRowView `onTapGesture`, `popover`; Diff/ReviewBar.swift; ChangesStore `pasteReview`; Changes/ChangesView.swift alert; WindowHub+PanelHost `terminal.paste` with `takesPasteAsText`] (a popover, not a dialog: the lane lists it)
- No comments on a diff against a branch or a commit [ChangesStore `canReview`]
- GPUI's diff has no hunk navigation keys, no find, no copy and no image diff: nothing to match.
- The comments are the terminal's and go to that terminal — WRONG (one `Review` per window)
- ‹ › wrap round the list — WRONG (clamped)
- A submodule's card: old → new hashes, "New commits" / "Commits left behind" with counts, a row per commit (subject, hash, author, age), "and N more" — PARTIAL (one chip of text)
- Header badge with the submodule's words — ABSENT
- "File N of M" always, and the commit's short hash from History — PARTIAL
- Closing by the header's button gives the terminal the keys — ABSENT
- "Renamed from <name>" also for a move that kept the name — WRONG (says "Renamed")
- Header and line open icons with Finder as the app — ABSENT
- Syntax colours from tree-sitter over the whole old and new file — PARTIAL (Swift's line lexer; the lane lists it)
- Rows built once per diff, off the main thread — WRONG shape (rebuilt in the view's body)

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Review comments are kept per terminal and pasted into the terminal the diff was opened from. Swift keeps one `Review` in the window's `ChangesStore`; `follow` never clears it and `host.paste` goes to the selected terminal. Comments written beside terminal A are still there under B (same folder: bars and the review bar show; another folder: the bar shows on any working-tree diff), and "Paste into Terminal" sends A's comments, with A's paths, to B | features/git/changes/mod.rs:87-88, 122-125; diff/review.rs:84-87, 246-273 | PanelUI/Stores/ChangesStore.swift `review` (per terminal id), `pasteReview`; WindowUI/WorkspaceRootView.swift:54 | M | yes |
| ‹ › step with wrap-around (`rem_euclid`): from the last file to the first. Swift `ChangesStore.step` uses `ListNavigation.move`, which clamps, so the arrows stop at the ends | features/git/diff/mod.rs:302-312 | PanelUI/Stores/ChangesStore.swift `step` (wrap for the header's buttons; the list's arrow keys stay clamped) | S | yes |
| Submodule card: "abc1234 → def5678" in the code font, sections "New commits" and "Commits left behind" each with its total, one two-line row per commit (subject; short hash, author, age), "and N more", then the notes, in a body that scrolls. Swift joins the label, the notes and "+ abc1234 subject" lines into one string shown as a single chip over the hatch: no hashes line, no totals, no author or age, no "and N more", and a long list is cut off by the card | features/git/diff/submodule.rs:23-87, 98-122, 126-165 | ffi/src/git/records.rs `GitDiff::Submodule` (old, new, totals); GitBridge/GitDiffFile.swift `file()`; a new PanelUI/Diff/SubmoduleCard.swift from DiffView `content` | M | yes |
| Header badge with the submodule's words ("2 new commits") where a file has "New file" | features/git/diff/header.rs:112-115 | PanelUI/Diff/DiffHeader.swift `badges` (`file.submodule`) | S | yes |
| "File N of M" shows whatever the width; Swift shows it only when a split fits (`if let position, splitFits`). From History the commit's short hash follows it in primary; `DiffHeader` has no such field | features/git/diff/header.rs:75-76 | PanelUI/Diff/DiffHeader.swift `body` | S | yes |
| Re-read when the base moves (`status.base_rev != spec.old`: the merge-base of a compared branch) and, while comparing with a branch, when the file on disk was edited (the stamp is taken for every working-tree diff). Swift's `stamp(of:)` is nil while `compare != nil` and no base is compared, so a compared diff whose counts stay the same is left stale | features/git/diff/load/mod.rs:109-111, 270-284 | PanelUI/Stores/ChangesStore.swift `stamp(of:)`, `keepOpenDiffInStep` (keep the stamp when comparing; carry `base_rev` in `ChangesResult`) | S | yes (edge) |
| "esc close" / × clicked: the terminal gets the keyboard (`close_diff` focuses it). Swift's `close` is `store.close` alone; only the review paste calls `host.focusTerminal()` | features/git/diff/mod.rs:276-286; header.rs:219-222, 231-234 | PanelUI/Shell/PanelCoverTitle.swift `header` (close, then `host.focusTerminal()`) | S | yes |
| Rename badge: "Renamed from <old name>" whenever the old path is known. Swift uses `ChangedFile.renamedFrom`, the row's rule, which is nil when only the folder changed, so a moved file's header says "Renamed" | features/git/diff/header.rs:98, 105-111 | PanelUI/Diff/DiffHeader.swift `badges` (the last component of `originalPath`) | S | yes (edge) |
| The header's open button and the per-line icon exist with Finder as the Open in app (they reveal). Swift asks `host?.editorName != nil` in both | features/git/open_file.rs:80-92; diff/header.rs:175; diff/render.rs:471-475 | PanelUI/Diff/DiffHeader.swift `body`; Diff/DiffView.swift `lineOpen` | S | yes (edge) |
| Syntax colours come from tree-sitter over the whole old and new file (a removed line from the old file, others from the new). The FFI asks core for no file text (`max_highlight_lines: 0`, `max_text_bytes: 0`) and Swift colours each line with its own lexer (`PanelBackends.syntax` defaults to `LexerHighlighter`, the factory sets no other): a string or comment that spans lines, and every language the lexer lacks, differ | features/git/diff/highlight.rs:25-46; diff/rows.rs:114-131 | ffi/src/git/service.rs `file_diff` (spans from core's parse per line); PanelModel/Diff/DiffHighlighting.swift | L | yes |
| Rows are built once per diff on the background executor and kept. Swift's `DiffView.lines` calls `DiffRows.split/unified` and `numberDigits` in the view's body, and `scrollX` is view state that changes on every sideways scroll step, so each step walks every line again; with a patch loaded by "Load diff" (9 MB in the lane's run) that is the whole patch per frame. From reading, not measured | features/git/diff/rows.rs:99-164; diff/load/mod.rs:88-131, 228-238 | PanelUI/Stores/DiffSession.swift `read` (keep both row lists with the diff); PanelUI/Diff/DiffView.swift `lines` | S | only on huge diffs |

Checklist misses: the checklist grades the comment flow YES and the README says outright that the line comment popover was never driven (`s-panel/README.txt:37`); no entry asks whose the comments are, so the per-window `Review` was not seen. The step wrap is in the checklist (line 108, "NO (wrong)"), was never fixed and is in none of the row's "Left" lists; the same holds for "File N of M" (line 107) and the submodule card (line 156, now a chip, still not the card). The checklist's badge list names the submodule label (line 110) and the round's fixes took every badge but that one. Not in the checklist at all: closing by click handing the keys to the terminal, the re-read triggers while comparing (line 142 names only "file or status entry"), the moved-file rename badge, the Finder case of the open icons, and how often the rows are built.

### P04 History: log; commit details; compare branch; copy; PR info — verdict: gaps

GPUI does:
- Tab from the feature table; log of HEAD or the picked rev, at most `LOG_LIMIT`, count grouped in the head; no paging, no graph [core tabs → WorkspaceBridge/ChromeMapping.swift; PanelUI/History/HistoryView.swift `HistoryListView`; ffi/src/git/service.rs `log`]
- Read off the UI thread, stale answers dropped, re-read when the repository changes, slow poll behind it [PanelUI/Stores/HistoryStore.swift `refresh`, `follow`; GitBridge/RepoChanges.swift; ffi/src/git/watch.rs] (Swift is event-driven with a 30 s poll; GPUI polls every 5 s while the tab shows: same result)
- States: "Reading history…", "This terminal's folder isn't inside a git repository.", git problem card with Retry, the head and picker kept when a branch's log can't be read, empty log = head only [HistoryView.swift `HistoryListView.body` switch on `store.phase`; PanelUI/Shell/GitProblemView.swift]
- Commit row: subject, short hash, "merge", author, age; click toggles (one open at a time); whole subject as tooltip [PanelUI/History/CommitRowView.swift; HistoryStore `toggle`]
- Row right-click: Copy Hash, Copy Short Hash, Copy Message, Copy Subject, Copy Author; message and author read the details first for a commit never opened [PanelUI/History/CommitMenu.swift `CommitCopies`; HistoryStore `text(of:for:)`]
- Opened commit block: full subject + Copy message, compact message with Show more / Show less, trailers, "message continues"/"too long" lines, ref chips with "+N more", author / committer / co-authors, hash with Copy hash and the "more ways to copy" menu, parents as links (only those the log lists), Changes fact with tinted numbers, "Reading details…" and the error line [PanelUI/History/CommitDetailsView.swift; ffi/src/git/shown.rs]
- Files under the open commit: "Loading files…", "No file changes.", file rows at depth 1, submodule rows; click opens the diff with ‹ › over the commit's files [HistoryView.swift `details(of:)`; HistoryStore `open`, `step`]
- A commit's file menu: Open in <editor>, Reveal in Finder, Copy Path, Copy Relative Path, File History; open/reveal disabled when the file is gone [PanelUI/Changes/PathMenu.swift, `onDisk` nil asks the disk]
- File History: the log limited to paths, the tab shown, a chip with × that shows every commit again [PanelUI/Controller/PanelController.swift `showFileHistory`; HistoryStore `filter(by:)`; HistoryView.swift `filterChip`]
- Keys: ↑ ↓ over commits, details row and files; → ← show or fold; ↩ / Space on a commit folds, on a file opens its diff with the list keeping the keys; ⌘C puts the hash of the commit under the cursor on the clipboard [PanelUI/Stores/HistoryStore+Walk.swift; PanelUI/Shell/SelectableListSplit.swift `onCopyCommand`]
- Branch picker: chip "HEAD" or the rev, search box, ↩ takes the first match, branches plus the stashes (History only) [PanelUI/Shell/RevPicker.swift; HistoryStore `loadRevs`, `pick`; ffi/src/git/service.rs `revs`]
- Compare with a branch (Changes tab): "vs <branch>" chip, merge-base diff, untracked not apart, "<branch> no longer exists · showing vs HEAD" [PanelUI/Changes/ChangesListView.swift:29, 53; PanelUI/Stores/ChangesStore.swift `setCompare`]
- PR info (title row chip): `gh` asked per branch through core's `PrCache`, only in a trusted project and when the title row shows the part; number, state colour, checks mark, tooltip words, click opens the URL [ffi/src/workspace_extras.rs `pull_requests`, `approved`; PanesUI/Title/PullRequestChip.swift]
- Each terminal keeps its own History (picked rev, file filter, open commit) — ABSENT
- A copy says what was copied (notice), a cut message says so, a failed read says so — ABSENT
- Details menu (right-click on the block, ↩ on its row): Show more / Show less, the copies, Open Parent <hash> — PARTIAL (copies only, no key)
- A followed parent is scrolled into view — PARTIAL (opened, not scrolled to)
- Palette's commit opens in the tab whatever the log shows — PARTIAL
- Details read again when the log changes (refs moved) — ABSENT
- Ages re-drawn when a label changes — ABSENT (named in the row's notes)
- Refs and parents wrap — ABSENT (named in the row's notes)
- Picker: Esc only closes; "Loading branches…"; "HEAD (current branch)"; closed when another terminal is selected — WRONG / PARTIAL
- Open-in-editor icon only on a commit's files that are in the working tree now — WRONG
- "File History" only while History is on; it folds the open commit — PARTIAL
- PR checks finished → a notification in the first terminal on that branch — ABSENT

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| One `HistoryState` per terminal: the picked rev, the path filter and the open commit survive selecting another terminal and coming back, and a `cd` keeps the picked rev. Swift has one `HistoryStore` per window whose `follow` clears rev, paths, expanded and details whenever the folder changes | features/git/history/mod.rs:47-54, 68-72; def.rs:91-97 | PanelUI/Stores/HistoryStore.swift `follow` (keep rev/paths/expanded per terminal id; PanelController `follow(folder:)` would pass the id) | M | yes |
| Copy notices: "Copied" chip per kind (hash, short hash, message, subject, author), a different one when the message was cut, and `Failed::CopyCommit` when the details can't be read. Swift's buttons, menus and ⌘C write the pasteboard and say nothing; a failed read is silent | features/git/history/copy.rs:33-43, 79-82, 104-108, 128-131 | WindowUI/SystemPanelHost.swift `copy` (as `copyPath` does with `notify`), PanelUI/History/CommitMenu.swift `item`, CommitDetailsView.swift `copyButton`, PanelUI/Shell/SelectableListSplit.swift `onCopyCommand` | S | yes |
| Details menu: right-click on the opened block lists Show more / Show less (when offered), the five copies, then "Open Parent <short>" for each parent the log lists; ↩ on the details row opens the same menu under the row. Swift's block menu is the copies only, and `press` ignores `details-` rows, so the keyboard reaches neither Show more, nor a parent, nor Copy message | features/git/history/menu.rs:41-72, 77-111; keys.rs:62-64; details_view.rs:54-57 | PanelUI/History/CommitDetailsView.swift `.contextMenu` (the `whole` state has to move to the store); PanelUI/Stores/HistoryStore+Walk.swift `press` | M | yes |
| Following a parent opens it and scrolls its row into view. Swift's `show` opens it; nothing scrolls (PanelCard scrolls on `cursor` only, which `show` does not set) | features/git/history/refresh.rs:111-117; view.rs:57-63 | PanelUI/Stores/HistoryStore.swift `show` (set `cursor = "commit-" + id`) | S | yes |
| A commit picked in the palette: the tab is shown, the path filter is cleared, the log goes back to HEAD when the commit is not on the picked branch (`is_ancestor`; always for the stashes), and the commit is opened with files and details read even before the log lists it. Swift calls `history.show(sha)` right after `.showPanelTab`: it does nothing unless the log already loaded lists the commit (filtered log, other branch, log still being read) | features/git/history/api.rs:18-28; refresh.rs:121-166; state.rs:243-265 | Wormhole/Platform/PaletteSourcing.swift `.commit` case, PaletteSourcing+Listed.swift:51; a `HistoryStore.reveal(sha)` | M | yes |
| When the log changes, the open commit's details are read again (a branch or tag moved onto or off it) and kept on screen meanwhile; details are also read at every opening. Swift reads them in `toggle` only: the ref chips stay as they were. Also `refresh`'s guard compares folder and rev but not `paths`, so a filtered answer landing after the chip was cleared is shown as the whole log | features/git/history/refresh.rs:59-68, 89-91; state.rs:378-396, 154 | PanelUI/Stores/HistoryStore.swift `refresh` (re-read `details` for `expanded`; add `paths` to the guard) | S | yes (edge) |
| Ages ("12m", "3h") are re-drawn by a one-shot timer at the next change (`time_words` → `next_age_change`). Swift reads `Date()` when the body happens to re-render (`controller.now` is never set outside samples) | features/git/history/def.rs:43-45; mod.rs:78-85 | PanelUI/Shell/PanelContentView.swift:18 (a `TimelineView` or a timer on core's `next_age_change`) | S | yes |
| Ref chips and parent links wrap onto more lines (up to 8 refs + "+N more", 4 parents). Swift lays both out in one `HStack`: in a narrow panel names are squeezed or cut | features/git/history/details_view.rs:227-232, 295 | PanelUI/History/CommitRefs.swift, CommitDetailsView.swift `refs`, `parents` (a flow layout) | S | yes (hides names) |
| Picker: closing it changes nothing. Swift's `onExitCommand { choose(selected) }` runs `pick`, which folds the open commit, closes its diff and reads the log again (Changes: reads the status again). Also missing: the "Loading branches…" hint while the list is read (Swift shows a list with HEAD alone), the first option's words "HEAD (current branch)" / "HEAD (uncommitted changes)" (Swift: "HEAD"), the compare hint "Changes since this branch split off from it", and closing the picker when another terminal is selected | features/git/picker.rs:79-82, 149-152, 294-298; history/def.rs:81-88 | PanelUI/Shell/RevPicker.swift `RevList.body` (`onExitCommand` should only close), `item`, `RevPicker` (`open = false` on a folder change) | S | yes |
| The icon that opens a commit's file in the editor shows only on files that are regular files in the working tree now (`present_files`, first 2,000). Swift's row shows it for every file the commit did not delete (`onDisk ?? (file.kind != .deleted)`), so a file removed since offers an open that leads nowhere (the menu, which asks the disk, is right) | features/git/history/refresh.rs:18-27; view.rs:332-336 | PanelUI/Changes/FileRowView.swift:47 (pass `onDisk` from HistoryView.swift `details(of:)`, with `host.exists`) | S | yes (edge) |
| "File History" is in the path menu only while History is on, and showing a file's history folds the open commit. Swift lists it always (with History off it selects a tab the core does not list) and keeps the open commit and its diff | features/git/path_menu.rs:21-22, 65-72; history/refresh.rs:76-83 | PanelUI/Changes/PathMenu.swift `body`; PanelUI/Stores/HistoryStore.swift `filter(by:)` | S | yes (edge) |
| PR checks that finish are told as a notification in the first terminal on that branch (`Settled::notification`, through the same path as `Wormhole notify`, so the notification settings apply). The FFI drops `PrCache::finish`'s result | window/title/extras.rs:215-244; crates/wormhole-core/src/git/pr.rs:234-247 | ffi/src/workspace_extras.rs `pull_requests` (take the `Settled`, post it as the model's control notification) | S | yes |
| `gh` is asked under the window's profile's approvals; the FFI's `approved` always reads the Default profile's trust store and config | window/title/extras.rs:183-191; window/feature_host.rs:403-413 | ffi/src/workspace_extras.rs `approved` | S | yes (other profiles only) |

Checklist misses: `checklist-git-tabs.md` had the copy notices (line 182), the palette's commit (184), the picker's words and hints (item 16) and the `present` rule (item 13), but the row's notes list only "details menu items, refs wrapping, age timer" as left, so those four were dropped on the way to the grade (the palette path now exists and is partial, the other three are unchanged in the source). Never on the checklist: History kept per terminal, Esc in the picker running `pick`, details re-read when the log moves, the `paths` hole in `refresh`'s guard, scrolling to a followed parent, "File History" with History off, and the PR checks notification (the lane treats PR info as "the title row's chip" and checked the chip only). `s2-panel-b` says "no difference seen" from a still pair, which cannot show any of these; ⌘C and the copies were explicitly not driven.

### P05 Worktrees: new terminal in worktree; remove — verdict: gaps

GPUI does:
- Shell → New Terminal in New Worktree… and the palette, handled only where the terminal in front is in a repository (greyed otherwise) [Wormhole/Actions/ActionRouter+Edit.swift:21 → PanelUI/Stores/WorktreeStore.swift `ask`; WindowUI/Menu/MenuStateMaker.swift:67]
- Repository = the main working tree, so a terminal in a worktree makes siblings [ffi/src/git/worktrees.rs `repo_of`]
- Refusal as an alert "Can't make a worktree here" with the reason (project not trusted with its settings on) [WorktreeStore `ask`; GitBridge/RealWorktreeBackend.swift `problem`; ffi/src/files_panel/project/service.rs `worktree_refusal`]
- Dialog: names the repository, branch field (placeholder fix/login), "Folder: ~/…" following the typing, git's reason for a name it won't take, Esc / Cancel, ↩ / Create, only one at a time [PanelUI/Worktree/WorktreeSheets.swift `NewWorktreeSheet`; WorktreeStore `replan`, `footnote`, `create`]
- Trust asked again at Create [RealWorktreeBackend `create`]
- `git worktree add` off the main thread, hooks off, never forced; new branch from HEAD or an existing one; ports block found and recorded; recorded in the store's `worktrees` meta [ffi/src/git/worktrees.rs `create`]
- A terminal opens in the new folder; the project's `worktree.setup` runs first in it [Wormhole/PanelBackendsFactory.swift:55-59 `worktreeCreated`; ffi/src/files_panel/project/service.rs `worktree_setup`] (typed with a Return, GPUI pastes and submits)
- Git's refusal: alert "Couldn't create the worktree" in git's words [WorktreeStore `create`]
- Closing a terminal in a worktree Wormhole made asks even when nothing runs: the note, "Close and Remove Worktree" unless it has changes [Wormhole/AppController+WorktreeClose.swift `worktreeClose`; AppController+Quit.swift `closeTerminal`]
- Closing a group: each worktree once, removable only without changes and without a terminal outside the group in it, plural words, "Close and Remove Worktrees" [same, `group: true`; AppController+Quit.swift `closeGroup`]
- Removal after the close, only a worktree Wormhole made, branch stays, forgotten when gone, git's refusal as "Couldn't remove the worktree" [WorktreeStore `remove`; ffi/src/git/worktrees.rs `remove`]
- Worktrees whose folder is gone are forgotten [ffi/src/git/worktrees.rs `made_at` → `prune`] (GPUI at launch, Swift at the first close question)
- `wh new-terminal --worktree <branch> [--folder]`: only opens the dialog, filled in; replies `asked` with branch and folder [ffi/src/hook_server/control/worktree.rs `ask` → HookBridge `.worktreeAsked` → Wormhole/Hooks/HookHost.swift:70 → WindowUI/Hub/WindowHub+Worktree.swift]
- Ports in the environment (`WORMHOLE_PORT`, `WORMHOLE_PORTS`) of every terminal that starts in a worktree Wormhole made — ABSENT (named in the row's notes, for the first terminal only)
- `worktree_base` and `worktree_clone_build_folders` — ABSENT (settings never reach the service)
- `--command` of `new-terminal --worktree`, joined as `setup && command` — ABSENT (refused as Unsupported)
- Command line refused when the project is not trusted or a dialog is already up — WRONG (replies `asked: true`)
- Trust asked of the repository's project, and not at all with project folders off — WRONG
- Close question for a folded group, and terminals in folded groups counted as "another terminal in it" — WRONG
- Worktrees remembered per profile — WRONG (Default profile's store always)
- A dialog that is up follows the window to another profile — not checked on the Swift side (no path found that moves a window while the sheet is up)

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Ports: every terminal that starts with its folder in a worktree Wormhole made gets `WORMHOLE_PORT` / `WORMHOLE_PORTS` (the feature's `spawn` hook, after the project's so they win): the first one, a later ⌘T there, and a terminal restored at launch. Swift's `worktreeCreated` closure receives the environment and drops it (`{ made, _ in`); `.newTerminalIn(folder)` carries none, and nothing looks a folder up in the recorded worktrees at spawn | features/git/worktree/def.rs:37-44; window/terminals.rs:188-204; crates/wormhole-core/src/git/worktree/mod.rs:67 | ffi `terminal.rs` `spawn_options` (look the cwd up in `Worktrees::find`, as the row's note "the core model starts it" says); Wormhole/PanelBackendsFactory.swift:55 | M | yes (`$WORMHOLE_PORT` is empty, the documented use fails) |
| `worktree_base` (where worktrees go, `~` expanded) and `worktree_clone_build_folders` (clone `node_modules`, `target` into the new worktree, removed with it). `GitBridge.makeBackends` is called without them and `RealWorktreeBackend.setOptions` has no caller: the sheet always plans beside the repository and never clones. The command line's reply does read the setting (`control/worktree.rs` `base`), so with a base set it reports a folder other than the one the sheet then makes | features/git/worktree/ask.rs:39-50, 186-187, 280, 310-321; settings_window/sections/terminal.rs:104-110 | Wormhole/PanelBackendsFactory.swift:51 (pass both, `~` expanded) and a `setOptions` call where settings changes are heard | S | yes |
| `new-terminal --worktree <branch> --command <c>`: the terminal runs `setup && command` as one pasted, submitted line, so the command starts only after the setup worked. The FFI refuses any `--command` with `Unsupported`; `WorktreeAsked` carries no command | features/git/worktree/ask.rs:97-132, 343-360 | ffi/src/hook_server/control/worktree.rs `ask`, ffi/src/hook_server/events.rs `WorktreeAsked`, PanelUI/Stores/WorktreeStore.swift `ask`/`create`, Wormhole/PanelBackendsFactory.swift `worktreeCreated` | M | yes (CLI) |
| The setup line is pasted (bracketed) and submitted into the terminal the window opened for the worktree. Swift writes `setup + "\r"` raw into `.current`, whichever terminal is selected once two awaits have passed; the lane's evidence drove the sheet and the close question, not a project with `worktree.setup` | features/git/worktree/ask.rs:355-360 | Wormhole/PanelBackendsFactory.swift:56-58 (`.paste(setup, submit: true, in:)` aimed at the terminal `newTerminalIn` made) | S | yes (edge: multi-line setup, a fast selection change) |
| Command line: `Refused` with the reason when the project is not trusted, `Failed` "a worktree dialog is already up in that window" when one is. The FFI checks neither: it replies `asked: true`, after which the window shows the refusal alert, or `WorktreeStore.ask` resets the branch of the sheet that is up | features/git/worktree/ask.rs:120-125 | ffi/src/hook_server/control/worktree.rs `ask` (call the project service's `worktree_refusal`; a flag for the sheet being up) | S | yes (scripts) |
| Trust is asked about the repository (the main working tree) and its `.wormhole/`. Swift passes the terminal's folder to `allowed` (`problem(making:)`, `create`), and `find_project` stops at the first `.git`: from a terminal inside a linked worktree the project asked about is the worktree's checked-out copy (refused although the repository's project is trusted) or none (allowed although the repository's project is not). Also: with the project-folders feature off GPUI allows without asking (the FFI's comment says it does not look at the switch), and an unreadable `.wormhole` is refused in GPUI with its own words | features/git/worktree/ask.rs:85-86, 119-120, 272; features/project/api.rs:90-106 | GitBridge/RealWorktreeBackend.swift `problem`, `create` (resolve the repository first, as `plan` does); ffi/src/files_panel/project/service.rs `worktree_refusal` | S | yes (edge) |
| Closing a group asks about the worktrees of all its terminals, and "another terminal in it" looks at every terminal of the window. Swift builds both lists from `sections…rows`, and a folded group lists no rows (the comment in `closeGroup` says so): closing a folded group never asks about its worktrees, and a terminal in a folded group does not keep a worktree from being offered for removal. `hasPrefix(made.folder)` is also a string test: a terminal in `…/fix-login2` counts as inside `…/fix-login` | features/git/worktree/close.rs:61-73 | Wormhole/AppController+WorktreeClose.swift `worktreeClose` (take the terminals from the state's full list; compare path components); Wormhole/AppController+Quit.swift `closeGroup` | S | yes |
| The worktrees Wormhole made are kept per profile (each profile's store, swapped with the shown profile, pruned at load). `WorktreeService.load`/`save` use `Store::open()`, the Default profile's database, whatever profile the window is in: worktrees a non-default profile made in the GPUI app are unknown (no removal offer, no ports), and new ones are written into Default's list | app_state/profiles.rs:230-236, 289-293; app_state/mod.rs:262-296 | ffi/src/git/worktrees.rs `load`, `save` (`Store::open_for` the profile the factory already has: `PanelBackendsFactory.make(profile:)`) | S | yes (other profiles) |
| A trust refusal at Create keeps the dialog up with the reason under the field. Swift closes the sheet and shows it as "Couldn't create the worktree" | features/git/worktree/ask.rs:268-278 | PanelUI/Stores/WorktreeStore.swift `create` | S | yes (minor) |

Checklist misses: the lane's checklist had the settings (`worktree_base`, `worktree_clone_build_folders`: line 200 and item 10) and the `--command` refusal (line 196), and both fell out of the row's notes, which end at "ports env not passed, sheet and alert are the system's"; the source still has `makeBackends` called without them and `setOptions` without a caller. It names the ports only as "the spawn env" of the new terminal, not that every later terminal in the worktree (⌘T, restore) gets them in GPUI. Not on the checklist at all: the command line's missing `Refused` / "already up" answers and its folder computed from a setting the sheet ignores; trust asked of the terminal's folder rather than the repository; the folded group in the close question (the group case was driven? no: `s2-panel-b/driven.txt` drives one terminal, a dirty one and a clean one, an untrusted and an approved project, never a group, a folded group, a project with `worktree.setup`, or a second profile); the per-profile store; pasting versus typing the setup. The notes' "setup command still not passed" is out of date: it is passed (PanelBackendsFactory.swift:56-58), though never driven.

### P06 Notes and tasks: next steps; reorder; stash prompt; notes.md sync — verdict: gaps

GPUI does:
- Two cards, Next steps and Notes, each with a count (open steps; notes with something in them), a fold and a "+"; an empty section has no chevron [PanelUI/Notes/StepsCard.swift, NotesCard.swift]
- A step: tick, click to edit in place, ↩ keeps it and opens the next below, ↩ on an empty one ends, Esc keeps and returns the keys, ⌫ on an empty one deletes and edits the one above, edited to nothing is deleted, ⌘↩ sends (⇧ submits), ⌥⌘↑/↓ moves [PanelUI/Notes/TaskRowView.swift; PanelUI/Stores/NotesStore.swift `removeEmpty`, `send(text:submit:)`]
- More than five done steps fold into "N done" after the open ones [PanelModel/Notes/TaskList.swift `rows(showDone:)`; StepsCard `DoneFold`]
- A note: first line is the title (core's rule), chevron folds, click writes, title quiets while writing, placeholder, ⌘↩ sends the selection else the paragraph under the cursor (core's `paragraph_at`), Esc returns the keys, a note left blank is thrown away, ⌫ in an empty one deletes and opens the one above [PanelUI/Notes/NoteRowView.swift; ffi/src/notes_text.rs]
- Hover actions Send to terminal and Delete on steps and notes [PanelUI/Notes/RowActions via TaskRowView, NoteRowView]
- Drag to reorder steps and notes, drop line on the landing edge [PanelUI/Notes/ReorderDrop.swift]
- New Note (⌥⌘N, "+"): Notes tab shown, section unfolded, new note has the keys [ActionRouter+View.swift "newNote"; NotesStore `newNote`, `freshNote`]
- Stash Prompt (⌥⌘S, the strip's button): the agent's input box becomes a new note; nothing typed, nothing happens [NotesStore `stashPrompt`; ffi/src/contents.rs `grab_prompt`] — WRONG in two details (table)
- List keys: two bands as ⇥ stops, ↑↓ over bands, steps, "N done", notes; Space ticks or folds, ↩ writes, → ← fold [PanelUI/Stores/NotesStore+Walk.swift]
- Count of open items on the terminal's sidebar row; "notes and tasks are discarded." in the close question [Wormhole/Panel/NotesCoordinator.swift `tell`; Wormhole/AppController+Quit.swift `stakes`]
- Saved per terminal with the folds, per profile; forgotten with the terminal, kept when it moves window [NotesCoordinator `changed`, `departing`; ffi/src/files_panel/notes_store.rs]
- notes.md: three-way merge for every terminal of the window, only for a trusted project that wants it, base kept across restarts and dropped with the terminal, the Project tab says why it waits or failed, a note keeps its identity through a merge [NotesCoordinator `sync`; ffi/src/files_panel/project/notes_sync.rs; FilesBridge/Notes/NotesMapping.swift `applying`]
- A project's prompt copied into a new note [PanelUI/Stores/ProjectStore.swift:106]
- A merge applied to the terminal is saved like an edit — WRONG for the terminal on show
- ⇧⌘E from the terminal goes into the note being worked on, else the first, else a new one, opening the Notes tab — ABSENT
- Sending gives the terminal the keyboard; with submit, Return follows a moment after the paste — PARTIAL
- The step being written has the same keys and actions as any step (⌘↩, ⌫ to the one above, Send, ✕) — PARTIAL
- An open note is cut at 12 lines as wrapped, the last two fading, "Show all · N lines", cut again after a fold — PARTIAL (known to the lane)
- With the feature off nothing of it reacts (no note made, no sync) — PARTIAL

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| A merge from notes.md goes through `apply`, which marks the window dirty, so it is saved and the row's count follows. Swift's `sync()` sets `saved = merged` before `hub.panel.notes.model = merged`, so `changed()` finds nothing to save: for the terminal on show the merged text is not written to the database and `tell` is not called until the next edit or the next terminal switch. Quit before that and the database holds the text from before the merge while the saved base is the merged one, which `plan_sync` reads at the next launch as a local change (what the file added reads as deleted locally and is taken out of notes.md) | features/notes/api.rs:54-98 (`dirty`); features/project/notes_sync.rs:122-129; crates/wormhole-core/src/project/notes/sync.rs:41-64 | Wormhole/Panel/NotesCoordinator.swift `sync()` lines 94-99 (`persistence.save` and `tell` after the model is set) | S | yes (stale row count; a notes.md edit undone after a restart) |
| ⇧⌘E from the terminal: the notes feature takes the keys (`focus_in`): the note last worked on, else the first note, else a new one, with the Notes tab shown, the panel opened and the section unfolded. Swift's `toggleTerminalAndPanel` goes to the content of whatever tab is on show, and only when the panel is already open | window/shortcuts.rs:11-18; window/feature_host.rs:333-348; features/notes/note_ops.rs:333-354 | WindowUI/Focus/WindowFocus.swift `toggleTerminalAndPanel` (with Notes offered: `.showPanelTab(.notes)` then `freshNote` of the last or first note, or `newNote()`) | S | yes |
| Stash prompt keeps the keyboard in the terminal ("to answer the agent's question") and takes the terminal's selection before the input box. Swift opens the new note for writing (`freshNote`), so the keys leave the terminal, and reads the input box only | features/notes/note_ops.rs:313-329; terminal_view/mod.rs:646-651 | PanelUI/Stores/NotesStore.swift `stashPrompt` (add without `freshNote`, then `host.focusTerminal()`); WindowUI/Hub/WindowHub+PanelHost.swift:14 (selection first) | S | yes |
| Send to terminal (the row's action, ⌘↩) pastes and focuses the terminal; ⌘⇧↩ presses Return after `SUBMIT_DELAY`. Swift pastes and leaves the keys in the note or step; `submit()` writes `\r` in the same turn as the paste (the project's own paste waits 120 ms in `PanelEffects`, this path does not). The row's Send also sends the saved words of a step, not what is typed in its open editor | window/feature_host.rs:241-254; window/feature_terminals.rs:23-33; features/notes/task_rows.rs:142-147 | PanelUI/Stores/NotesStore.swift `send(text:submit:)`, `send(_:)`; WindowUI/Hub/WindowHub+PanelHost.swift:12-13 | S | yes |
| The step being written is an editor like any other: ⌘↩ sends what is typed, ⌫ on an empty one drops it and edits the step above, and it has the hover Send and ✕ (discard). `TaskDraftRow` has ↩, Esc and blur only | features/notes/tasks.rs:84-97, 153-182, 221-228; features/notes/task_rows.rs:136-160 | PanelUI/Notes/TaskDraftRow.swift (`onKeyPress` for ⌘↩ and ⌫, `RowActions`) | S | yes |
| An open note that nobody edits shows the lines after its title as the panel wraps them, at most 12, the last two at 0.65 and 0.3, then "Show all · N lines"; folding resets "Show all". Swift counts source lines (a long paragraph is never cut, twelve short lines that wrap are), fades with one gradient, and keeps `showAll` through a fold | features/notes/view.rs:186-204, 233-264, 289-340; features/notes/note_ops.rs:277-284 | PanelUI/Notes/NoteRowView.swift `bodyText`, `fade` | M | yes |
| Feature off (`notes_and_tasks` flag or the person's switch): `new_note`, `stash_prompt` and `focus_in` return at once and the sync does not run. Swift's "newNote" adds a blank note to the model whatever the tab list says (and its menu line is still there, see P01), and the sync timer runs for every terminal regardless of the Notes feature | features/notes/note_ops.rs:124, 321, 340; features/project/notes_sync.rs:46-50 | Wormhole/Actions/ActionRouter+View.swift:28-29 (guard on `hub.panelTabs.contains(.notes)`); Wormhole/Panel/NotesCoordinator.swift `setSync` | S | yes (flag off) |
| The sync waits only while a step or note of that terminal has an editor open (`being_written`). Swift asks whether the app's key window has an `NSTextView` as first responder: any field of any window holds the merge back, and with no key window (another app in front) a note being written is merged over | features/notes/api.rs:45-50; features/project/notes_sync.rs:86 | Wormhole/Panel/NotesCoordinator.swift:88 (a flag `NotesStore` sets from its editors' focus) | S | edge |
| Small rules kept in Swift instead of core's: a whitespace-only note counts as written in the band (`!text.isEmpty`; GPUI `chunks_blank`); the body under the title is a Swift copy of `note_body` (a first line of spaces becomes the title's line); clicking a note's title puts the cursor at the end of its text; ⌥⌘↑/↓ are read off the keystroke, not the keymap's `list_action` | features/notes/mod.rs:125-127, 198-200; crates/wormhole-core/src/view/notes/paragraphs.rs:56; features/notes/note_ops.rs:154-166; features/notes/items.rs:9-14 | PanelModel/Notes/NotesModel.swift:23-26, 42; PanelUI/Notes/NoteRowView.swift `open`; PanelUI/Notes/ReorderDrop.swift `reorderKeys` | S | edge |

Checklist misses: `s-panel/checklist-notes-project.md` has the merge, the trust wait and "every terminal on the tick", and wave 2 drove three terminals, but nothing asks whether a merged result is saved: the driven case read it back by selecting each terminal, which is the one path that saves. ⇧⌘E into the notes (`focus_in`) has no row at all (`s2-panel-b/driven.txt` uses shift-cmd-e only to reach the Changes list). 1d has the stash's notice as a difference and "nothing refocuses the terminal explicitly", but not that the note now takes the keys, nor the selection. "Send action" was left "not traced further": the focus and the submit delay were never checked. The draft row got a line for its missing ✕ only. The feature-off row is "not checked" and stayed so.

### P07 Files tab: tree; viewer; rename; pins — verdict: gaps

GPUI does:
- Tab offered by core's feature mask (flag `files`, the person's switch) [WindowUI/Hub/WindowHub+Focus.swift `panelTabs`]
- Card: path head, then the tree; "This terminal hasn't said which folder it is in yet." without a folder [PanelUI/Files/FilesView.swift `tree`]
- A folder is listed lazily, off the main thread, by core: lstat only, never through a link, folders first, natural order, names cleaned, 5,000 cap with "N more" / "N+ more" [FilesBridge/Files/RustFilesBackend.swift `list` → ffi/src/files_panel/files.rs `list`]
- Notes under a folder: Reading…, Empty folder, gone / denied / unreadable [PanelUI/Files/FileEntryRow.swift `words`]
- Click: a folder selects and toggles, a file selects and opens the viewer, a link only selects [PanelUI/Stores/FilesStore.swift `click`]
- Keys: one ⇥ stop with the head as its band, ↑ ↓, ⌥↑ ⌥↓, → opens / first child, ← closes / parent, ↩ and Space press, the viewer follows the keys onto files and never toggles a folder by walking [PanelUI/Stores/FilesStore+Walk.swift; PanelUI/Shell/SelectableListSplit.swift `handle`; PanelModel/Shared/PanelWalk.swift]
- Path menu: the folder (checked) and its parents with Finder's icons (generic off the boot volume), the pins (twins say where they are), Pin / Unpin "name", Choose Folder…, Follow Terminal once a folder was chosen [PanelUI/Files/FilesPathButton.swift `content`; PanelModel/Files/PathMenuModel.swift]
- Row menu: Open in ▸ (apps, the chosen one checked; off for a link), Reveal in Finder, Copy Path, Rename…, Pin / Unpin Folder on a folder [PanelUI/Files/FileRowMenu.swift]
- Copy Path refuses a path with a control character and says so; VoiceOver says "Path copied" [WindowUI/SystemPanelHost.swift `copyPath`]
- A test instance only logs Open in and Reveal [WindowUI/SystemPanelHost.swift `launches`]
- Rename in place: the name selected without its extension, ↩ commits, Esc or losing the keys cancels, the same name only closes, core's `rename_within` (refuses what exists, a bad name, anything through a link), a refusal keeps the field open, pins of a renamed folder move in `config.json` [PanelUI/Files/RenameField.swift; PanelUI/Stores/FilesStore+Rename.swift `commitRename`; ffi/src/files_panel/files.rs `rename`, `move_pins`]
- Viewer over the terminal with its header in the title row: path, size (the `size_units` setting), Open in, close; Esc closes [PanelUI/Shell/PanelCoverView.swift; PanelUI/Shell/PanelCoverTitle.swift; PanelUI/Files/FileViewerView.swift `FileViewerHeader`; WindowUI/Focus/WindowFocus.swift `escape`]
- Viewer bodies: Reading…, numbered text with `tab_width` and the code font, control and bidi characters as dimmed placeholders, a line cut at 5,000 characters with …, "This file is empty.", the truncated footer, binary panel with size, Open in… and Reveal, the five "unavailable" sentences (no buttons for gone / outside), a picture scaled to fit [PanelUI/Files/FileViewerBody.swift; PanelUI/Diff/DiffCode.swift `text`; ffi/src/files_panel/files.rs `read`]
- Caps and link safety of the read are core's (2 MiB text, 50 MiB picture, never out of the root) [ffi/src/files_panel/files.rs `read` → `storage::files::load`]
- The tree and the open file are read again when the disk changes (FSEvents, a 2 s poll where there is no stream, a 30 s backstop) [FilesStore.swift `retarget`, `reload`; ffi/src/files_panel/watch.rs]
- The tree is of the terminal's repository root, else its folder — WRONG (always the cwd)
- One tree per terminal: open folders, selection and a folder chosen by hand belong to the terminal; selecting another terminal takes the viewer and a rename down — ABSENT (one store per window; known to the lane)
- One cover at a time, whatever panel tab shows — WRONG (the cover is a function of the tab)
- Lines cut at `\n` with a `\r` before it dropped; a last line break adds no line — WRONG (CRLF files)
- Pictures decoded off the main thread by a decoder that refuses absurd sizes and scales to 4,096 px; "too large" sentence — ABSENT
- Line numbers stay put while the code scrolls sideways — ABSENT (known to the lane)
- Pinned folders are the window's own profile's — WRONG (always the Default profile's `config.json`)
- The title row's path crumb: click shows the file in Finder, resting on it opens the card, right-click the path menu — ABSENT
- Open in button always there (Finder shows the file where no editor is known) — PARTIAL
- What is opened is resolved inside the root and checked to still be a file or folder; a file is never handed to its default app — PARTIAL
- Rename: the catalog's five sentences, the typed name untouched, no cancel while it runs, selection and viewer follow a file inside a renamed folder — PARTIAL
- File and folder icons from the icon theme in force (`icon_theme` setting) — PARTIAL (the bundled theme only; known to the lane)
- The head and the menu name `/` by the disk's own name — WRONG ("Macintosh HD" literal)
- Nothing is read while the tab is hidden or the feature is off; one read per folder at a time; a folder or file that looks the same (one lstat) is not read again — PARTIAL
- Closing the viewer gives the terminal the keyboard — PARTIAL
- Syntax colours from tree-sitter, made off the main thread — PARTIAL (Swift's own lexer, `Wormhole/PanelBackendsFactory.swift` leaves `syntax` at `LexerHighlighter`; counted under P03, not again here)

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| The tree is of the terminal's repository root (`band_folder`), else its folder. Swift follows the cwd: in `repo/crates/x` it shows `crates/x`, and the path menu's first entry is that subfolder | features/files/view.rs:51; features/files/read.rs:145,161; window/facts.rs:85-87 | WindowUI/WorkspaceRootView.swift:54 (`hub.panel.follow`) or PanelUI/Stores/FilesStore.swift `follow` (ask core's `git::find_root` through the FFI first) | S | yes |
| Per-terminal state: each terminal keeps its tree, open folders, selection and hand-chosen folder; another terminal selected dismisses the viewer and a rename. Swift: one `FilesStore` per window; a folder chosen by hand stays for every terminal, a terminal in another folder rebuilds the tree, one in the same folder keeps viewer and rename up | features/files/mod.rs:98-120, 177-180; features/files/def.rs:81 | PanelUI/Stores/FilesStore.swift (state keyed by terminal id, fed from WindowUI/Hub/WindowHub.swift:63) | M | yes |
| One cover per window, independent of the panel's tab: opening a file takes an open diff down for good (and the reverse), and the viewer stays over the terminal while another tab shows. Swift derives the cover from the tab: switching to Notes hides the file, switching back shows it again, and a diff and a file stay "open" side by side | features/files/viewer/mod.rs:166-168; window/feature_host.rs:128, 149-165 | PanelUI/Controller/PanelController.swift `cover` (one stored cover; the stores close each other) | S | yes |
| Text is cut at `\n`, a `\r` before it is not part of the line, and a final break adds no line. Swift splits a `String` on the Character `"\n"`: `"\r\n"` is one Character in Swift, so a CRLF file is never split: it shows as ONE line of ␍␊ pictures cut at 5,000 characters. An LF file gets one extra empty numbered line at the end | features/files/viewer/lines.rs:41-45, 59-63 | PanelUI/Stores/FilesStore+Viewer.swift:29 `readViewed` (split on `unicodeScalars`/`utf8` at 0x0A, drop a trailing 0x0D and the empty last piece) | S | yes |
| A picture is decoded off the main thread by the system's decoder, which reads the declared size first, refuses an absurd one ("This picture is too large to show.") and returns a thumbnail of at most 4,096 px, first frame only. Swift calls `NSImage(data:)` on up to 50 MiB of repository bytes inside the view's `body` (main thread, again on every redraw), with no dimension check and no "too large" case. Hostile input: a small file can declare a huge bitmap | features/files/viewer/image.rs:15-25; features/files/viewer/read.rs:57; features/files/viewer/card.rs:153-159; platform/image.rs:15, 29 | PanelUI/Files/FileViewerBody.swift `picture` and PanelUI/Stores/FilesStore+Viewer.swift `readViewed` (decode once, off main, `CGImageSourceCreateThumbnailAtIndex` with a max pixel size, or core's `platform::image::decode` through the FFI) | S | yes (a stall, memory) |
| Line numbers are a fixed gutter; a sideways swipe moves the code only | features/files/viewer/card.rs:209-304, 321-349 | PanelUI/Files/FileViewerBody.swift `FileLines` | M | yes |
| Pins are read and written in the settings of the window's own profile. The FFI uses `Config::load()`, which is the Default profile whatever the window's (or Wormhole's) profile is. Also: after a rename moved pins in the file, Swift never reads `pins` again, so Pin / Unpin on that folder shows the old state until the root changes | features/files/pins.rs:14-36 | ffi/src/files_panel/files.rs:67-74, 91-95 and ffi/src/files_panel/settings_edit.rs:13-21 (take the profile); PanelUI/Stores/FilesStore+Rename.swift `commitRename` (reload `pins`) | S | yes (with profiles) |
| The file's crumb in the title row is a button: click shows the file in Finder, resting on it opens the path card (Reveal, Copy path), right-click the path menu. Swift draws the path as text | window/title/breadcrumb.rs:167-232; features/files/def.rs:58-62 | PanelUI/Files/FileViewerView.swift `FileViewerHeader` | S | yes |
| The viewer's Open in button is always there: with no known editor it is Finder showing the file. Swift hides it when `editorName` is nil (Finder chosen, or no editor installed), leaving a text file's viewer with close only | features/files/viewer/card.rs:97-123; features/files/open.rs:58-63 | PanelUI/Files/FileViewerView.swift:50-52 `FileViewerHeader` | S | yes |
| Before anything is handed to an app the path is resolved inside the root (`resolve_path_within`) and must still be a file / folder of the row's kind, off the main thread; a file is only ever opened in a known editor or shown in Finder. Swift launches `root + path` unresolved (a folder swapped for a link after the listing is followed), and when core's launch does not start it falls back to `NSWorkspace.shared.open(file)`: the default app, which for `x.command` or a folder named `x.app` is to run it | features/files/open.rs:1-11, 29-43, 66-79 | WindowUI/SystemPanelHost.swift:58-63 `open(_:line:)`, 70-74 `open(_:inBundle:)` (resolve through `repoFileResolve`-like FFI for files and folders; drop the `NSWorkspace.open` fallback) | S | no (security) |
| Rename refusals say the catalog's sentences on the chip ("Not renamed: that isn't a usable name", "… something with that name exists already", "… Wormhole isn't allowed to change it", "… it isn't there any more", "Couldn't rename it"); what was typed goes to core untouched; the field cannot be cancelled or committed twice while the rename runs; the selection and an open viewer follow a file inside a renamed folder. Swift has five other English literals (also under the field), trims spaces first, has no busy guard (↩ twice renames twice, the second reports "gone"), and follows only an exact path | features/files/rename.rs:40-48, 121-127, 137-160, 195-213; features/files/viewer/mod.rs:238-255; crates/wormhole-text/locales/en/notices.ftl:100-110 | PanelUI/Stores/FilesStore+Rename.swift `commitRename`, `words` | S | yes |
| Row, head and viewer icons come from the icon theme the person chose (`SettingsStore.icons`, none included) | features/files/view.rs:99, 114; features/files/row.rs:58-76 | WindowUI/PanelBody.swift:23 (`static let icons = BundledIconTheme()`) and WindowUI/TerminalColumn.swift:42 | M | yes |
| `/` is named by the boot volume's own name, in the head and the path menu | features/files/head.rs:26-32; features/files/menus.rs:21-27 | PanelUI/Stores/FilesStore.swift:122 `pathMenu` (`volume: "Macintosh HD"`) | S | yes (a renamed disk) |
| Nothing is listed while the tab is hidden or the Files feature is off (its flag is off by default in a release); a folder is read once at a time whatever asks (`in_flight`), and a folder or the open file whose lstat stamp is unchanged is not read again. Swift starts the tree, its watch and the reads for every window as soon as a terminal has a folder, flag and tab regardless; every wake lists every open folder again and reads the whole open file (up to 50 MiB) to compare it; a read that hangs on a dead share is started again by every wake | features/files/read.rs:29-34, 44-46, 83-99, 154-167; features/files/viewer/read.rs:48-53, 71-74, 94-109 | PanelUI/Stores/FilesStore.swift `follow`, `retarget`, `reload`; PanelUI/Stores/FilesStore+Viewer.swift `readViewed` (gate on tab shown and feature; stamps from an FFI `stamp_below`) | M | no (I/O, threads) |
| Closing the viewer (the X, Esc) gives the terminal it covered the keyboard. Swift's X only clears the store; Esc refocuses the list when the keys were in an area and nothing otherwise | features/files/viewer/mod.rs:179-190; window/feature_host.rs:194-200 | PanelUI/Files/FileViewerView.swift:53 (close → `host.focusTerminal()`); WindowUI/Focus/WindowFocus.swift:186-188 | S | yes |
| A name that is not UTF-8 is a path like any other (shown cleaned, opened, renamed). Through the FFI it becomes a lossy string, and every call on it answers "gone" | features/files/row.rs:54-55 (paths are `PathBuf`); crates/wormhole-core/src/view/file_tree/dir.rs | ffi/src/files_panel/files_records.rs:16-25, 43 (carry the bytes, or an index into the listing) | M | yes (edge) |

Checklist misses: `checklist-files-usage-shell.md` is thorough on rows, menus and the viewer's panels, and most of
its NO rows were closed in wave 2. What it did not have, or had wrong: (1) the repository root: its row says
"whether that is the repository root is unverified" and nobody verified it; it is not. (2) CRLF files and the extra
last line: it compares the 5,000-character cap and the placeholders but never how the text is cut into lines. (3)
The picture's decoder: it lists the missing "too large" sentence as wording (S) and misses that Swift decodes
untrusted bytes on the main thread on every redraw with no size limit. (4) "Open runs off the main thread,
link-safe: YES (host side), not re-verified": the host does not resolve the path and has a default-app fallback.
(5) The cover being tied to the tab, and the crumb being a button (the title row is the window's, outside
`features/files`). (6) The Files flag being off, not only the tab hidden, still reads and watches. (7) `driven.txt`
of s2-panel-b drove the row menu and the head stop only: no CRLF file, no picture, no terminal in a subfolder of a
repository, no second profile, so none of the above could show. The tracker's remaining "hover open icon" is not a
GPUI behaviour.

### P08 Project tab: .wormhole folders; trust; actions — verdict: gaps

Security summary first. What runs is gated in Rust on both sides by the same rule (present, switched on, trusted
content): `ffi/src/files_panel/project/service.rs` `fresh` → `hub.rs` `refusal` before any effect, read from one
snapshot, with `approve` asking the owner before `trust_request` on the request the tab last showed
(`project/trust.rs:47-90`, `state.rs:16-19`). An untrusted or changed project gets no action, no layout, no worktree
setup, no settings overrides and no notes.md read or write (`notes_sync.rs:79-92`, `overrides.rs:36-40`,
`service.rs:50-62, 172-181`); prompts need the Prompts switch only, as in GPUI. Open URL is http(s) only on the
Swift side too (`FilesBridge/Project/PanelEffect.swift:58-61`). Three things are not at GPUI's level: the Project
commands window can trust content that was never on screen (row 1), trust is read from the wrong profile's store in
a window of another profile (row 2), and "Open terminals" types the layout's commands into terminals that are
already open (row 3).

GPUI does:
- Tab offered while the terminal has a folder (kept while it is the tab on show) [WindowUI/Hub/WindowHub+Focus.swift:16]
- "Folder colour" card: None, a custom hex swatch (not clickable), the eight names, a picker saving a hex colour, the line saying where the colour is kept, dimmed when the project locks it or its `config.json` can't be read; written to the project's `config.json` by core's splice, else to the person's own [PanelUI/Project/FolderColorCard.swift; PanelUI/Stores/ProjectStore.swift `setColor`; ffi/src/files_panel/project/paint.rs]
- No project: the two sentences, "Create project folder" (off without a folder), "Creates ~/x/.wormhole/", the create error; core's `init` refuses what is there; a race is a success [PanelUI/Project/NoProjectCard.swift; ProjectStore.swift `create`; ffi/src/files_panel/project/service.rs `create_project`]
- Project head: name, trust badge trusted / changed / not trusted [PanelUI/Project/ProjectHeadCard.swift]
- Trust callout only when something can run, waits for trust, or notes sync waits; title and text per state (untrusted / changed, with or without commands, "its notes.md sync"); lines wrapped never cut, what runs in `trust.command`; 5 of more than 6 lines and "View all N…" [PanelUI/Project/TrustNotice.swift; PanelModel/Project/ProjectState.swift `notice`; ffi/src/files_panel/project/state.rs `build`]
- Approve: "Trust with Touch ID" / "Trust (approve with password)" / "Waiting for approval…", Touch ID or password first, cancel is silent, a failure reads "Not trusted: …", "Can't trust this folder until it loads: …", exactly what the tab showed is trusted [ProjectStore.swift `approve`; FilesBridge/Project/OwnerAuthentication.swift; ffi/src/files_panel/project/trust.rs `approve`]
- Trusted: the quiet line and Revoke trust [TrustNotice.swift `trusted`; ffi/src/files_panel/project/trust.rs `revoke`]
- Errors (stores reset, save failures, files that don't load, "Notes sync: …") and warnings (an action's key that is taken) [ProjectHeadCard.swift; ffi/src/files_panel/project/state.rs:74-93]
- "Use from this project": a switch per part present and always Shared notes, hint and warning colour from core's standing; per project, for the person only [PanelUI/Project/ProjectSectionCards.swift `ProjectFeaturesCard`; service.rs `set_feature`]
- Actions card (enabled only): title, detail, key glyphs; a click runs: run (typed + Return), prompt (text or file from the checked snapshot, pasted, optional submit), open URL, chain (depth 16), target current / new / named with cwd [ProjectSectionCards.swift `ProjectActionsCard`; ProjectStore.swift `run`; ffi/src/files_panel/project/effects.rs `of_action`; Wormhole/Panel/PanelEffects.swift `perform`]
- Actions from the Project Actions menu and the palette [Wormhole/Actions/ActionRouter.swift:43-45; Wormhole/Menus/RecentProjects.swift:43; Wormhole/Platform/PaletteSourcing.swift:36]
- Prompts card: paste into the terminal (trimmed, not submitted), copy into a new note [ProjectSectionCards.swift `ProjectPromptsCard`; service.rs `send_prompt`, `prompt_text`]
- Terminal layout card: names, "Open terminals" opening each with name, cwd, run, focus [ProjectSectionCards.swift `ProjectLayoutCard`; effects.rs `of_layout`; PanelEffects.swift `open`]
- "View all N…": a "Project commands" window, 640 x 480, every line, status sentence, "Nothing here would run.", the approve button, follows the folder (`git pull`), Esc / ⌘W [PanelUI/Stores/ProjectStore.swift `viewAll` → Wormhole/Aux/AuxController.swift `showTrust`; AuxWindows/Trust/TrustView.swift; ffi/src/aux_backends/trust.rs]
- A trusted project's settings layered over the person's while its terminal is in front, rows marked [Wormhole/Settings/ProjectSettingsFollower.swift; ffi/src/files_panel/project/overrides.rs]
- Worktree of a repository with a project: refused unless trusted with Settings on; `worktree.setup` run first [Wormhole/PanelBackendsFactory.swift:52-59; service.rs `worktree_allowed`, `worktree_setup`]
- The tab reads again when `.wormhole/` or the trust changes and when the window comes forward [ProjectStore.swift `follow`, `reload`; WindowUI/WorkspaceRootView.swift:51]
- Approval in the Project commands window trusts exactly the request captured before the dialog — WRONG (security)
- Trust and switches are the window's own profile's — WRONG (security, with profiles)
- "Open terminals" skips a terminal of that name that is already open — WRONG (types its `run` into it again)
- A trusted project's `env` and `shell` for new terminals in it — ABSENT
- The project's theme (`theme.json`, no trust needed, switchable) — ABSENT
- An action's own key runs it — ABSENT
- Project feature off: no `.wormhole/` is read or written anywhere — PARTIAL (only the tab goes)
- Folder colour and Create are for the band's folder (the repository root) — WRONG (the cwd)
- A named target is brought in front before it is typed into — PARTIAL
- Copy into a new note shows the Notes tab with the note under the keys — PARTIAL
- Hub problems shown in the no-project card too; the head names an older project's `.andromeda` — PARTIAL

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| SECURITY. The approval trusts exactly the request that was on screen: it is cloned before the dialog and `finish_approval` trusts that clone, so files changed meanwhile read "changed". In Swift's Project commands window `approve()` first calls `review()` again (re-reads the disk, replaces the desk's `shown`), and while the Touch ID dialog is up every file event runs `TrustViewModel.reload()` → `desk.review()`, which replaces `shown` again; `approveReviewed()` then trusts whatever was read last. A `git pull`, a checkout or any process rewriting `.wormhole/actions.json` during the dialog is approved by the finger already on the sensor. Where the watch is not live (`follow()` false, no poll) a click approves content the window never drew. The panel's own button is right (`project/trust.rs:51-70` clones first) | features/project/trust.rs:15-27, 54-78; features/project/trust_window.rs:155-166 | AuxBridge/TrustBridge.swift:40-54 `approve`; AuxModel/Trust/TrustViewModel.swift:18-28; ffi/src/aux_backends/trust.rs:78-123 (take the request out of `shown` before authenticating and pass it to `approve_reviewed`, or refuse when the fingerprint moved) | S | no (security) |
| SECURITY. A window reads its own profile's trust store and switches (`hub_of(profile)`), never the shown profile's. Swift builds each window's `ProjectService` from `profile::active_dir()` (the profile Wormhole is in when the window is made); the `profile` handed to `makeBackends` reaches only the notes. A window opened with "Open a window in ▸ Work" while Wormhole is in Default runs Work's terminals under Default's approvals and writes approvals made there into Default's store; `followProfile` is called only by "Move this window to". The Project commands window likewise takes `activeProfileID`, not the window's | features/project/mod.rs:120-132; features/project/trust.rs:33-37; features/project/trust_window.rs:37-38, 128 | FilesBridge/FilesBridge.swift:45-46 `makeBackends` and ffi/src/files_panel/project/service.rs:74-77 `new` (construct with the profile); Wormhole/Aux/AuxBackends.swift:68 `trust(root:)` | S | yes (with profiles) |
| "Open terminals" opens only what is not open: a terminal whose own (custom) name is the layout's is skipped, nothing is typed into it and it is not selected. Swift finds one by its shown title (an automatic title can match), reuses it, selects it when `focus`, and types `run` + Return into it again: a second click, or a restored window, re-runs every command of the layout in terminals that may hold an editor, an agent or a server at that moment | window/feature_terminals.rs:100-126 | Wormhole/Panel/PanelEffects.swift:44-50 `open`, 63-65 `existing(named:)` | S | yes |
| A new terminal started in a project that is trusted with Settings on gets the project's `env` and `shell` (`config.json`), also on restore. The FFI's `spawn_options` adds only the worktree's ports; the Settings row still says "… environment variables and shell from config.json" and the worktree refusal still warns of them | features/project/detect.rs:128-152; features/project/def.rs:63 | ffi/src/terminal.rs:359-374 `spawn_options` (ask the window's `ProjectService` hub: `find_project` + `enabled(Settings)`, read fresh) | M | yes |
| The project's theme (`theme.json`) is laid over the theme while the project's terminal is in front and its Theme switch is on (no trust: it is harmless). Swift shows the Theme switch and applies nothing | features/project/detect.rs:106-124 | Wormhole/Settings/ProjectSettingsFollower.swift `reload` + a `theme_overrides` beside ffi/src/files_panel/project/overrides.rs, into the theme sync | M | yes |
| A project action's key (`actions.json` `key`) runs it in the window in front, checked on every ⌘ or function key, only when trusted with Actions on. Swift shows the key on the row, in the palette and in the shortcut sheet and binds it nowhere | features/project/actions.rs:56-70; app/handlers.rs:258-275 | WindowUI/Focus/WindowFocus+Keys.swift (the window's key monitor) or key equivalents on PlatformModel/Menus/MenuBuilder+Dynamic.swift:11-12, dispatching `.runProjectAction` | S | yes |
| With the Project feature off (its flag or the person's switch) nothing reads or writes a `.wormhole/`: no overrides, no env or shell, no worktree condition or setup, no notes.md sync, no colour write, and applied overrides are unwound at once. Swift only loses the tab; the FFI says so itself ("does not look at the Project feature switch") | features/project/mod.rs:70-72; features/project/detect.rs:24-45, 131; features/project/notes_sync.rs:46-50; features/project/api.rs:93, 113; features/project/color.rs:27 | ffi/src/files_panel/project/service.rs:163-181, overrides.rs:32, notes_sync.rs:68 (one `enabled` fact from the window's feature mask, checked first) | S | partly |
| Folder colour and "Create project folder" act on the band's folder: the repository's root, else the folder. Swift hands the cwd: in `repo/sub` a colour is stored for `repo/sub` and never tints the band (which names `repo`), and Create makes `repo/sub/.wormhole/` (its caption says so) where GPUI makes `repo/.wormhole/` | features/project/panel.rs:55-56, 104, 175-177; features/project/create.rs:1-3, 16 | PanelUI/Stores/ProjectStore.swift `follow`, `setColor`, `create` (a second, band folder beside the cwd; the project lookup keeps the cwd) | S | yes |
| An action aimed at a named terminal selects that terminal, open already or made, before typing. Swift types into an existing one without bringing it in front | window/feature_terminals.rs:70-78 | Wormhole/Panel/PanelEffects.swift:58 `terminal(for:)` | S | yes |
| "Copy into a new note" shows the Notes tab, the note unfolded with the keyboard in it. Swift appends the note and stays on the Project tab: nothing visibly happens | features/project/notes_sync.rs:142-153; features/notes/api.rs:15-24; features/notes/note_ops.rs:116-132 | PanelUI/Stores/ProjectStore.swift `copyToNote` (show the tab through the host) | S | yes |
| The hub's problems ("Project preferences were reset: …", "Trusted projects were reset, approve them again: …", a failed save) show in the no-project card as well | features/project/panel.rs:223; features/project/hub.rs:55-81 | PanelUI/Project/NoProjectCard.swift:22 (only `store.error` of Create); needs the hub error without a project from ffi/src/files_panel/project/service.rs | S | yes (rare) |
| The head reads `name/<dir>` with the folder's real name (`.andromeda` for an older project) | features/project/panel.rs:248 | PanelUI/Project/ProjectHeadCard.swift:17 (`.wormhole` literal; `ProjectStateRecord` needs the dir name) | S | yes (old projects) |

Checklist misses: `checklist-notes-project.md` compares the tab's cards closely (strings, numbers, the callout's
cases) and its trust rows are right for the panel's own button. It never leaves `panel.rs`, `trust.rs` and
`actions.rs`, so what the feature does outside the tab is absent from it: `detect.rs` (`spawn_options`: env and
shell; `apply_project_settings`: the theme; the unwind when the feature goes off, which it marks "not checked"),
the action key in `app/handlers.rs` (it lists key and menu as one NO; the menu was then built, the key not), and
the window side of the events in `window/feature_terminals.rs`, where its row "Open layout: reuse a terminal with
the same name; cwd; run; focus: YES" is wrong: GPUI skips an open terminal, Swift types into it. It has no row for
whose profile's trust a window reads, and none for the folder being the band's rather than the cwd. The Project
commands window was graded from `s2-panel-b/driven.txt` by "approving there writes trusted-projects.json and a
fresh instance reads status=trusted" with `WORMHOLE_AUTH=allow`, which answers at once: a file changing while the
dialog is up, the one case the "exactly what was shown" rule exists for, was neither read for nor driven, and
`README.txt` lists only "the system's Touch ID prompt itself" as untried.

### P09 Usage tab: plan limit gauges — verdict: gaps

GPUI does:
- Tab offered only with an agent on screen, back to the tab before it when the agent leaves (P01) [WindowUI/Hub/WindowHub+Focus.swift `isShown`, `leaveMissingTab`]
- No reports: one card with the two paragraphs [PanelUI/Usage/UsageView.swift]
- One card per agent that reported, in the order they first did; the latest report of an agent wins; kept across restarts [ffi/src/files_panel/usage.rs `record`, `load`; PanelUI/Stores/UsageStore.swift]
- Fed by Claude Code's status line (`rate_limits`) and the Codex and Gemini hooks over the hook socket [ffi/src/hook_server; Wormhole/Hooks/HookHost.swift]
- Card head: the registry's logo (or monogram) in the muted colour, short name, "Updated just now" / "Updated 3m ago" [PanelUI/Usage/UsageCard.swift `AgentMark`; ffi/src/files_panel/usage.rs `AgentLogoRecord`]
- "Plan usage limits" only with limits; a row per limit: label, "25% · resets 34m", an 8 pt bar in primary, warning from 80 %, danger at the limit; a window that has reset reads "Awaiting update" at 0 %; a limit without a reset time has no "resets" part [UsageCard.swift `LimitRow`; PanelModel/Usage/UsageReport.swift `PlanLimit.shown`, `level` (a copy of core's rule, same clamp, rounding and durations)]
- Gemini: "Latest model response", "N tokens" ungrouped, the caption [UsageCard.swift]
- VoiceOver: each bar "label, words" [UsageCard.swift `LimitRow`]
- The reports are one value every window draws and watches — WRONG
- The words redraw at the second (under a minute) or the minute boundary of each word — PARTIAL
- The reports are the shown profile's, read the first time a profile is shown, dropped when it is deleted — differs (table)

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| One app-wide entity holds the reports and every window's tab repaints when an agent reports. Swift makes a `UsageService` per window (`FilesBridge.makeBackends` in `PanelBackendsFactory.make`, called from `makeSession`), and the hook server is started once with `sessions.first?.panelUsage`: a second window's Usage tab shows what the database held when that window opened and never moves, and after the first window closes the reports go to a service no window reads | features/usage/state.rs:53-81; app/tick.rs:48-49 | Wormhole/PanelBackendsFactory.swift `make` and Wormhole/AppController+Windows.swift `makeSession` (one shared `UsageService` per profile); Wormhole/Hooks/HookHost.swift:27-31 | S | yes (two windows) |
| `next_change` arms a timer for the next word that reads differently: each second in a limit's last minute ("resets 45s"), each report's own minute boundary for "Updated Nm ago", and the reset itself ("Awaiting update", bar to 0). Swift draws from `TimelineView(.everyMinute)`: seconds never count down and each of the three can be up to 59 s late | features/usage/def.rs:42; crates/wormhole-core/src/agents/usage/mod.rs `next_change`; window/clock.rs:65-74 | PanelUI/Usage/UsageView.swift:33 (an explicit schedule from the reports, or core's `next_change` over the FFI) | S | yes (minor) |
| Reports are kept per profile and the windows draw the shown profile's: a profile switch shows that profile's (read from its database once), a deleted profile's go. Swift binds each window's service to the profile that was active when the window was made and follows only "Move Window to profile" (`followProfile`); a report is written to whichever profile the first window's service follows | features/usage/state.rs:26-51, 85-95 | ffi/src/files_panel/usage.rs `UsageService::new`, `follow_profile`; Wormhole/Settings/ProfileChipBinding.swift:125 (falls out of the shared service of the first row) | S | edge (several profiles) |

Checklist misses: section 3 of `s-panel/checklist-files-usage-shell.md` checks one window throughout: it has no row for "every window shows the same reports" and the wave-2 run drove `usage-status` in a single window, so the per-window service was not seen. The per-second refresh it listed (PARTIAL, S) is still as it was and is absent from the row's notes; the per-profile row it marked "not verified" is still unverified in `s2-panel-b`.

### W08 Close and quit questions; Quit and End All — verdict: gaps

GPUI does:
- Native alert sheet on the window, closing button first (Return), Cancel last (Esc), an optional middle button, a checkbox variant [AuxWindows/Questions/QuestionSheet.swift `init`, `ask(in:)`]
- ⌘W / pane ×/ pane menu / exited bar / tab × close through one path that asks only while an agent works or waits or a command runs [Wormhole/AppController+Quit.swift `closeTerminal`; callers ActionRouter+Terminals.swift:17, WindowHub+Panes.swift:14, WindowHub+Sidebar.swift:37, WindowHub+Overlays.swift:12]
- Title `Close “name”?`, detail from who is at stake, the programs (each once, 40 characters, four then "and N more"), "Its shell ends." / "Closing ends the process." / "Closing ends them.", then the notes stake [AuxModel/Questions/CloseQuestions.swift `session`, StillRunning.swift]
- Worktree extra: the note and the "also" button, asked even when nothing runs [AppController+Quit.swift:97-115 `worktreeClose`]
- `wh close` always asks and adds "The command line asked to close it." [AppController+Quit.swift:98, 107; Hooks/HookHost.swift:69]
- A terminal whose host runs but cannot be shown: End the Host / Leave It Running / Cancel [AppController+Quit.swift `askToEndHost`]
- Closing the only terminal opens a fresh one, the window stays [core `workspace/close.rs:43-45`, shared]
- Close group: one question, same parts, "Close Group", worktree extra [AppController+Quit.swift `closeGroup`]
- Close window (⇧⌘W, the red button): "Close this window?" / "Close Window"; the last window quits instead, saved, asked as a quit [Wormhole/AppController+WindowEnding.swift `shouldClose`; AppController+Quit.swift `askToClose`]
- Quit: no question when nothing runs; plain Quit/Cancel, or with the box "Keep terminals running in the background" (ticked each time) when any terminal is hosted; detail by hosted/local counts; other profiles' hidden terminals named [AuxModel/Questions/QuitQuestion.swift, QuitFacts.swift; AuxWindows/Questions/QuitFlow.swift; AppController+Quit.swift `quitFacts`]
- Quit and End All Terminals: its own title and button, no box, menu item only with Persistent terminals [ActionRouter+App.swift:21; PlatformModel/Menus/MenuBuilder.swift:41 `persistentOnly`]
- On the way out the windows are saved and hosted terminals let go of [Wormhole/AppController.swift:127-134 `applicationWillTerminate`; WorkspaceBridge/RustWorkspaceBackend+Hosting.swift `finishQuit`]
- Restart-to-update question reads the same running facts [AppController+Quit.swift `runningTerminals`; Aux/AuxBackends.swift:103] (the question itself belongs to the update row; not audited here)
- The Dock's Quit and logging out do not ask in GPUI; Swift asks there too (`applicationShouldTerminate`). A deliberate difference (QuitFlow.swift:1-3), noted, not a gap
- Which terminals count as running (the rule itself) — WRONG in three ways (below)
- Terminals in a folded group, or hidden by the sidebar's search, are counted and can be closed — WRONG
- Sidebar ×, mouse row menu, palette ⌘⌫ ask — WRONG (first audit's gap 4, still there)
- One question at a time (a second ⌘W or ⌘Q while one is up does nothing) — ABSENT
- The close button's dot while closing would ask — ABSENT
- A window or group whose only stake is a host that cannot be shown asks — PARTIAL
- Quit and End All ends every live host of the data folder — PARTIAL
- SIGTERM quits saved, without the question — ABSENT

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| An agent sitting idle at its prompt still asks: `needs_close_confirmation` is "a program other than the shell is in the foreground, or status Busy/Attention", and the question reads "Still running: claude. Closing ends the process." Swift's `runsSomething` is `activity == working/needsInput` or a second line that is a command; a told terminal's second line is never a command (core `workspace/words.rs:46-52`), so an idle agent (or any program that reports its own status) closes and quits with no question, taking its session with it | window/session.rs:151-154, 199-201; windows.rs:208-212 | Wormhole/AppController+Quit.swift `SidebarRow.runsSomething`, `runningCommand` (needs the foreground program in the snapshot: ffi/src/workspace_types.rs row/terminal record + core `TerminalFacts.command`) | M | yes |
| A plain terminal whose last command failed counts as running: `runningCommand` is "the second line is mono", and the failure line ("cargo build · exit 101") is mono too. Closing such a terminal asks, and the question, the window's, the group's and the quit's list "Still running: cargo build · exit 101" | crates/wormhole-core/src/workspace/words.rs:55-62 (the line); window/session.rs:144-147 (what GPUI lists: the live foreground command) | Wormhole/AppController+Quit.swift `runningCommand` (same fix as above: a real "command in front" fact, not the row's wording) | S | yes |
| The window, group and quit questions list an agent's own program among "Still running: claude, caffeinate" (`running_programs(false)`); Swift lists only plain terminals' commands, so an agent is only ever "An agent is still working." and never named | window/windows.rs:144-149, 466; group_ops.rs:39; quit_question.rs:104-107 | Wormhole/AppController+Quit.swift `quitFacts`, `askToClose`, `closeGroup` (`programs:`) | S | yes |
| Every terminal of the window is looked at. Swift reads `state.sections.flatMap(\.rows)`, and the core lists no rows for a folded group and none that the sidebar's search leaves out (`workspace/snapshot/sidebar.rs:35-36, 129`). So: (a) `quitFacts` and `shouldClose` miss whatever runs in a folded group or outside the search, and the app quits or the window closes with no question; (b) `closeTerminal` finds no row and returns, so ⌘W, a pane's ×, `wh close` on a terminal that is selected or on screen but folded away or filtered out does nothing; (c) `runningTerminals` (restart to update) misses them too. `closeGroup` covers the folded case with `workingCount` but then has no programs to name | window/windows.rs:128-133, 453-457 (`self.sessions`) | Wormhole/AppController+Quit.swift `allRows`, `quitFacts`, `closeTerminal`; AppController+WindowEnding.swift:33 (walk `state.all` with a per-terminal running fact) | M | yes |
| Sidebar ×, VoiceOver "Close terminal", the mouse row menu's Close terminal and the palette's ⌘⌫ go through the question | window/sidebar_host.rs:500; command_palette/mod.rs:561 | SidebarUI/Row/TerminalRowView.swift:33, 68; SidebarUI/Menu/RowMenu.swift:32; Wormhole/Platform/PaletteSourcing.swift:75 (a `close` closure on `SidebarContext`, as `closeGroup` has) | S | yes |
| One question at a time: `closing_session_prompt`, `closing_prompt` and `quit_prompt` drop a second request while a sheet is up. Swift has no such state: each ⌘W starts another `QuestionSheet` task on the same window (AppKit queues it and shows it after the first is answered) and a second Close Window does the same. Not run | window/windows.rs:138-141, 213-216, 443-446; group_ops.rs:34; host_close.rs:24 | Wormhole/AppController+Quit.swift `closeTerminal`, `closeGroup`, `askToClose` (a per-session "asking" flag) | S | yes (edge) |
| The red close button shows the dot (document-edited mark) while closing the window would ask | window/windows.rs:103-109 | WindowUI/WorkspaceWindowController.swift (`window.isDocumentEdited` from the same running fact) | S | yes |
| A host that runs and cannot be shown counts as running for the window's and the group's question too; Swift counts it for quit (`quitFacts`) and the single close, not in `shouldClose` or `closeGroup` | window/session.rs:151-154 | Wormhole/AppController+WindowEnding.swift:33; AppController+Quit.swift:146 | S | yes (edge) |
| Quit and End All (and the box unticked) ends every live session host of the data folder, each given 5 s, off the main thread, after the windows are saved. Swift ends only the terminals this run made (`desk.finish` walks `made`): a host no window holds (another profile's that was never opened, an orphan not adopted, one behind a terminal closed with "Leave It Running") outlives "End All" | window/windows.rs:413-436 (`hosts::end_all_live`) | ffi/src/hosting/desk.rs:259 `finish` (call `hosts::end_all_live(&hosts_dir, 5 s)` when `end_hosts`) | S | yes (edge) |
| `kill` (SIGTERM) quits as ⌘Q does, saved, without the question. Swift installs no handler (no `signal`/`quit_signal` use in Sources): SIGTERM ends the process with nothing saved since the last save and hosted terminals not let go of | main.rs:104; app/sigterm.rs:11-30 | Wormhole/AppController.swift launch (a `DispatchSourceSignal` for SIGTERM calling `terminateUnattended()`) | S | yes (edge) |

Checklist misses: `s-platform/W08.txt` proves the words of five questions through held answers (a plain `sleep 30`, `wh close`, quit with a live host, End All). It never tried an idle agent, a failed last command, a folded group, a search in the sidebar, a second ⌘W while a question is up, or a close from the row's ×: every case above passes its transcript. Its fix "a plain terminal is no longer called an agent" is right, but the same change is what stopped an idle agent from counting at all. The tracker's note says the sheets, the Dock's Quit and logging out need a person; it does not say that the running rule itself is derived from the row's second line.

### O02 Shortcut sheet — verdict: gaps

GPUI does:
- ⌥⌘/, Help ▸ Keyboard Shortcuts and the palette's action toggle the sheet over the front window [Wormhole/Actions/ActionRouter+View.swift:31 `keyboardShortcuts` → `.showOverlay(.shortcuts)`; WindowUI/WindowBody.swift:137]
- One overlay at a time: opening it takes the palette down, and the reverse [core `Action::ShowOverlay`, one `overlay` field]
- Sections and rows from core `keymap::sheet`, rows of a hidden action left out, a run with a gap written as single keys and its label naming only what is shown [ffi/src/keymap/lists.rs:77 `keymap_sheet` → `sheet::sections_without`; hidden list from `PaletteSourcing.actionFacts(...).hidden`]
- Keys are the action table's (the one recordable key, `jump_waiting_key`, shows its default on both sides: same core call)
- Filter by label or key, a modifier glyph matching the key exactly, the match marked in the label [ffi `keymap_sheet`, `keymap_sheet_matches`; PaletteUI/ShortcutLine.swift]
- Up to three columns by width, fewer when the filter leaves fewer sections, the sheet shrinking to them (min 520) [PaletteUI/SheetLayout.swift `init`, `columnsFitting`; ffi `keymap_sheet_columns`]
- Body scrolls inside the card, card capped to the window's height [PaletteUI/SheetBody.swift `ScrollView`, SheetLayout `bodyRoom`]
- "No shortcut matches" when nothing is left [SheetBody.swift:15]
- Trusted project's keys block under the sections, filtered and counted in the footer [PaletteModel/ProjectKeys.swift; PaletteUI/ProjectKeysBlock.swift]
- Footer: "N of M · matches labels and keys (“⌘W” finds Close terminal)" while filtering; else the hotkey line (with the set key, or where to set it) and "Also in Help › Keyboard Shortcuts and the palette" [PaletteUI/SheetFooter.swift]
- Esc clears the filter, then closes; a click outside the card closes [ShortcutSheetView.swift:46-49; OverlayChrome.swift:48]
- The filter has the keyboard when the sheet opens [PaletteField.swift `FocusingField.viewDidMoveToWindow`]
- Rows are not clickable on either side (nothing to match)
- VoiceOver name "Keyboard shortcuts" [ShortcutSheetView.swift:60]
- Closing gives the keyboard back to what had it before the first overlay opened (a terminal closed meanwhile: the current one) — ABSENT in the source
- Does not open while the first-run page is up — ABSENT
- Esc closes whatever in the window has the keyboard (the key is intercepted for the whole window) — PARTIAL

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Focus returns where it was when the sheet (or the palette) closes: `end_overlay` records the focused handle and the terminal it was, `close_overlay` refocuses it, or the current terminal if that one is gone. In Swift nothing does this: `.showOverlay(nil)` only clears the core's field, the field's view leaves the window, and no caller of `makeFirstResponder`/`focusTerminal` hears it (searched `showOverlay`, `state.overlay`, `makeFirstResponder`, `focusTerminal` across Sources; `WindowFocus` only reads `overlay != nil` in `escape()`). AppKit then leaves the window as first responder, so typing goes nowhere until a click. Not run; the first audit's O01 line credits `OverlayChrome` with this, which holds no such code | window/overlay.rs:138-151, 166-173; shortcut_sheet.rs:82-85 | WindowUI/WindowBody.swift `overlay` (remember `window.firstResponder` when `state.overlay` becomes non-nil, restore on nil) or WindowUI/Focus/WindowFocus.swift (a `returnFocus` pair) | S | yes |
| No sheet while the first-run page is open over the window | shortcut_sheet.rs:65-66 | Wormhole/Actions/ActionRouter+View.swift `keyboardShortcuts` (guard on the onboarding state) | S | yes (edge) |
| Esc is taken for the window, ahead of the field, so it closes the sheet whatever has the keyboard. Swift hears Esc only as the filter field's `cancelOperation`; if the field is not first responder (after the focus gap above, or VoiceOver moved off it) Esc does nothing | shortcut_sheet.rs:114-122 | PaletteUI/ShortcutSheetView.swift (an `.onExitCommand` or key monitor on the chrome) | S | yes (edge) |

Checklist misses: the lane's evidence is three `shortcut-rows` dumps (rows, a key filter, an empty result) and paired screenshots. None of it presses Esc, closes the sheet and types, or opens it over the palette, so the focus return was never exercised: `r-verify/O02.txt` and `s-overlays/O02.txt` read the model, not the window. `r-verify/O02.txt` is also a dump of the older hand-written list (9 rows) and no longer describes the code.

### O04 Context menus: row; group; path; panel items; terminal — verdict: gaps

Every place the GPUI app opens a context menu (right-click handlers and `platform::on_right_click`, found by
`MouseButton::Right` and `platform::show_menu`): sidebar row, group head, the group switcher's cards, the + button
(shells), pane header, title row, folder crumb, terminal grid, Changes file and folder rows, History commit row,
commit details block, a commit's file rows, Files rows, Settings rows and theme cards. GPUI has none on Notes rows,
tab-bar tabs, the status bar, the panel's tab bar or the sidebar's empty area; neither has Swift.

GPUI does:
- Terminal grid: Open Link / Copy Link over a link; Copy, Paste, Select All; Find…; the block's Copy Command, Copy Output, Put Command at Prompt, Fold Output; Split Right / Down; Clear Buffer; keys and icons; right-click or ⌃-click, the program keeps the click unless ⇧ [PlatformModel/Context/TerminalMenu.swift `nodes`; Wormhole/AppController+Context.swift `terminalMenu`, `runContext`; TerminalMetal/Pointer/TerminalMenu.swift, TerminalPointer.swift:30] — complete, one enablement difference (table)
- Sidebar row: 11 to 13 items in GPUI's order with checks [SidebarUI/Menu/RowMenu.swift; keyboard copy PlatformModel/Context/RowMenu.swift via Wormhole/Menus/RowMenuKeyboard.swift] — four differences (table)
- ⌃↩ on a sidebar row opens its menu under the row [SidebarUI/Keys/SidebarView+Keys.swift:20-23 → `openRowMenu`]
- Group head: Rename…, colours, folder items, the five actions [SidebarUI/Menu/GroupMenu.swift] — differences (table)
- Pane header and its ⋯: Split right, Split down, Zoom / Unzoom pane, Move out of split, Close pane, greyed by room and split, the pane focused first, Close asking [PanesUI/Panes/PaneMenuItems.swift; PaneHeader.swift:43] — complete (keys are written ⌘D, ⇧⌘D, ⇧⌘↩, ⌘W in the view, not read from the keymap)
- Folder crumb: the path as heading, Open in Finder, Copy path, Insert into terminal (off when exited), the folders down to the volume with Finder's icons [PanesUI/Title/PathMenuItems.swift; FolderCrumb.swift:40] — complete
- Title row elsewhere: "Show in title row", each hideable part checked, Reset title row, Customize title row… [PanesUI/Title/TitleRowMenu.swift; TitleRowView.swift:41] — two differences (table)
- Changes file row, folder row, a commit's file: Open in <editor> (not with Finder chosen), Reveal in Finder, Copy Path(s), Copy Relative Path(s), File History; several ⌘-marked files share one menu [PanelUI/Changes/PathMenu.swift; FileRowView.swift:64, 111] — one difference (table)
- History commit row: the five copies [PanelUI/History/CommitMenu.swift; HistoryView.swift:82] — complete
- Files row: Open in ▸, Reveal in Finder, Copy Path, Rename…, Pin / Unpin Folder [PanelUI/Files/FileRowMenu.swift; FileEntryRow.swift:32] — the tracker's "no Open in ▸" is out of date: it is there; one difference (table)
- Files path button menu, Changes view-options menu: button menus, present [PanelUI/Files/FilesPathButton.swift; PanelUI/Changes/ViewOptionsMenu.swift] (not compared item by item: not context menus)
- Group head ⌃↩, title strip ⌃↩, ↩ on a commit's details — ABSENT
- Commit details block's own menu (Show more/less, Open Parent) — PARTIAL
- Group switcher card menu, + button's "New terminal with" menu, Settings row menu — ABSENT

`PlatformModel/Context/GroupMenu.swift`, `PanelMenus.swift` and `PathMenu.swift` build the group, panel and path
menus as native nodes (the group one with the swatch row) and have no caller outside PlatformModel and its tests:
the app's menus for those are the SwiftUI ones above. They count as absent, and are the ready-made fix for the
group head's dots.

| Menu | Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|---|
| Sidebar row | Close terminal asks first (mouse menu). The ⌃↩ copy of the same menu asks | sidebar/row_menu.rs:187; window/sidebar_host.rs:500 | SidebarUI/Menu/RowMenu.swift:32 | S | yes |
| Sidebar row | "Add terminal to group ▸": a group that keeps only its folder's terminals is greyed for a terminal outside it (`allowed`). Swift offers every group in both copies of the menu; the core refuses silently, so the item does nothing | sidebar/row_menu.rs:28-33, 159; group_head.rs:212 | SidebarUI/Menu/RowMenu.swift:23-25; Wormhole/Menus/RowMenuKeyboard.swift:23 (`RowMenuFacts.groups` has no `allowed`); needs the fact in the group snapshot | S | yes |
| Sidebar row | "Show beside… ▸" lists only terminals the panes have room for (`can_join` against the pane room); Swift lists every terminal not in the same split, and a pick without room is refused silently | window/sidebar_drops.rs:106-120 | SidebarUI/Menu/RowMenu.swift:56; RowMenuKeyboard.swift:24 | S | yes (edge) |
| Sidebar row | Copy path posts the "Copied" notice | sidebar/row_menu.rs:217-222 | SidebarUI/Menu/RowMenu.swift:46-50; RowMenuKeyboard.swift `.copyPath` | S | yes |
| Group head | The nine colour dots are a row in the menu; Swift has a "Colour ▸" submenu of names (known to the lane). `SwatchMenuView` and `PlatformModel.GroupMenu.nodes` exist and are unused | sidebar/group_popover.rs:224 | SidebarUI/Group/GroupHeadView.swift:60 (present the native menu instead of `.contextMenu`) | M | yes |
| Group head | Rename… renames in place in the head; Swift opens the group's popover. Other colour… opens the system colour picker; Swift opens the popover | sidebar/group_popover.rs:220-229 | SidebarUI/Menu/GroupMenu.swift:15, 23 | S | yes |
| Group head | ⌃↩ with the keys on a group's head opens its menu under it. Swift's ⌃↩ handles `.row` stops only | sidebar/row_menu.rs:73-79; group_head.rs:147-157 | SidebarUI/Keys/SidebarView+Keys.swift:20 (a `.group` case) + a native group menu | S (after the native menu) | yes |
| Group switcher card | Right-click on a group's card in the tab bar's switcher opens the group's menu | sidebar/switcher.rs:432-438 | PanesUI/Tabs/GroupSwitcherView.swift (no `contextMenu`) | S | yes |
| + button | Right-click on New terminal (the sidebar's and the tab bar's) opens "New terminal with": the shells, the default checked, a pick opens a terminal with it. No such menu or string in Swift (the shells are reachable from the menu bar and the palette only) | sidebar/shells.rs:22-55; window/chrome.rs:177; sidebar/tab_bar.rs:373 | WindowUI (the title row's new-terminal button) and PanesUI/Tabs/TabBarView.swift; shells from `settingsModel.host.shellChoices()` | S | yes |
| Title strip | ⌃↩ with the keys on the title strip opens the folder's path menu, or the row's own menu when the row shows no folder; in the panel's lists ⌃↩ is swallowed so it never reaches the terminal. Swift's `WindowFocus.key` has no ⌃↩ | window/focus/mod.rs:283-284; focus/menu.rs:17-29 | WindowUI/Focus/WindowFocus+Keys.swift `key(_:)` | S | yes |
| Title row | Parts, Reset and Customize are greyed when the window cannot edit them (Zen mode, a window of a profile that is not the shown one). Swift leaves them enabled; in Zen `setPart`/`resetRow` silently do nothing, and no profile check exists | window/title/parts.rs:60-67, 78-98 | PanesUI/Title/TitleRowMenu.swift (an `editable` fact); WindowUI/Hub/WindowHub+Title.swift:28-30 | S | yes (edge) |
| Title row | Customize title row… opens Settings on the title-row setting (`RevealSetting`); Swift calls `showSettings()` with nothing to reveal | window/title/parts.rs:94-98 | WindowUI/Hub/WindowHub+Title.swift:30 (`settingsModel.reveal(id)` as the palette does) | S | yes |
| Terminal | Split Right / Split Down are greyed when the action is not available (no room, a diff over the terminal); Swift sets both from `router.table[...] != nil`, which is always true | terminal_view/context_menu.rs:251-252 | Wormhole/AppController+Context.swift:16-17 (use `MenuState.disabled`) | S | yes (edge) |
| Changes / commit file | "File History" is offered only when History can show it (`history: Option`); Swift always appends the separator and the item | features/git/path_menu.rs:60-67; changes/view.rs:275 | PanelUI/Changes/PathMenu.swift:37-38 | S | yes (edge) |
| Commit details | The details block's menu is "Show more"/"Show less" (when offered), the five copies, then "Open Parent <short>" for each parent the log lists. Swift's block has the copies only (Show more and the parents exist as controls in the block, not in the menu) | features/git/history/menu.rs:33-64, 68-84; details_view.rs:54-57 | PanelUI/History/CommitDetailsView.swift:43 | S | yes |
| Commit details | ↩ with the keys on the details row opens that menu under the row (the keyboard's way to the copies and the parents) | features/git/history/keys.rs:62-64; menu.rs:98-111 | PanelUI/History (no key path found; a native menu as the row's ⌃↩ has) | S | yes |
| Files row | Open in ▸ offers a file only the editors Wormhole knows (and Finder), a folder every app; apps carry their icons. Swift lists every app for both, without icons | features/files/open.rs:50-54; menus.rs:55-62 | PanelUI/Files/FileRowMenu.swift:31-37 | S | yes (edge) |
| Settings row | Right-click on a settings row: Copy Key, Copy as JSON, Reset to Default (off when at default). No such menu in SettingsUI (its only context menu is the theme cards') | style/row_keys.rs:208, 234-252 | SettingsUI/Rows (row wrapper) | S | yes |

Checklist misses: `O04-menus.txt` compares five menus by title and order with one fixture each, and calls row, terminal, path and title "identical". It did not try a folder-only group or a full split (enablement), Zen mode or another profile's window (title row), a diff or a full window (terminal splits), or what Close terminal does once picked: `O04-rowmenu.txt` picks every item through the keyboard menu, which is the one copy that asks, so the mouse menu's direct close passed unseen. It lists the group dots and (now fixed) Open in ▸, and says Changes and History were "not read": the details block's menu, File History's gate and the ↩ menu are what reading them finds. Not in any evidence: the group switcher's cards, the + button's shell menu, Settings rows, and the keyboard's menu key anywhere but a sidebar row (group head, title strip, commit details). The tracker's note "no Ctrl-Return keyboard menu" is half out of date (rows have it).


---

# All gaps, ranked

Most user-visible and largest first. Gaps that several rows share are merged and name every row. "Visible" is yes
unless the line says otherwise. The GPUI file:line and the Swift place of each are in its row's table above.

| # | Gap | Rows | Size |
|---|---|---|---|
| 1 | SECURITY: the Project commands window can trust content that was never shown: `TrustBridge.approve` re-reads the disk, and the window's watch loop calls `review()` during the Touch ID dialog, replacing what `approve_reviewed` trusts. GPUI clones the request before the dialog. (The panel's own Trust button is right.) | P08 | S |
| 2 | An idle agent closes and quits with no question: the running rule is taken from the sidebar row's wording, where GPUI asks whenever a program other than the shell is in the foreground. The same rule makes a terminal whose last command failed count as running ("Still running: cargo build · exit 101") | W08 | S |
| 3 | The close, window and quit questions see only the sidebar's listed rows: whatever runs in a folded group or outside the sidebar's search is not asked about, and ⌘W, the pane's × and `wh close` on such a terminal do nothing. Closing a folded group never asks about its worktrees | W08, P05 | S |
| 4 | Closing without the question (first audit's gap 4, still in the source): the row's ×, VoiceOver's Close terminal, the mouse row menu's Close terminal and the palette's ⌘⌫ send `.close` straight to the core; the ⌃↩ copy of the same menu asks | W08, O04 | S |
| 5 | Review comments are the window's, not the terminal's: written beside terminal A they stay under B and "Paste into Terminal" sends them to whichever terminal is selected | P03 | M (with 6) |
| 6 | Changes, diff, History and Files state is one store per window: an open diff or file stays over another terminal in the same folder; compare branch, folds, marks, picked rev, path filter, open commit, open folders and a hand-chosen folder are shared or reset where GPUI keeps them per terminal | P02, P03, P04, P07 | M |
| 7 | A notes.md merge into the terminal on show is never saved (`saved = merged` set before the model): quit before the next edit and the next launch reads the stale database as a deletion and removes the lines from notes.md. Data loss | P06 | S |
| 8 | SECURITY: each window's project service uses the active profile's trust store and switches, not the window's: a window of another profile runs under the wrong approvals | P08 | S |
| 9 | "Open terminals" (a project layout) types `run` + Return again into terminals that are already open, matched by shown title; GPUI skips them | P08 | S |
| 10 | A trusted project's `env` and `shell` are not given to new or restored terminals; the Settings row and the worktree refusal say they are | P08 | M |
| 11 | Worktree settings do nothing: `worktree_base` and `worktree_clone_build_folders` never reach the service (`setOptions` has no caller), so the sheet always plans beside the repository and never clones build folders | P05 | S |
| 12 | Worktree ports (`WORMHOLE_PORT`, `WORMHOLE_PORTS`) reach no terminal: the environment is dropped at creation and the FFI's `spawn_options` names no port (checked by hand: nothing in `ffi/src/terminal.rs`), although the comment in `PanelBackendsFactory.swift:53-55` says it does | P05 | M |
| 13 | The Files tree, the folder colour and "Create project folder" act on the cwd, not the repository root: in `repo/sub` the tree shows `sub`, the colour never tints the band and Create makes `repo/sub/.wormhole/` | P07, P08 | S |
| 14 | A CRLF file shows as one line cut at 5,000 characters (Swift's `"\r\n"` is one Character, never split); an LF file gets one extra numbered line. From Swift semantics, not run | P07 | S |
| 15 | "File History" from the Changes menu shows History's body under a strip that still marks Changes, and clicking Changes then does nothing (only `controller.tab` is set). The item also shows with History off | P02, P04, O04 | S |
| 16 | Usage updates reach the first window only (one `UsageService` per window, the hook server fed `sessions.first`); reports are bound to the profile active when the window was made | P09 | S |
| 17 | ⇧⌘E from the terminal never reaches the notes (GPUI: the working note, else the first, else a new one, opening the tab and the panel) | P06 | S |
| 18 | No keyboard comes back when an overlay closes (sheet and palette): nothing refocuses the terminal or what had the keys; Esc closes the sheet only while its field is first responder. Not run | O02, O01 | S |
| 19 | Quit and End All ends only the hosts this run made, not every live host of the data folder (`end_all_live`) | W08 | S |
| 20 | SIGTERM: no handler, so `kill` ends the app with nothing saved and hosted terminals not let go of | W08 | S |
| 21 | No one-question-at-a-time guard (each ⌘W queues another sheet); no dot in the red close button while closing would ask; an unavailable host is not counted for the window's and group's question; agents' programs are never named in "Still running" | W08 | S×4 |
| 22 | Actions of a tab that is not offered stay live: the menu hides by `flag`, never by the `feature` tag (`keymapHiddenActions` has no caller); ⌃⌘4 without an agent shows Changes; `newNote` adds a note and the notes sync runs with the Notes feature off; with Project off the FFI still reads `.wormhole/` | P01, P06, P08 | S+S |
| 23 | Project action keys (`actions.json` `key`) are shown on the row, in the palette and in the sheet and bound nowhere | P08 | S |
| 24 | The project's theme (`theme.json`) is never applied; the Theme switch is shown | P08 | M |
| 25 | Open in has no resolve-within-root and no kind check before handing a path to an app, and falls back to the default app (security: a folder swapped for a link); pictures are decoded with `NSImage(data:)` in the view body with no size cap (up to 50 MiB of repository bytes) | P07 | S+S |
| 26 | Pins, worktree records and the `gh` approval read the Default profile whatever the window's; pins are not re-read after a rename | P07, P05, P04 | S×3 |
| 27 | Copies post no notice: History's five copies (and the cut-message and failed-read notices), Copy Path, Copy Relative Path, Copy error, the row menu's Copy path | P02, P04, O04 | S |
| 28 | History's details: no menu with Show more, the copies and Open Parent; ↩ on the details row does nothing, so the keyboard reaches none of them; following a parent does not scroll to it | P04, O04 | M |
| 29 | A commit picked in the palette opens only if the loaded log already lists it (GPUI clears the filter, goes back to HEAD when needed and reads it) | P04 | S |
| 30 | Esc in the rev picker runs `pick` (folds the open commit, closes its diff, reads again) where closing changes nothing; no "Loading branches…" hint or HEAD wording | P04, P02 | S |
| 31 | The diff's ‹ › stop at the ends where GPUI wraps; "File N of M" only when a split fits; no commit hash after it from History | P03 | S |
| 32 | The submodule diff is one text chip: no hashes line, sections with totals, commit rows with author and age, "and N more"; no header badge | P03 | M |
| 33 | A diff compared with a branch is not re-read when the base moves or the file is edited (stamp nil while comparing) | P03 | S |
| 34 | Syntax colours are a per-line lexer, not tree-sitter over the whole file (multi-line strings and comments, languages the lexer lacks); known to the lane | P03, P07 | L |
| 35 | Stash prompt takes the keyboard away from the terminal and ignores the terminal's selection; Send to terminal leaves the keys in the note and submits with no delay after the paste; the draft step lacks ⌘↩, ⌫ and its actions | P06 | S×3 |
| 36 | The notes sync's "being written" test is "any NSTextView is first responder in the key window": any field holds the merge back, and with another app in front a note being written is merged over | P06 | S |
| 37 | Worktree command line: `--command` refused, `asked: true` where GPUI refuses (untrusted, dialog already up); the setup line is typed raw into whichever terminal is current after two awaits; trust is asked of the terminal's folder, not the repository | P05 | S×4 |
| 38 | PR checks that finish post no notification (the FFI drops `PrCache::finish`'s result) | P04 | S |
| 39 | Context menus missing outright: ⌃↩ on a group head and on the title strip, the group switcher card's menu, right-click on + ("New terminal with" shells), the Settings row's menu (Copy Key, Copy as JSON, Reset to Default) | O04 | S each |
| 40 | Context menu enablement: groups that keep only their folder's terminals are offered to any terminal, "Show beside" lists terminals there is no room for, the title row's Parts/Reset/Customize stay enabled in Zen and in another profile's window, the terminal's Split Right/Down are always enabled; Customize title row does not reveal the setting; group Rename… and Other colour… open the popover; the colour dots are a submenu of names | O04 | S each |
| 41 | The Files cover follows the panel's tab (switching to Notes hides the open file; a diff and a file can both be kept) where GPUI has one cover per window; the file's crumb is text, not a button; line numbers scroll sideways with the code | P07 | M+S+M |
| 42 | Files, small: icons ignore the chosen icon theme; Open in button hidden with Finder as the app (also the Changes and diff open icons); rename refusals lack the catalog's sentences; `/` named "Macintosh HD"; a non-UTF-8 name answers "gone"; the tree is read with the tab hidden or the feature off | P07, P02, P03 | S each, M icons |
| 43 | Closing the panel, the diff or the file viewer does not give the terminal the keyboard | P01, P03, P07 | S |
| 44 | Zen mode: show-tab sets the panel wanted in core, so leaving Zen opens a panel that was closed; a stale Usage tab is left only while the panel is on screen | P01 | S+S |
| 45 | Time words: History's ages redraw only when the body happens to; Usage counts no seconds in a limit's last minute and can be 59 s late | P04, P09 | S |
| 46 | Project, small: an action aimed at a named terminal does not bring it in front; "Copy into a new note" shows nothing; the hub's problems are missing from the no-project card; the head says `.wormhole` for an `.andromeda` project; a trust refusal at Create closes the worktree sheet | P08, P05 | S each |
| 47 | Changes, small: ⌘-click marks a submodule row; a nested repository ("dir/") is drawn as a file; showing the tab does not read the status again; "Renamed from" missing when only the folder changed; History's ref chips and parents do not wrap; the open icon on files since removed | P02, P03, P04 | S each |
| 48 | The sheet opens over the first-run page | O02 | S |
| 49 | An open note's preview counts source lines, not wrapped lines (known to the lane); small note rules kept in Swift instead of core's | P06 | M |
| 50 | Tooltips lack the keys; VoiceOver's tab role, position and words; the diff's rows are rebuilt in the view body on every sideways scroll step (read, not measured) | P01, P03 | S; VoiceOver and speed only |
