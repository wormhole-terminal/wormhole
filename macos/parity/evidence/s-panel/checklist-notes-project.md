# Right panel: Notes and Project tabs, GPUI vs Swift

Read-only comparison. Nothing was built or run; every Swift claim below comes from reading the file named.

Path shorthand:
- `G/` = `crates/wormhole/src/features/`
- `C/` = `crates/wormhole-core/src/`
- `Gs/` = `crates/wormhole/src/style/`
- `S/` = `macos/swift/Sources/`
- `F/` = `crates/wormhole-ffi/src/files_panel/project/` (the Rust side Swift links)

Status: YES = same behaviour, PARTIAL = present but different or incomplete, NO = absent.

---------------------------------------------------------------------------------------------------

## 1. Notes tab (Next steps card + Notes card)

### 1a. Structure, counts, folds

| State / interaction | GPUI file:line | Swift | Swift file or "missing" | Note |
|---|---|---|---|---|
| Two cards: Next steps then Notes, one virtual list, 16 pt spacer at the end | `C/view/notes/rows.rs:56-88`, `G/notes/view.rs:67` | YES | `S/PanelUI/Notes/NotesView.swift`, `StepsCard.swift`, `NotesCard.swift`; tail `PanelMetrics.swift:28` | Swift is a plain ScrollView (not virtual); fine until very long lists |
| Next steps band: title, open-count badge (hidden at 0), "+" "Add next step" | `G/notes/task_rows.rs:22-53` | YES | `StepsCard.swift:23-27` | |
| Notes band: title, count of notes with text, "+" tooltip "New note" + key | `G/notes/view.rs:71-101` (tooltip with key `:32`) | PARTIAL | `NotesCard.swift:20-24` | Tooltip has no key name (GPUI `keymap::with_key("New note", NewNote)` = "⌥⌘N") |
| Count of filled notes = notes that are not whitespace-only | `G/notes/mod.rs:198-200` (`chunks_blank`) | PARTIAL | `PanelModel/Notes/NotesModel.swift:41` | Swift counts `!text.isEmpty`; a note of only spaces/newlines is counted. (FilesBridge `NotesMapping.swift:33-35` does trim, for the sync only) |
| Section with nothing inside has no chevron and is never left folded | `G/notes/note_ops.rs:198,260`; `C/.../rows.rs:49-51` | PARTIAL | `StepsCard.swift:24` (steps: YES); `NotesCard.swift:21` | Notes: chevron hidden when empty (YES) but `notesCollapsed` is not reset when the last note is deleted, so a later added note by other means stays hidden. "+" resets it (`:24`) |
| Fold Next steps / Notes by band click | `view.rs:90`, `task_rows.rs:42` | YES | `CardBand.swift:36-50`, `NotesCard.swift:23`, `StepsCard.swift:26` | |
| Fold state saved per terminal | `G/notes/mod.rs:158-159` | YES | `FilesBridge/Notes/NotesMapping.swift:15-16,19-24` | |
| Band VoiceOver label | `view.rs:99` `spoken::section_band("Notes", n, Notes)` -> "Notes, 2 notes" / "Notes, no notes"; steps "Next steps, 2 open" / "Next steps, none open" (`C/view/spoken.rs:131-141`) | PARTIAL | `NotesCard.swift:23` ("Notes, N written"), `StepsCard.swift:25` ("Next steps, 0 open") | Wording differs; 0 case differs |
| Panel tab's spoken label "Notes, 2 open next steps, 1 note" | `view.rs:345-352`, `C/view/spoken.rs:90-99` | NO | `S/WindowUI/PanelTabBar.swift:30-33` | Tab button says only "2 of 6" (+ badge) |
| Empty note list / empty tasks | both bands stay; empty section has no chevron; no placeholder text | YES | `StepsCard.swift:24`, `NotesCard.swift:21` | GPUI has no "empty" copy either |
| Row count on the sidebar row = filled notes + open steps | `G/notes/def.rs:52-60`, `mod.rs:119-121` | YES | `S/Wormhole/Panel/NotesCoordinator.swift:71` (`setNotes`) | Counts via Rust `setNotes`; not re-verified |
| Close question mentions "notes and tasks are discarded" | `G/notes/def.rs:72-76`, `C/view/notes/stake.rs:6` | NO | `S/Wormhole/AppController+Quit.swift:74,114,128` pass `stakes: []` | The Swift close question never lists them |
| Feature off (`notes_and_tasks`): no tab, no keys, data kept | `G/notes/mod.rs:54-59` | not checked | `S/WindowUI/Hub/WindowHub+Focus.swift:7` (`panelTabs` from the core layout) | Tab list comes from core; not verified for this flag |

### 1b. Next steps (tasks)

| State / interaction | GPUI file:line | Swift | Swift file or "missing" | Note |
|---|---|---|---|---|
| Row: checkbox in 16 pt lead column, text, hover actions Send (arrow-up-right) + Delete (x), min height 28 | `Gs/section.rs:84-153` (`ROW_H` 28 `scale.rs:26`, radius 8 `scale.rs:52`) | YES | `TaskRowView.swift:24-34`, `RowActions.swift`, `BandButton.swift` | Metrics match (28, 8, gap 8, inset 8) |
| Checkbox 14 pt, mark 10 pt, chip radius, tokens `checkbox.*` | `Gs/checkbox.rs:19-26` | YES | `TaskCheckbox.swift:13-23`, `PanelMetrics.swift:30-31` | |
| Checkbox motion | none (no transition in `checkbox.rs`) | WRONG (extra) | `TaskCheckbox.swift:17,19,25`; `PanelMotion.swift:14` | Swift pops the fill with a bouncy spring and draws the tick; GPUI switches instantly. Remove or confirm as a deliberate gain |
| Done step colour | `Gs/section.rs:158-159` `item_label(..., quiet=done)` -> `ui.muted` (`text.muted`) + line-through | WRONG | `TaskRowView.swift:49-50` uses `textFaint` for text and for the strike | Should be `textMuted` (`DesignSystem/Theme/Tokens1.swift:49`) |
| Tick with checkbox click; Space on a focused row | `task_rows.rs:101-105`; `list.rs:88` | PARTIAL | `TaskRowView.swift:25` (click YES) | Space on a walked row: no row cursor in Swift (see 1e) |
| Click text edits in place, cursor at end | `task_rows.rs:119-127`, `tasks.rs:55-114` | PARTIAL | `TaskRowView.swift:54`, `:39-47` | Cursor placement at end not set explicitly |
| Task editor placeholder "Next step" | `tasks.rs:83` | PARTIAL | `TaskRowView.swift:41` (empty placeholder) | Draft row uses "New next step" (`TaskDraftRow.swift:17`), GPUI "Next step" |
| Enter in a task: keep it and open a new empty step right below ("checklist" flow) | `tasks.rs:136-150` | PARTIAL | `TaskDraftRow.swift:19-25` (draft only, appends at end) | In an existing step Enter only finishes (`TaskRowView.swift:43`); GPUI inserts the new draft after that step, not at the end |
| Enter in an empty editor: done writing, focus to terminal | `tasks.rs:147` | NO | missing | Swift: empty submit does nothing (`TaskList.add` ignores it, `TaskDraftRow.swift:22`) |
| Esc in a task editor: keeps the typed text (blur finishes), keyboard goes to the terminal | `C/view/notes/keys.rs:25-27`, `tasks.rs:166` | WRONG | `TaskRowView.swift:44,58-62` (Esc drops the change), `TaskDraftRow.swift:25` (Esc discards non-empty draft) | GPUI: Esc = `ItemKey::Leave` = `FocusTerminal`; blur then saves. Swift loses the text and keeps focus |
| Blur with text: kept (edited step saved, new draft inserted) | `tasks.rs:94-96`, `core tasks.rs finish` | PARTIAL | `TaskRowView.swift:46` (YES for edits); `TaskDraftRow.swift:31` | Draft: blur with text neither adds nor discards in Swift (text lost when the row goes) |
| Edited down to empty: step deleted; new draft left empty: dropped | `C/view/notes/tasks.rs:154-184` (`finish`) | YES | `TaskList.swift:40-44`, `TaskDraftRow.swift:31` | |
| Backspace in an empty editor: delete the step and edit the one above | `keys.rs:27`, `tasks.rs:167-173` | NO | missing | |
| ⌥⌘↑ / ⌥⌘↓ in editor moves the step (not for a new draft) | `keys.rs:28-32`, `tasks.rs:174-178`, keymap `table.rs:237-238` | PARTIAL | `ReorderDrop.swift:60-66`, `StepsCard.swift:37-39` | Only while the field has focus; not as a list action on a walked row |
| ⌘↵ in a step sends it to the terminal, ⌘⇧↵ also submits | `tasks.rs:90-93` | NO | missing | |
| Send action: trimmed, empty ignored, pasted without submit | `note_ops.rs:304-310` | PARTIAL | `NotesStore.swift:21` -> `PanelHost.send` | Trim/empty check not in Swift (`SystemPanelHost.swift:34` forwards `terminal.send`; not traced further) |
| Delete action (x), also for a draft | `task_rows.rs:150-159`, `tasks.rs:209-228` | PARTIAL | `StepsCard.swift:36`, draft: `TaskDraftRow` has no x | Draft row has no hover actions in Swift |
| New draft row placed where it will go (before the step now there; before the "N done" fold) | `C/view/panel.rs:23-54`, `rows.rs:64` | PARTIAL | `StepsCard.swift:46-48` (always last, after the fold row) | |
| "N done" fold when more than 5 steps are done; open steps, then fold row, then done steps | `C/view/limits.rs:14`, `panel.rs:26-54` | YES | `TaskList.swift:75-86`, `StepsCard.swift:42-43`, `DoneFold` `:56-71` | Constant 5 matches |
| "N done" row: muted text, chevron, click folds | `task_rows.rs:62-71` | YES | `StepsCard.swift:63-70` | |
| Drag to reorder (grip, 0.4 opacity of the lifted row, drop line 2 pt in primary above/below, ghost with label) | `Gs/section.rs:57-59,129-147`, `Gs/reorder.rs`, `scale.rs:126,130` | PARTIAL | `ReorderDrop.swift:9-66` | Drop line YES (2 pt primary). Missing: grip on hover, the 0.4 dim of the dragged row, GPUI's lifted ghost (SwiftUI default preview); drop line is inset 8 pt, GPUI spans the row |
| Reorder rules (`move_item`, `step`, `follow`, `drop_edge`) | `C/view/reorder.rs:15-54` | YES | `PanelModel/Shared/Reorder.swift:8-26` | `follow` (cursor tracking) not ported; not needed without a cursor |
| Edit follows moves/deletes | `C/view/notes/tasks.rs:25-37` | n/a | per-row `@State` | Different model; check by test |
| No reorder while the row is an editor | `task_rows.rs:163` | NO | `StepsCard.swift:40` always `.reorderable` | Dragging inside the field starts a row drag instead of selecting |
| Persist: add, tick, rename, delete, move saved | `mod.rs:154-169` | YES | `NotesCoordinator.swift:52-55` | |
| Tasks/notes also synced with `.wormhole/notes.md` | see 3 | | | |

### 1c. Notes

| State / interaction | GPUI file:line | Swift | Swift file or "missing" | Note |
|---|---|---|---|---|
| Row: chevron lead, title (first non-empty line, with leading `# - * >` stripped), min height 28 | `view.rs:114,129-143`, `C/view/notes/paragraphs.rs:55-59` | PARTIAL | `NoteRowView.swift:24-38`; `NotesModel.swift:17-19` | Swift does not strip `#-*> `; "# Plan" shows as "# Plan" |
| Empty note title "New note" | `view.rs:114` | YES | `NotesModel.swift:18` | |
| Open note shows lines after the title, trimmed of blank lines; one-line note shows only its title | `paragraphs.rs:61-80`, `view.rs:186` | YES | `NotesModel.swift:22-25`, `NoteRowView.swift:56` | |
| Body text 14 pt on 20 pt line | `view.rs:278-279` | YES | `PanelMetrics.swift:33,36`, `NoteRowView.swift:61` | line gap 3 is an approximation |
| Cap 12 lines, counted as WRAPPED lines, last two lines fade (0.65 then 0.3), then "Show all . N lines" in primary | `view.rs:284,289-340`, `limits.rs:17`, `panel.rs:95-97` | PARTIAL | `NoteRowView.swift:56-72,76-80` | Swift counts source lines (`:48,57`), so a long wrapped paragraph is never cut and a 12-line note that wraps is not. Fade is one gradient over the block (0.7 to 1 -> 0.3), not per-line 0.65/0.3 |
| Cap lifted by "Show all", reset when the note is folded and reopened | `note_ops.rs:277-292` | PARTIAL | `NoteRowView.swift:17,44,66` | `showAll` never reset on fold |
| Click title/body: open, unfold, cursor to end, editor has keys | `note_ops.rs:137-172` | PARTIAL | `NoteRowView.swift:82-86` | No cursor-to-end |
| New note ("+" or ⌥⌘N): tab shown, section unfolded, new editor focused at once | `note_ops.rs:116-132`, `def.rs:41-50` | PARTIAL | `NotesCard.swift:24` | "+" appends an empty `Note` but nothing focuses it (`NoteRowView.editing` starts false; `freshNote` is never read by any view, `NotesStore.swift:12,40,47`). Result: an empty collapsed-looking row "New note" |
| New Note key (action `newNote`, ⌥⌘N) | `def.rs:41-50`; keymap `table.rs:235` | NO | `S/Wormhole/Actions/ActionRouter+View.swift` has `stashPrompt` (`:28`) but no `newNote` | Menu item/key does nothing |
| Editor placeholder: "Write a note or a prompt draft...  Command-Return sends the paragraph under the cursor (or the selection); Command-Shift-Return also submits." wrapped to panel width | `view.rs:29`, `note_ops.rs:29,52` | NO | `NoteRowView.swift:50` (`TextEditor` has no placeholder) | |
| Editor min 3 rows, grows with text to 1000 rows | `mod.rs:50-52`, `view.rs:168-174` | PARTIAL | `NoteRowView.swift:51` (`minHeight` 60, no auto-grow) | |
| Editing state: title muted, row keeps hover fill and actions | `view.rs:118,143`, `section.rs:128-129` | YES | `NoteRowView.swift:32,35,41` | |
| Command-Return sends selection else paragraph under cursor; Command-Shift-Return also submits | `note_ops.rs:40-43`, `paragraphs.rs:13-50` | NO | missing | Swift note body has no key handling except reorder |
| Esc in note: keyboard to terminal; empty note thrown away | `keys.rs:26`, `note_ops.rs:223`, `176-207` | PARTIAL | `NoteRowView.swift:52` (Esc leaves the editor only) | No terminal focus; no pruning of blank notes anywhere in Swift (a note left empty stays in the list and the DB) |
| Backspace in empty note: delete it and open the one above | `note_ops.rs:224-230` | NO | missing | |
| Prune blank notes on blur and on tick | `note_ops.rs:64-70,176-207`, `def.rs:35-38` | NO | missing | |
| ⌥⌘↑/↓ moves a note | `note_ops.rs:231-235` | PARTIAL | `NoteRowView.swift:53`, `NotesCard.swift:30-32` | Only while editing |
| Hover actions Send (whole text, not submitted) and Delete | `view.rs:151-162` | YES | `NoteRowView.swift:35` | trim: see Send above |
| Drag to reorder notes | `view.rs:206-214` | PARTIAL | `NotesCard.swift:33` | as for steps |
| Fold a note, chevron turns 160 ms ease in-out | `view.rs:136`, `Gs/controls.rs:174-183`, `timing.rs:15` | YES | `Disclosure.swift:16-17`, `PanelMotion.swift:16-18` | |
| Note body appear/disappear animation | none (rows rebuilt) | WRONG (extra) | `NoteRowView.swift:39,43`, `PanelMotion.swift:30` | Swift slides body 6 pt; GPUI cuts |
| Row insert/remove animations in the list | none | WRONG (extra) | `StepsCard.swift:41,43,47,51-52`, `NotesCard.swift:34,38-39` | Same: offset + opacity transitions that GPUI does not have |
| Save per terminal on every change | `mod.rs:154-169` | YES | `NotesCoordinator.swift:52-55` | |

### 1d. Stash prompt

| State / interaction | GPUI file:line | Swift | Swift file or "missing" | Note |
|---|---|---|---|---|
| Stash button (inbox icon, tooltip "Stash prompt into a new note" + key) in the tab strip beside the panel toggle, only while Notes is on | `view.rs:404-437` | NO | `S/PanelUI/Notes/StashPromptButton.swift` exists but nothing instantiates it (grep over `macos/` finds only its declaration) | The strip (`WindowUI/PanelTabBar.swift`) has no stash button |
| Stash action (⌥⌘S): move the typed prompt (or selection) into a new note, show Notes, unfold, keep the keyboard in the terminal | `note_ops.rs:313-329`, keymap `table.rs:236` | PARTIAL | `ActionRouter+View.swift:28`, `NotesStore.swift:28-33` | Works via the key; Swift adds a notice "Prompt stashed in a new note" (GPUI has none); nothing refocuses the terminal explicitly |
| Nothing typed: nothing happens | `note_ops.rs:324` | YES | `NotesStore.swift:29` | |
| Icon | `IconName::Inbox` | PARTIAL | `StashPromptButton.swift:19` (`tray.and.arrow.down`) | Different glyph; unused anyway |

### 1e. Keyboard, focus, VoiceOver

| State / interaction | GPUI file:line | Swift | Swift file or "missing" | Note |
|---|---|---|---|---|
| List keyboard: Notes tab is a walked list with two Tab stops (the two bands) | `G/notes/list.rs:56-64,105-133`, `C/view/notes/rows.rs:100-118` | NO | `S/PanelUI/Controller/PanelKeyboard.swift:1-4` ("Notes, Project and Usage have none") | |
| Up/Down move the row cursor over bands, steps, "N done", notes (a step being written is the editor's) | `rows.rs:103-118` | NO | missing | |
| Right/Left fold or open a band / "N done" | `list.rs:67-74` | NO | missing | |
| Return: edit a step; open a note for writing (keyboard stays in the list otherwise) | `list.rs:89-97` | NO | missing | |
| Space: tick a step, fold a note; on a band or "N done" folds it | `list.rs:88,93,98` | NO | missing | |
| Esc ladder from an editor: terminal | `keys.rs:26` | PARTIAL | `NoteRowView.swift:52`, `TaskRowView.swift:44` | see rows above |
| Show Notes key (ctrl-cmd-3) | keymap `table.rs:185` | YES | `ActionRouter+View.swift:24` | |
| Checkbox VoiceOver: step text as label, checked state | `task_rows.rs:96-100` | YES | `TaskRowView.swift:27-28` | |
| Note row VoiceOver | `view.rs:143` (label = title) | PARTIAL | `NoteRowView.swift:29-30` | chevron button named after the note, folded/open |
| New-draft checkbox label "New next step" | `task_rows.rs:98` | NO | `TaskDraftRow.swift:16` (`TaskCheckbox` is accessibility-hidden) | |

### 1f. notes.md sync (both ways)

| State / interaction | GPUI file:line | Swift | Swift file or "missing" | Note |
|---|---|---|---|---|
| Three-way merge of notes and tasks with `.wormhole/notes.md`, only for a trusted project the person wants synced | `G/project/notes_sync.rs:38-138`, core `project::plan_sync` | YES | `S/FilesBridge/Notes/RustNotesPersistence.swift:30-34`, `F/notes_sync.rs:66-155`, `NotesMapping.swift:40-53` | Merge is core's |
| Runs on the tick for every terminal, whatever tab is up | `def.rs` (project) `tick`: every 3rd tick | PARTIAL | `S/Wormhole/Panel/NotesCoordinator.swift:57,74-80` | Swift syncs only the shown terminal and only while the Notes tab is visible, every 2 s. The "waiting" message below therefore appears only after Notes was shown once |
| Waits while a note/step editor is being written | `notes_sync.rs:86` | PARTIAL | `NotesCoordinator.swift:84` (first responder is `NSTextView`) | Heuristic; also true for any text view in the window |
| Untrusted: neither read nor written; Project tab says "Notes sync: waits for the project to be trusted; its notes.md is neither read nor written until then" | `notes_sync.rs:14-17,69-83` | YES (text) | `F/notes_sync.rs:17-20,92`; shown via `F/state.rs:74-81` | See PARTIAL above for when it is produced |
| Errors reading/writing notes.md shown in the Project tab | `notes_sync.rs:101-121`; `project/panel.rs:268-272` | YES | `F/state.rs:74-81` -> `ProjectHeadCard.swift:24` | |
| Terminal closed: sync base forgotten | `hub.rs:117-120` | YES | `NotesCoordinator.swift:65`, `RustNotesPersistence.swift:37` | |

### 1g. Exact metrics: Notes (GPUI vs Swift)

| Item | GPUI | Swift | Verdict |
|---|---|---|---|
| Row min height | `ROW_H` 28 (`scale.rs:26`) | `Metric.row` 28 (`Scale/Metric.swift:13`) | same |
| Band height | `BAND_H` 32 (`scale.rs:36`) | `Metric.band` 32 | same |
| Lead column | `ITEM_LEAD_W` 16 (`scale.rs:88`) | `Metric.lead` 16 | same |
| Row inset / gap | `CARD_INSET` 8 (`bands.rs:271`), `gap_2` | `PanelMetrics.inset` 8, `gap` 8 (`PanelMetrics.swift:16-18`) | same |
| Card margin, pad, radius, gap | 10 / 4 / 12 / 8 | `PanelMetrics.swift:11-14` | same |
| Row hover/active radius | `CARD_ROW_RADIUS` 8 | `Radius.row` | same |
| Done step text | `ui.muted` | `textFaint` (`TaskRowView.swift:49`) | WRONG |
| Dragged row opacity | `DRAG_LIFT_OPACITY` 0.4 (`scale.rs:126`) | none | missing |
| Note body wrap width | panel_w - (`PANEL_ROW_X` 23 + 16 + 8) - 10 right margin | SwiftUI default wrap | not measured |
| Type roles | body 14 (`text_body`), caption 12 | `TextRole.body/caption` | same |

---------------------------------------------------------------------------------------------------

## 2. Project tab

GPUI has no list walk on this tab (`G/project/def.rs:44` `walk: None`), so there are no row-cursor keys to match.

### 2a. Folder colour card

| State / interaction | GPUI file:line | Swift | Swift file or "missing" | Note |
|---|---|---|---|---|
| Card "Folder colour" shown for any terminal that has a folder, with or without a project; above everything else | `G/project/panel.rs:84-95,102-170`; tab shown rule `def.rs:34-37` | YES | `S/PanelUI/Project/ProjectView.swift:21`, `FolderColorCard.swift:16-38` | |
| Row: None, [custom hex swatch], 8 named colours (red, orange, yellow, green, teal, blue, purple, pink), then "+" picker | `panel.rs:119-157`, `C/theme/folder_color.rs:21-31,48-54` | YES | `FolderColorCard.swift:20-31` | Same order |
| Each choice: 28 pt hit, 16 pt swatch, ring 2 + gap 2 round the chosen, None shows a ban icon, disabled = 50 % except the chosen | `Gs/folder_colors.rs:29-81` | YES | `ColorChoice.swift:20-54` | Numbers match (16/2/2/0.5) |
| Named colour as the theme draws it: hue fixed per name, lightness and chroma of the theme's ANSI 1-6 (chroma floor 0.08), then moved to be legible (3:1) on window and sidebar | `C/theme/folder_color.rs:88-135` | PARTIAL | `S/PanelUI/Project/FolderTint.swift:1-4,11-35` | Hues and L/C match; the "legible" step is missing (comment says so). Colours can be too close to the window/sidebar surface in some themes |
| Custom hex colour (set in config.json or picked): shown as a chosen, non-clickable swatch, tooltip "Custom colour #xxxxxx, set in config.json" | `panel.rs:125-138` | YES | `FolderColorCard.swift:22-25` | |
| Picker "+" (accessibility label "Pick a colour"): GPUI colour picker anchored top-right, featured colours = the 8 named, starts at the folder's current colour, saves a hex colour | `panel.rs:148-157`, `folder_color.rs:146-179` | PARTIAL | `FolderColorCard.swift:12,41-47` | System `ColorPicker` in a popover; initial value is `Color.accentColor` (`:12`), not the folder's colour; no featured colours; every change writes (GPUI too) |
| Where it is kept: own (Wormhole config, by path) vs project `config.json` `project_color`; line under the row says which | `folder_color.rs:25-120` | YES | `F/paint.rs:14-60`, `FolderPaint.swift:23`, `FolderColorCard.swift:34` | Notes strings copied |
| Locked by the project / config unreadable: row dimmed, no picker | `folder_color.rs:75-81`, `panel.rs:106,148` | YES | `FolderPaint.swift:20`, `FolderColorCard.swift:29`, `ColorChoice.swift:37` | |
| Tint percentage | The Project tab has no percentage control. Tint strength is fixed: 36 % at the band/bar's left edge, 20 % at 12 % width, 0 at 40 % (`C/project/color.rs:29-36`), the title row's bar wears half of it (`color.rs:38-40`) | n/a | `S/PanelUI/Project/` none; no `0.36` tint in Swift (`grep`: only unrelated 0.36 values) | Tint is drawn by the sidebar band and title row, outside this tab; check those separately |
| Free colours also set from the sidebar band menu | `C/theme/folder_color.rs` doc | not in scope | | |
| Write errors ("Couldn't change the project's colour: ...") shown | `G/project/color.rs:14-16,57-61` | PARTIAL | `RustProjectBackend.swift:93-96` -> notice chip | GPUI puts it in the tab's error list; Swift in the notice chip |
| Project colour needs no trust; not in the trust fingerprint | `C/project/color.rs:8-17` | YES | core | |

### 2b. No project folder

| State / interaction | GPUI file:line | Swift | Swift file or "missing" | Note |
|---|---|---|---|---|
| Card "Project folder": "This terminal's folder has no .wormhole/ project folder." + about text | `panel.rs:172-195` | YES | `S/PanelUI/Project/NoProjectCard.swift:14-17` | |
| Primary small button "Create project folder", disabled with no folder | `panel.rs:202-215` | YES | `NoProjectCard.swift:19` (`busy: store.folder == nil`) | |
| Caption: "Creates ~/path/.wormhole/" or "This terminal's folder isn't known yet." | `panel.rs:175-177,218-220` | YES | `NoProjectCard.swift:30-35` | |
| Error line: "Couldn't create the project folder: ..." and other hub errors | `G/project/create.rs:45`, `panel.rs:223` | PARTIAL | `NoProjectCard.swift:22`, `ProjectStore.swift:15,56-61` | Only the create error; hub errors (prefs/trust store reset) are not shown here. Swift shows the error in danger caption |
| Created project is untrusted and sets nothing; tab turns into the project at once | `create.rs:13-15,49` | YES | `ProjectStore.swift:56-61,107-115` | reload after the call |
| Already-there race treated as success | `create.rs:39-41` | not checked | `F/service.rs` (create) | |
| Loading | no loading state (synchronous) | YES | `ProjectView.swift:28` (`store.loaded` gate) | Swift hides the tab body until read; fine |

### 2c. Project present: header, trust, errors

| State / interaction | GPUI file:line | Swift | Swift file or "missing" | Note |
|---|---|---|---|---|
| Head row (32 pt): folder icon, "name/.wormhole" (or ".andromeda" legacy), trust badge | `panel.rs:235-252` | PARTIAL | `ProjectHeadCard.swift:15-22` | `.wormhole` hard-coded at `:17`; GPUI uses `folder.dir_name()` |
| Badge text/colours: "trusted" success, "changed" warning, "not trusted" muted | `panel.rs:595-602` | YES | `ProjectHeadCard.swift:29-35`, `ProjectState.swift:11-17` | |
| Trust card shown only when `request.is_some()` and (commands or settings needing trust or notes sync waiting) | `panel.rs:256-264` | YES | `F/state.rs:32`, `ProjectHeadCard.swift:23` | `asksForTrust` |
| Trusted: one line (shield-check success, caption muted, "Actions, the terminal layout and project env may run." or "Applies ...") + ghost xsmall "Revoke trust", 28 pt row | `panel.rs:463-482` | PARTIAL | `TrustNotice.swift:19-28` | Revoke is a secondary (filled) button, not ghost; line may wrap to 2 lines (`:23`), GPUI truncates to 1 |
| Untrusted/changed: amber warning callout, title and text per case | `panel.rs:483-538,606-625` | YES | `TrustNotice.swift:30-43`, `Callout.swift`, `ProjectState.swift:101-110` | Strings identical |
| "applies" wording incl. "its notes.md sync" | `panel.rs:458-462` | YES | `F/state.rs:25-31` | |
| Trust lines: caption in code font, `p_2`, radius, hover fill, `what` muted, `runs` in `trust.command`, note muted, wrapped never cut | `panel.rs:494-495,504-520,573-592` | YES | `TrustNotice.swift:47-77` | |
| Lines shown in full up to 6; more than that shows 5 and "View all N..." | `panel.rs:38-41,494` (`TRUST_LINES_MAX` 6, `SHOWN` 5) | WRONG | `ProjectState.swift:118-119` (`linesMax` 8, `linesShown` 5) | A list of 7 or 8 lines shows in full in Swift, folded in GPUI |
| Approve button wording: "Trust with Touch ID" / "Trust (approve with password)" (no biometrics) / "Waiting for approval..." | `panel.rs:484-490` | PARTIAL | `TrustNotice.swift:37` | Always "Trust with Touch ID"; `Authenticating.biometricsAvailable()` exists (`PlatformModel/Auth/Authenticating.swift:29`) but the tab never asks it |
| Approval reason: "trust the project folder "name" and let it run commands" | `G/project/trust.rs:51` | not checked | `F/trust.rs`, `FilesBridge/Project/OwnerAuthentication.swift:14` | Reason string is built in Rust FFI; not read |
| Cancelled: silent; failed: "Not trusted: ..." ; trusts exactly what was shown | `trust.rs:15-27` | YES | `F/trust.rs:53-82` | |
| Touch ID in automated runs | `WORMHOLE_AUTH` | YES | `OwnerAuthentication.swift:3-4` | |
| Revoke | `trust.rs:76-80` | YES | `ProjectStore.swift:77-80` | |
| "View all N..." opens a separate window "Project commands" 640x480 (min 480x320), follows the hub's copy of the project, has the same approve button, shows trust state line, "N lines", empty "Nothing here would run.", "This project is no longer open in a terminal.", Esc or Cmd-W closes | `G/project/trust_window.rs:1-9,26-28,80-170` | PARTIAL | `S/PanelUI/Project/TrustLinesSheet.swift:8-25` | A modal sheet 520x420 with title, a different sentence, the lines, and Close only. No approve button (its own header comment claims one), no count, no empty/gone states. Esc closes (`keyboardShortcut(.cancelAction)`) |
| Errors list: hub error, project load errors, "Notes sync: ..." in danger caption (`py_1`, inset 8) | `panel.rs:265-272,566-568` | YES | `F/state.rs:74-81`, `ProjectHeadCard.swift:24,38-42` | |
| Warnings (e.g. an action key already taken) in warning-text colour | `panel.rs:274-276` | YES | `ProjectHeadCard.swift:25` | |

### 2d. Sections (trusted project, or untrusted where the section needs no trust)

| State / interaction | GPUI file:line | Swift | Swift file or "missing" | Note |
|---|---|---|---|---|
| Card "Use from this project": one two-line row per feature present (Settings, Theme, Actions, Prompts, Terminal layout) and always Shared notes; switch on the right, hint under the title, warning colour when held by trust | `panel.rs:278-314,627-657`, `C/project/features.rs` | YES | `S/PanelUI/Project/ProjectSectionCards.swift:9-27`, `F/state.rs:34-47` | |
| Feature switch behaviour (prefs per project, for you only) | `trust.rs:92-101` | YES | `ProjectStore.swift:63-66` | |
| Feature glyphs (settings, palette, play, file-text, square-terminal, file-plus) | `panel.rs:648-657` | PARTIAL | `ProjectFeature.swift:30-39` | SF Symbols stand in for Lucide |
| Notes row hints: "Syncs once the project is trusted" (warning), description, "On creates notes.md and syncs notes" | `panel.rs:630-641` | YES | `F/state.rs:99-108` | |
| Actions card: shown when Actions feature enabled and not empty; count; row = Play icon (primary), title, detail (description or command / `prompts/file` / first prompt line / URL / "N steps"), key glyphs trailing caption faint; click runs | `panel.rs:316-351,659-671` | PARTIAL | `ProjectSectionCards.swift:29-47`, `F/state.rs:49-53,113-133`, `ProjectStore.swift:82-85` | Click path YES (Rust decides, window performs: `Wormhole/Panel/PanelEffects.swift:14-42`) |
| Running an action: run (typed + Return), prompt (paste, optional submit), open (http/https only), chain (depth cap 16), targets current/new/named | `G/project/actions.rs:111-150` | YES | `FilesBridge/Project/PanelEffect.swift:24-66`, `PanelEffects.swift:28-42` | |
| Run an action from its key or the Project Actions menu | `G/project/def.rs:90-97,105-124` | NO | `S/Wormhole/Actions/ActionRouter.swift:43` (`.runProjectAction` -> `break`) | Keys and the menu list do nothing; only the tab's click works |
| Prompts card: when Prompts enabled and not empty; count; row = file-text icon, title, description; hover actions "Paste into the terminal" (arrow-up-right, trimmed body, no submit) and "Copy into a new note" (file-plus) | `panel.rs:353-406`, `actions.rs:73-99` | PARTIAL | `ProjectSectionCards.swift:49-71`, `ProjectStore.swift:87-100` | Copy-to-note makes the note but does not focus it (same `freshNote` gap, `NotesStore.swift:36-41`) |
| Terminal layout card: when Layout enabled and layout non-empty; count; row 38 pt, names joined by " . " caption muted, left pad 8+24, primary xsmall "Open terminals" | `panel.rs:408-438` | YES | `ProjectSectionCards.swift:73-88` | Button is `NoticeButton` (22+2 pt) vs GPUI xsmall; visually close |
| Open layout: reuse a terminal with the same name; cwd; run; focus | `actions.rs:34-54` | YES | `PanelEffects.swift:44-50` | |
| Section bands have VoiceOver "Actions, 4 items" / "Prompts, no items" | `panel.rs:561`, `spoken.rs:138` | NO | `ProjectSectionCards.swift:36,57,80` (`CardBand` without `spoken`) | |
| Two-line row geometry: fixed 44 pt, title one line 20, detail ONE truncated line 16, 4 above/below | `Gs/rows/commits.rs:33-76` (`ROW_TWO_LINE_H` 44, `NAME_LINE` 1.25 rem, `META_LINE` 1.0 rem) | WRONG | `S/PanelUI/Shell/TwoLineRow.swift:30-31,40` | Detail is `lineLimit(2)` and the row is `minHeight`, so long hints grow the row; a row without detail is 28 (GPUI 44) |
| Section visibility follows `ProjectHub::enabled` (present + switched on + trusted if needed) | `G/project/hub.rs:98-102` | YES | `F/state.rs:49-72` | Filtered in Rust |
| Project folders feature flag off: tab inert, no read/write | `G/project/mod.rs:81-89` | not checked | | |
| Trust check on every run/send | `actions.rs:21-31` | YES | `RustProjectBackend.swift:1-4,65-83` | "Not allowed: ..." goes to the notice chip (`PanelEffects.swift:23-26`) |

### 2e. Motion and keyboard on this tab

| State | GPUI | Swift | Note |
|---|---|---|---|
| Section cards have no fold animation, no row transitions | `panel.rs` (plain divs) | PARTIAL (extra `.transition(motion.content)` for the tab change at `PanelContentView.swift:23-24`; GPUI also slides tabs 140 ms, `view::motion`) | tab change is shared |
| Keyboard | Tab/arrow handled by buttons and switches only | YES | SwiftUI controls |

---------------------------------------------------------------------------------------------------

## 3. MISSING or WRONG in Swift, by priority

Size: S = under half a day, M = about a day, L = several days.

1. **New note does not open for writing; no ⌥⌘N; no placeholder; no Esc to terminal; no blank-note pruning; no ⌘↵ send** (`NotesCard.swift:24`, `NoteRowView.swift:50-54`, `ActionRouter+View.swift`). The Notes tab's core flow is broken: "+" leaves an empty "New note" row that nothing focuses, and empty notes are kept forever. Needs: focus-on-create (use `freshNote`, also for stash and prompt-to-note as GPUI's `note_from`), `newNote` action, placeholder overlay, Esc -> `host.focusTerminal()`, prune on blur and on tab tick, ⌘↵ / ⌘⇧↵ (paragraph under cursor or selection), Backspace-in-empty. **M**
2. **Task editor keys and flows** (`TaskRowView.swift:41-62`, `TaskDraftRow.swift:17-31`): Esc must keep text and return to the terminal (now drops it); Enter in an existing step must insert the next draft right below; Enter or Backspace in an empty field must leave/delete-and-go-up; blur of a non-empty draft must keep it; ⌘↵ sends the step; placeholder "Next step"; draft needs hover x. **M**
3. **Notes tab keyboard list (arrows, Return, Space, left/right fold, Tab stops at the two bands, ⌥⌘↑/↓ on a walked row)** (`PanelKeyboard.swift:1-4`, nothing in `PanelUI/Notes/`). Needs a row cursor over bands, steps, "N done" and notes, as `rows.rs:100-118` + `list.rs:56-133` (the Changes/History tabs already use `SelectableListSplit.swift`). **L**
4. **Stash button missing from the tab strip** (`StashPromptButton.swift` is never instantiated; `WindowUI/PanelTabBar.swift`). Add it beside the panel toggle while Notes is on, with the key in the tooltip. **S**
5. **Project actions: keys and the "Project Actions" menu do nothing** (`ActionRouter.swift:43`). Route `.runProjectAction(id)` and the action keys to `ProjectStore.run` / the FFI `runAction`; list actions in the menu state only for trusted + Actions on. **M**
6. **Done step colour wrong** (`TaskRowView.swift:49-50`: `textFaint` -> `textMuted`, strike in the same colour). **S**
7. **Trust lines limit wrong** (`ProjectState.swift:118`: `linesMax` 8 -> 6). **S**
8. **Approve button wording ignores Touch ID availability** (`TrustNotice.swift:37`): add "Trust (approve with password)" using `Authenticating.biometricsAvailable()`. **S**
9. **"View all N..." is not the GPUI window**: no approve button, wrong sentence, no line count / empty / project-gone states, 520x420 vs 640x480 (`TrustLinesSheet.swift`). Make it a window (or sheet) with the approve button and the states of `trust_window.rs:80-170`. **M**
10. **Two-line row geometry** (`TwoLineRow.swift:30-31,40`): fixed 44 pt, one truncated detail line, no-detail rows also 44. Affects Features, Actions, Prompts. **S**
11. **Note title/body rules and wrapped cap** (`NotesModel.swift:17-19`, `NoteRowView.swift:48-72,76-80`): strip leading `# - * >` from the title; cap 12 wrapped lines, not source lines; per-line fade 0.65 / 0.3; reset `showAll` on fold; whitespace-only notes not counted (`NotesModel.swift:41`); editor min 3 rows and auto-grow. **M**
12. **Extra motion not in GPUI** (`TaskCheckbox.swift:17-25`, `StepsCard.swift:41,43,47`, `NotesCard.swift:34`, `NoteRowView.swift:39,43`): bouncy checkbox, row insert/remove slides, body slide. Decide: remove for parity or list as accepted gains. **S**
13. **Drag details** (`ReorderDrop.swift`): grip on hover (`section.rs:57-59,132-147`), 0.4 opacity on the dragged row (`scale.rs:126`), no drag from inside an editor (`task_rows.rs:163`), drop line full row width. **S**
14. **Spoken labels**: band labels "Notes, 2 notes" / "Notes, no notes" / "Next steps, none open" (`NotesCard.swift:23`, `StepsCard.swift:25`); Project bands "Actions, 4 items" (`ProjectSectionCards.swift:36,57,80`); panel tab "Notes, 2 open next steps, 1 note" (`PanelTabBar.swift:30-33`); new-draft checkbox "New next step". **S**
15. **Close question ignores notes and tasks** (`AppController+Quit.swift:74,114,128` pass `stakes: []`): add "notes and tasks are discarded." when filled notes + open steps > 0 (`stake.rs:6`). **S**
16. **notes.md sync only while the Notes tab is up and only for the shown terminal** (`NotesCoordinator.swift:57,74-80`): GPUI syncs every terminal on the tick whatever tab is shown, so the "Notes sync: waits for the project to be trusted" line and merges lag in Swift. Poll in the background (or on the Rust watch) for all terminals. **M**
17. **Folder colour picker**: starts at accent colour, no featured colours, system panel instead of GPUI's popover (`FolderColorCard.swift:12,41-47`); `FolderTint.swift` lacks the legibility step (`folder_color.rs:98-118`). **M**
18. **Smaller text/layout deltas**: tooltip of "+" lacks the key (`NotesCard.swift:24`); `.wormhole` hard-coded in head (`ProjectHeadCard.swift:17`, GPUI uses `dir_name()` incl. legacy `.andromeda`); Revoke is not a ghost button and the trusted line may wrap (`TrustNotice.swift:23,25`); hub errors not shown in the no-project card (`NoProjectCard.swift:22`); stash adds a notice GPUI does not (`NotesStore.swift:31`); empty `notesCollapsed` not reset on last delete (`NotesCard.swift:21`). **S** together

Not verified (needs running or deeper tracing): whether `terminal.send` trims and skips empty text; whether the Notes feature flag and `project_folders` flag hide the tabs in the Swift window (`WindowHub+Focus.swift:7` takes the core layout); Rust FFI approval reason string; exact wrap widths and the 14/20 line metrics of a note body.
