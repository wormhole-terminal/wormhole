Round S, lane L (right panel): what was checked, how, and what is left.

Method: throwaway repos (scratchpad/s-panel/fx/repo: staged, unstaged, untracked, renamed, deleted, binary, mode-change,
submodule changes, bidi/control/long-line files, 162 commits, tags, a stash, a worktree; fx/proj: a .wormhole project),
a GPUI devctl instance (debug build, `scripts/devctl-instance.sh`) and a Swift --background instance driven by `control`
commands (`window-shot`, `diff N`, `history N`, `tab`, `files-open|files-expand|files-rename-start`, `panel-option`,
`usage-status`, `project-state`). Captures: scratchpad/s-panel/cap/<dark|light>-<state>.png (Swift) and
cap/g-<dark|light>-<state>.png (GPUI), pairs in cap/pair-*.png and scratchpad/s-panel/proj2.png, hist4.png, g-tabs4.png.
Colours differ between a GPUI shot and a Swift window-shot only by the window's blur/vibrancy (GPUI shots have none).

Checklists written from the GPUI sources, one row per state, with the Swift status: checklist-git-tabs.md (Changes, Diff,
History, Worktrees), checklist-notes-project.md (Notes, Project), checklist-files-usage-shell.md (panel shell, Files, Usage).

Fixed in this lane (all compared in dark and light where a shot exists):
  P01  Usage tab only with an agent on screen (leaves to the tab before it), Project only with a folder, GPUI glyphs for
       Project and Files, only Changes counts, stash button in the strip while Notes is up.
  P02  failed refresh keeps the list under a red line, deleted compare branch line, git problem cards (git missing,
       dubious owner, failure) with Retry and Copy error, GPUI wording for loading / not a repository, History label HEAD.
  P03  header as the title row's path crumb with the file icon, controls only over lines (binary, LFS, submodule keep the
       x), unwrapped diffs scroll sideways (they drew nothing before), control and bidi characters spelled out, lines cut
       at 5,000 characters, LFS / too-large / submodule notes from core instead of a failure line.
  P05  worktrees allowed in a folder without a project (was refused everywhere), `newTerminalInWorktree` still has no handler.
  P06  new note opens for writing (+ and the newNote action), esc keeps and gives the keys back, command-return sends the
       selection or the paragraph (core's rule over the FFI), return opens the next step below, backspace on an empty row
       deletes it and opens the one above, blank notes are thrown away, GPUI placeholder, done steps muted.
  P07  viewer header in the title row with the path crumb, binary/unavailable panels with icon, Open in and Reveal,
       placeholders and the line cap, the unwrapped viewer starts top left, sizes worded by core (`3 KB`).
  P08  flat callout with its icon and indent, Folder colour folds, the "Notes sync: waits for the project to be trusted" line,
       project actions run from the menu.
  Review paste copies instead of pasting when the program has no bracketed paste (always pasted before).

Inputs for the settings lane: `PanelController.apply(PanelViewOptions)` (changes_tree, group_untracked, diff_split, diff_wrap,
tab_width, code font) and `PanelController.onOptionsChanged` (what a control in the panel changed); `control panel-option`
drives it. The settings lane's `PanelPreferences` (exp/swift 491c854a3) is the same idea: reconcile on merge, keep one.

Not exercised from a script (a person must try): every pointer path (row clicks, drags to reorder, the tab strip, the diff's
line comment popover, context menus), typing into the note and step editors with the keyboard (command-return, esc, backspace,
return: covered by NotesStore/TaskList tests on the same entry points), ctrl-command number keys, VoiceOver.

Left (see the checklists for file:line): list keyboard for Notes, Changes and History (arrows, return, space, tab stops,
cursor ring); per-terminal Files state and the path head as a stop; the Open in submenu and "Open in <editor>"
(`editorName` is nil); load-anyway for too-large diffs; per-column notes (New file / Deleted / Empty file) and the mode and
"No highlighting" badges; "View all N" as a window; the row menu for folders and submodules; multi-select; Usage brand logos
and the 8 pt pill bar; the viewer's fixed gutter; worktree close question and ports; tooltips with keys; extra motion that
GPUI does not have (bouncy checkbox, row slides) is kept and listed as a gain.
