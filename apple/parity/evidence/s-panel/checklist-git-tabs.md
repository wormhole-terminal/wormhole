# Right panel: Changes, Diff, History, Worktrees — GPUI states vs the Swift panel

Read-only comparison, nothing built or run. Every Swift claim was checked by reading the file named.
Paths: `G:` = `/Users/benjamin/Projects/terminal/wormhole/crates/wormhole/src/features/git/`, `C:` = `crates/wormhole-core/src/`,
`S:` = `apple/swift/Sources/` (swift-s-panel worktree). Status: YES / PARTIAL / NO.

Reading notes
- In GPUI the diff (Changes and History) is one view, `Changes::render_diff`, laid over the terminal; History only calls
  `changes.open_diff` (G:history/refresh.rs `open_commit_file`). Swift has two panes (`ChangesDiffPane`, `HistoryDiffPane`) over one `DiffView`.
- Worktrees has NO tab in GPUI (G:worktree/def.rs `tabs: &[]`): it is a menu/palette action, a dialog, the close question and the spawn env.
- The window hosts the cover in Swift (`presentation = .terminalArea`, S:WindowUI/Hub/WindowHub.swift:52), so the in-panel diff (`.panel`) is not what ships.
- GPUI icons are lucide (`IconName::*`); Swift uses SF Symbols everywhere (a global look difference, not listed per row).
- GPUI `features/git` has no animation call at all (grep `with_animation|motion::` is empty). See "Motion" below.

---------------------------------------------------------------------------------------------------

## 1. Changes tab

### 1a. States of the tab

| State | GPUI file:line | Swift | Swift file | Note |
|---|---|---|---|---|
| Not a repository | G:changes/view.rs:66-75 (text "This terminal's folder isn't inside a git repository." in `panel_card`) | PARTIAL | S:PanelUI/Changes/ChangesListView.swift:18 | Same card, other words: "This folder isn't in a git repository." |
| Loading, before git answered | view.rs:69-72 "Reading git status…", card without a head | PARTIAL | ChangesListView.swift:17 | Swift: "Reading changes…" and WITH the band head (compare chip, refresh) |
| git missing ("Changes need git, and it isn't installed." + hint "Install the Xcode Command Line Tools, then retry." + code well `xcode-select --install` + Retry) | G:problem.rs:42-50, render 82-130 | NO | missing | Swift has only `isRepository` bool + `GitFailure(message)`; no GitError kinds (S:GitBridge/GitQueue.swift:25-32) |
| "git doesn't trust who owns this folder" + `git config --global --add safe.directory '<path>'` + hint "If you trust it, run this once, then retry." + Copy error | problem.rs:53-61, 75-79 | NO | missing | needs core wording through the FFI |
| Generic git failure: "Couldn't read this repository." + git's first line in code font + Retry + Copy error | problem.rs:62-67, buttons 115-127 | PARTIAL | ChangesListView.swift:19 (`EmptyNote(text: why)`) | Message only, muted; no headline, no Retry, no Copy error, no code well |
| Refresh failed but a list exists: list stays, red caption "Couldn't refresh: <first line>" over it | problem.rs:134-143, view.rs:108 | NO (wrong) | S:PanelUI/Stores/ChangesStore.swift:60-61 | Swift sets `.failed` and the list DISAPPEARS (`ChangesListView.swift:19-20`) |
| Compared branch deleted: amber line "<branch> no longer exists · showing vs HEAD" (Shield icon, `warning_text`, ROW_H 28, px_4) | problem.rs:156-167, view.rs:107 | NO | missing | `GitRepoState.compareMissing` exists (S:GitBridge/GitMapping.swift:109) but `ChangesStore` calls `changes()` not `state()` (ChangesStore.swift:55) |
| Clean tree / empty list: head stays (compare picker reachable), note "No changes. Working tree is clean." | view.rs:109-122 | YES | ChangesListView.swift:27-31 | |
| Empty while comparing | same note ("No changes. Working tree is clean.") | WRONG | ChangesListView.swift:30 | Swift invents "No differences from <rev>." |
| Section bands: "Changes" / "Untracked" (title "Changes" when grouping off), chevron, count badge (grouped "1,234", hidden at 0), click folds | rows.rs:33-38, view.rs:283-346 | YES | S:PanelUI/Shell/CardBand.swift, ChangesListView.swift:43-55 | |
| Count rules: group untracked only when not comparing (`untracked_apart(setting, compared)`) | C:view/panel.rs:66-68, rows.rs:104 | NO | S:PanelModel/Changes/ChangeRows.swift:21-25 | `groupUntracked` ignores `compare`; compared list should be one "Changes" group |
| Long list: section over 200 files starts folded (`FOLD_ABOVE`), person's fold choice wins | C:view/limits.rs:11, panel.rs:58 | YES | ChangeRows.swift:6,28 | |
| FoldNote under a folded big section: "<N> of them are in <dir>/[, which isn't ignored]. Folded because there are more than 200." | panel.rs:73-91, rows.rs:135-138 | YES | ChangeRows.swift:53-62, ChangesListView.swift:66-69 | GPUI note row: `px(CARD_INSET) py_2 caption muted` (view.rs:172-178); Swift same insets |
| Spacer (16) under last card so the last row scrolls clear | rows.rs:147, view.rs:258 | PARTIAL | S:PanelUI/Shell/PanelCard.swift:30 (`tail - cardGap`) | Equivalent |
| List vs tree; chains of single folders compacted "a/b/c"; folders before files | rows.rs:141-192 | YES | ChangeRows.swift:36-40,64-78 | Swift sorts keys with Swift `String` order, Rust uses byte order (BTreeMap): differs for non-ASCII / mixed case only |
| Folder row: chevron, folder icon, name, click folds; collapse key per session+group+path | view.rs:179-209, rows.rs:44 | YES | S:PanelUI/Changes/FileRowView.swift:76-97 | |
| Folder row right-click menu (path menu of the folder: copies, Reveal, File History) | view.rs:192-204 | NO | missing | FolderRowView has no `.contextMenu` |
| Display order for the diff's ‹ › = list order incl. folded sections/dirs | rows.rs:154-166 | YES | ChangeRows.swift:46-51 | |

### 1b. File row (`style::file_row` + `file_line.rs`)

| Item | GPUI file:line | Swift | Swift file | Note |
|---|---|---|---|---|
| Icon (icon theme) in 16 lead column, tint muted / selected_muted | file_line.rs:51-58 | YES | FileRowView.swift:23, S:PanelUI/Shell/FileIcon.swift | |
| Name: at most 55 % of row, cut in the MIDDLE keeping the extension (`elide::middle`, measured in the UI font) | file_line.rs:176-181, C:view/elide.rs:17-38, scale.rs:143 | PARTIAL | FileRowView.swift:28-33 | SwiftUI `.truncationMode(.middle)` does not keep the extension; no measuring |
| Folder: caption, muted, cut at the FRONT keeping the folders nearest the file, gets exactly the room left after name, counts, letter, 5 gaps | file_line.rs:183-206 | PARTIAL | FileRowView.swift:34-37 | `.truncationMode(.head)` but room is not computed; may squeeze the name instead |
| Row padding: left `CARD_INSET 8 + depth*24`, right 8, min height 28, gap 8, radius 8 | rows/files.rs:59-62, rows/mod.rs:31-44, scale.rs:27,91,CARD_ROW_RADIUS | YES | FileRowView.swift:22,45-47, PanelRow (RowSurface.swift:34) | S:PanelMetrics.swift:16 inset 8; Metric.indent 24; Metric.row 28 |
| Renamed: old NAME (caption, muted) + arrow(12) before the new name when the name changed | file_line.rs:125-139, rows/files.rs:64-74 | YES | FileRowView.swift:24,57-62 | |
| Moved across folders: mark (FolderInput 12) before name; a rename that kept the name shows only the mark | file_line.rs:131-138, rows/files.rs:75-82 | PARTIAL | FileRowView.swift:25-27, ChangedFile.swift:54-57 | Swift mark is `folder.badge.gearshape`, GPUI "move into folder" glyph |
| Line counts "+N −M" caption, tabular; zero removals shows only +N; "bin" for binary; "LFS"; nothing for a nested repo folder (`FileLines::Folder`) | file_line.rs:43-52, labels.rs:162-171 | YES (Folder case: n/a) | S:PanelUI/Changes/LineCounts.swift | Nested repo ("dir/") shows as a plain file row in Swift with counts/letter (`ChangedFile` has no folder case; name falls out of `split("/")` correctly but the folder icon and empty counts are missing) |
| Kind letter bold caption in 12 pt column: M warning, A/U success, D/! danger, R primary-as-text | rows/files.rs:104-126 | YES | FileRowView.swift:40-43,64-71 | |
| Selected = diff open for this path OR ⌘-marked: fill + semibold name + selected_muted | view.rs:214-219 | PARTIAL | FileRowView.swift:29-30,55, RowSurface.swift | selected part only; marks missing |
| Tooltip with whole path (code font) + "Modified · +128 −40" only when the row cut the path or it is a rename | file_line.rs:75-97,142-165 | PARTIAL | FileRowView.swift:50 | Swift `.help` on every row, plain text |
| Deleted file has no open-in-editor icon; row hover icon "Open in <editor>" opens at file | view.rs:233-238, open_file.rs:101-113 | NO | missing | `PanelRow` passes `hovering` but FileRowView ignores it (`{ _ in`) |
| Submodule row: FolderGit2 icon, name (max 45 %), folder, "what changed" words (shrinks after folder), hover button "Open a terminal in <name>" (only when checked out and not deleted), kind letter; menu none; aria "Submodule <name>, <what>" | G:submodule_row.rs:17-69, G-style submodule.rs:36-95 | PARTIAL | S:PanelModel/Changes/ChangedFile.swift:38-45 (`Lines.submodule`), FileRowView.swift | Swift draws it as a file row with the words in the counts slot; no folder icon, no Enter button, a file menu that does nothing sensible |
| Nested repository listed by git as "dir/" (a folder row, no counts) | file_line.rs:37-46, 52 | NO | missing | |
| VoiceOver: row label | file_row has no explicit label (name text); band `section_band(title, n, Files)` e.g. "Changes, 3 files", "Changes, no files" | PARTIAL | FileRowView.swift:52, ChangesListView.swift:46 | Swift: "<name>, <Kind>" (extra), band says "Changes, 0 files" where core says "no files" |

### 1c. Band head controls (first card only)

| Item | GPUI file:line | Swift | Swift file | Note |
|---|---|---|---|---|
| Compare chip "vs [⎇] <branch|HEAD> ⌄" (compact select: 24 tall, caption 500, strong outline, prefix muted, branch mark `git.branch`) | view.rs:90-100, picker.rs:241-300 | YES (look) | S:PanelUI/Shell/RevPicker.swift:39-56 | height 24 / maxWidth 160 vs GPUI `BRANCH_CHIP_MAX 112`-style menu width 320 for the popover (RevPicker.swift:90 uses 260) |
| Picker list: first option "HEAD (uncommitted changes)" (History: "HEAD (current branch)"), then branches; stashes only in History | picker.rs:149-159 | PARTIAL | RevPicker.swift:81 | Swift row title is just "HEAD"/branch name |
| Picker hints: "Loading branches…" while loading; "Changes since this branch split off from it" (Compare) | picker.rs:292-298 | NO | missing | |
| Search box placeholder "Search branches"; Return picks the first match; esc closes | picker.rs:57, 212-223 | YES | RevPicker.swift:74-77,93 | Swift also lists tags/commits (core `revs`); GPUI lists branches only for Compare |
| Picker closes when another tab shows / another terminal is selected | def.rs:96-102, 123 | PARTIAL | ChangesStore.swift:34-47 (`follow` clears compare) | popover is SwiftUI state, not closed by tab change |
| View options button (SlidersHorizontal, tooltip "View options") → menu: `List` (check), `Tree` (check), separator, `Group untracked files separately` (check); writes the SAME settings as Settings → Git | view_menu.rs:11-29 | PARTIAL | S:PanelUI/Changes/ViewOptionsMenu.swift:11-19 | Swift: inline Picker titled "Show files as" with Tree BEFORE List, Toggle "Group untracked files"; changes `store.layout` only — never written to config.json |
| Refresh button (RefreshCw, tooltip "Refresh") | view.rs:312-319 | YES | ChangesListView.swift:52 | |
| Count badge after the controls | view.rs:332 | YES | CardBand.swift:24-28 | |

### 1d. Selection, keys, menus

| Item | GPUI file:line | Swift | Swift file | Note |
|---|---|---|---|---|
| Click on file opens its diff; ⌘-click marks/unmarks (multi-select), plain click clears marks | view.rs:247-254, list.rs:143,159 | PARTIAL | ChangesView.swift:22, FileRowView.swift | no marks |
| Right-click file menu: `Open in <editor>` (only if the app is not Finder; disabled when deleted) · `Reveal in Finder` (disabled when deleted) · — · `Copy Path` · `Copy Relative Path` · — · `File History`; with several marked: `Copy Paths`, `Copy Relative Paths`, —, `History of These Files` | path_menu.rs:26-73 | PARTIAL | S:PanelUI/Changes/ChangedFileMenu.swift:22-32 | Swift `host.editorName` is always nil (S:WindowUI/SystemPanelHost.swift:21) so "Open in" never appears; Reveal never disabled for deleted; no plural/multi forms; copies post no "Copied" notice (SystemPanelHost.swift:24-27 vs `notice::post(Copied::Path)` path_menu.rs:33) |
| File History: shows History tab limited to those paths | path_menu.rs:65-72 | YES | ChangedFileMenu.swift:31, S:PanelUI/Controller/PanelController.swift:50-53 | |
| Keys: ↑ ↓ move over bands, folders AND files (row cursor ring), → ← fold/open (← on child goes to parent), ↩ opens a file's diff (list keeps keys), Space/↩ on a band or folder folds; a diff that is open follows the cursor from file to file; ⇥ one stop per section (band leads it); F6 area | G:changes/list.rs:166-242, window/focus/panel.rs:1-8, 90-125 | PARTIAL (wrong shape) | S:PanelUI/Shell/SelectableListSplit.swift:44-48, ChangesView.swift:22-25 | Swift: ↑ ↓ step over FILES ONLY and open each diff immediately; no → ←, no ↩/Space (`fold`/`press` are nil for Changes), no ring on bands/dirs, no per-section tab stops. F6 request works (`keyboard.request`, SelectableListSplit.swift:40) |
| ⌘C on the cursor row | list.rs:235 `copy: None` for Changes (History has it) | n/a | — | |
| Esc closes the diff, keys back to the terminal | def.rs:75-79 | YES | S:PanelUI/Diff/DiffView.swift:48, PanelController.closeCover | |
| Tab count badge on the strip = changed files, "999+" cap | def.rs:47, limits.rs:35 | YES | S:PanelUI/Controller/PanelController.swift:56-64, S:PanelUI/Shell/TabCountBadge.swift:19 | |
| Spoken tab: "Changes", "Changes, no changes", "Changes, N files" | spoken.rs:100-110 | NO | missing (window strip) | not in PanelUI |
| Terminal ring / cached chrome | focus/panel.rs, `facts.panel_cursor` | NO | missing | no cursor ring |

### 1e. Settings that change the tab

| Key | How GPUI reads it | Swift | Note |
|---|---|---|---|
| `changes_tree` (default true) | list.rs:113 every frame; written by the view menu and Settings → Git | NO | `ChangesStore.layout = ChangeRows(tree: true)` (ChangesStore.swift:16); no reader of the key in S:Wormhole or S:WindowUI (grep) |
| `group_untracked` (default true) | list.rs:113 + `untracked_apart` | NO | same; plus compare rule missing |
| `sidebar_changes`, `sidebar_changes_scope/plain/shared/remember` | sidebar row counts, not this tab | n/a here | Swift reads `sidebar_changes` for the sidebar (S:Wormhole/Settings/SidebarSettingsMaker.swift:15); the other four are not read (grep) |
| `icon_theme` (file icons) | `crate::icons::file/folder` | YES | S:PanelUI/Shell/FileIcon.swift (`fileIcons` env) |
| `code_font_*` | only the tooltip path uses `code_font` here | n/a | |

---------------------------------------------------------------------------------------------------

## 2. Diff (opened from Changes or a History commit)

### 2a. Frame and header

| Item | GPUI file:line | Swift | Swift file | Note |
|---|---|---|---|---|
| Diff fills the terminal's own surface (no card inside a card), `surface.window` opaque, Esc closes | render.rs:407-418, def.rs:61-94 | YES | S:PanelUI/Shell/PanelCoverView.swift, Diff/DiffView.swift:47 | |
| Title row: ‹ › buttons ("Previous file"/"Next file"), "File N of M" (always), commit short hash in primary when from History, then the window's path crumb, then badges, counts, controls | header.rs:36-81 | PARTIAL | S:PanelUI/Diff/DiffHeader.swift:24-31, Shell/PanelCoverTitle.swift | Swift shows `file.name` only (no path crumb); "File N of M" only when split fits (:27); no commit context |
| ‹ › step WRAPS (rem_euclid) | diff/mod.rs:301-311 | NO (wrong) | S:PanelModel/Shared/ListNavigation.swift:6-10 | Swift clamps |
| Open-in-editor header button (ExternalLink) when the file is on disk | header.rs:175, open_file.rs:95 | NO | missing | |
| Badges (outlined `status_badge`, colour text): "New file" (old side missing) success · "Deleted" danger · "Renamed from <name>"/"Renamed" primary · submodule label muted · "No longer changed" · "Binary" · "LFS" · "Mode 100644 → 100755" · "No highlighting" · "Out of date" (tooltip = why refresh failed) | header.rs:99-139 | PARTIAL | DiffHeader.swift:58-68 | Swift has New file/Deleted/Renamed/Binary/LFS; NO "No longer changed", "Mode a → b", "No highlighting", "Out of date", submodule label |
| Badge look: OUTLINED (1 hairline in the colour, text in the colour, no fill), `radius_chip`, h 16, px_2 | G-style labels.rs:49-51 (`status_badge`), 53-67 | NO (wrong) | DiffHeader.swift:78-81 | Swift fills `color.opacity(0.15)`, no outline; same in History chips (S:PanelUI/History/CommitRefs.swift:33-35) |
| Counts "+N −M" in CODE font, caption, grouped (1,234), only over lines (not binary/LFS pointer/submodule) | header.rs:140-168 | YES | DiffHeader.swift:33-38 | |
| Wrap button (TextWrap, "Wrap long lines", `selected(wrap)`), shown if lines OR split fits | header.rs:176-186, 253-255 | PARTIAL | DiffHeader.swift:39-40 | Swift only with counts; shown dimmed (0.6) not "selected" |
| Split / Unified segmented control (track 24, segments 20) + keycap "esc" + "close" (only when ≥ 560 pt) ; narrower: × button "Close diff (esc)" | header.rs:188-236, C:limits.rs:21 | YES | DiffHeader.swift:42-52, DiffSegmented.swift | |
| Header heights 44 (switch) / 40 | header.rs docs | PARTIAL | DiffHeader.swift:55 (in title row: window gives height) | |

### 2b. Rows and body

| Item | GPUI file:line | Swift | Swift file | Note |
|---|---|---|---|---|
| Loading: "Loading diff…" (empty_note) | render.rs:431-433 | PARTIAL | DiffView.swift:75 | "Reading the diff…" |
| Error text (empty_note) | render.rs:434-436 | PARTIAL | DiffView.swift:76 | drawn as the chip, not a note |
| Split: two equal columns, divider line full height, fillers hatched (45°, 3 stripes per row), a whole missing side hatched as one block | render.rs:187-192, 517-523, 558-568 | PARTIAL | DiffRowView.swift:28-31, DiffHatch.swift, DiffView.swift:58,66 | per-row hatch, no whole-column hatch for new/deleted file; no divider below last row |
| Unified: two gutters (old, new) + marker + code | render.rs:212-217 | YES | DiffRowView.swift:64-69 | |
| Row height `max(22, round(font*1.75))`; wrapped continuation lines also a row tall (`line_height(row_height)`) | render.rs:83-85,203 | PARTIAL | S:Diff/DiffStyle.swift:16, DiffRowView.swift:35,120 | Swift sets only min height; wrapped lines use natural line height |
| Gutter `ceil(max(3,digits)*0.6*font + 20)`, marker 16, code pad-right 12 | render.rs:31-35,172-185,327 | PARTIAL | DiffStyle.swift:18-20 | Swift uses real glyph advance instead of 0.6 em |
| Hunk header row: `diff.hunk.*` colours, px_3, truncated; in split it stays in the old column | render.rs:206-211 | PARTIAL | DiffRowView.swift:21-24 | Swift header spans the full row |
| Tints from tokens `diff.added.background` etc.; `line_number`, `filler`, `border` | render.rs:110-129 | YES | DiffRowView.swift:125-131, S:DiffCode.swift | |
| "no newline at end" caption after the line | render.rs:324-326 | YES | DiffRowView.swift:116-118 | |
| Tabs expanded to `tab_width` (setting, default 4) | rows.rs `visible(&text, key.tab_width)` | PARTIAL | S:Diff/DiffCode.swift:11 (`tabWidth = 4` constant) | setting `tab_width` not read |
| Syntax colours (tree-sitter, from full old/new files; skipped past 20,000 lines → "No highlighting") | C:git/diff.rs:36, header.rs:130 | PARTIAL | S:PanelModel/Syntax/SyntaxLexer.swift, DiffHighlighting.swift | own lexer per hunk, not tree-sitter; no skip badge |
| Wrap OFF: sideways scroll moves code only (gutters stay), fade at cut edges, thin position bar (BAR_H 6, THUMB_MIN 24), no diagonal scroll | render.rs:41-52,338-382, diff/mod.rs:183-191,313-333 | PARTIAL | DiffView.swift:59 (`ScrollView([.vertical,.horizontal])`) | gutters scroll away with the code; no fades/bar |
| Default wrap | settings `diff_wrap = false` | WRONG | S:Stores/DiffSession.swift:17 (`wrap = true`) | |
| Split/unified setting `diff_split` (default true), write on switch, falls back to unified under 560 | diff/mod.rs:262-292 | PARTIAL | DiffSession.swift:16 (`split = true`, local) | not read, not persisted |
| Comment on a line (working-tree diff only): hover shows MessageSquarePlus at left; click opens dialog "Comment on <path:line[ (removed line)]>", quoted code chip, textarea (3-10 rows, placeholder "What should the agent change here?"), footer: key hint "⌘↩ saves", Remove (if editing), Cancel, "Add Comment"/"Save"; empty text removes | G:diff/review.rs:72-192, 212-240, render.rs:226-251 | YES | S:PanelUI/Diff/CommentEditor.swift, DiffRowView.swift:60-103 | popover instead of dialog; 4 fixed rows (no auto-grow); icon is `text.bubble` 10 pt |
| Commented line wears a 2-pt primary bar at left edge | render.rs:54,238 | YES | DiffRowView.swift:72-74, DiffStyle.swift:22 | |
| Review bar under rows (40 high): "N comments[ on M files]" muted · Clear (ghost) · "Paste into Terminal" (primary, tooltip "Paste the comments into the terminal's prompt, for you to send") | review.rs:285-335 | YES | S:PanelUI/Diff/ReviewBar.swift | |
| Paste: only into a program that takes bracketed paste; else copy to clipboard and alert "Copied the comments instead" (comments stay); success clears comments, closes diff, focuses terminal | review.rs:246-273 | PARTIAL | S:Stores/ChangesStore.swift:104-114, ChangesView.swift:45-49, WindowHub+PanelHost.swift:14-18 | Swift UI exists, but `host.paste` returns `true` whenever there is a source and never asks "does the program take bracketed paste" (`source.paste` returns void) — the safety branch can never run |
| Comments only for working tree vs HEAD (`is_working_tree`), none in History/compare | diff/load `is_working_tree`, render.rs:468-469 | PARTIAL | ChangesStore.swift:99 (`canReview = compare == nil`) | History diff has none (YES); compare rule (YES) |
| Open-at-line icon at the right end of a line while hovered (`open_line`; removed line → nearest) | render.rs:253-270 | NO | missing | |
| Review message text/limits (160-char quote, no control chars) | C:git/review.rs:21 | YES | S:PanelModel/Review/Review.swift:29,62-95 | ported by hand, not through the FFI |
| Open diff stays in step with the file: re-read when the file or its status entry changes, scroll kept; file leaves the list → diff stays with badge "No longer changed" | G:diff/load/mod.rs:270-283, 338-391 | NO (wrong) | S:Stores/ChangesStore.swift:59 (`close()` when the file leaves the list) | and an edited file is never re-read |
| Dropping the diff when compare changes / terminal changes | changes/mod.rs:67-71, 133-140 | YES | ChangesStore.swift:86-90, 34-47 | |

### 2c. File kinds (what the body shows)

| Kind | GPUI file:line | Swift | Swift file | Note |
|---|---|---|---|---|
| Text (modified/added/deleted/renamed/untracked/conflicted) | rows.rs, render.rs | YES | DiffRows.swift | |
| New file (old side missing): note "New file" over the old column, hatch whole column | notes.rs:46,71 (`Side::Missing`), render.rs:455 | PARTIAL | DiffNote.swift:20-25 | Swift shows no per-column note; only badge |
| Deleted: note "Deleted" over the new column | notes.rs:47 | PARTIAL | same | |
| Empty new file: "Empty file"; mode change only: "No content changes" (+ badge "Mode a → b") | notes.rs:51-52, header.rs:127-129 | PARTIAL | DiffNote.swift:23-24 | text yes, mode badge no (no `mode_change` in `DiffFile`) |
| Binary: note "Binary file" per column + sizes ("48.2 KB", "48.2 KB → 52.9 KB", in `size_units`) | notes.rs:48,57-59,78 | PARTIAL | DiffNote.swift:21 | note without sizes; `binarySizeUnits` never set (S:GitBridge/GitBridge.swift:23, no caller) |
| LFS: "Stored in Git LFS" + "12.3 MB → 12.8 MB · oid 4d4d4d…" + hover copy-object-id button ("Copy object id", notice) | notes.rs:49,61-66,79-82, 133-139 | NO | S:GitBridge/GitMapping.swift:35 | arrives as `GitFailure("Stored with Git LFS: <summary>")` drawn as a one-line chip |
| Too large to load unasked (> 8 MiB patch): title (size) + detail (changed lines) + "Load diff" (primary) + "Reveal in Finder"/"Reveal Current File in Finder" | notes.rs:147-186, diff/mod.rs:209-216,251-258, C:git/diff.rs:34 | NO | GitMapping.swift:36 | failure text "title. detail", no buttons; `loadAnyway: false` hard-coded (RealGitBackend.swift:31,51) |
| Submodule: from→to short hashes (code font), "New commits N" + rows (subject / short hash, author, age), "Commits left behind", "and N more", notes (not checked out, not fetched, inside note) | diff/submodule.rs:23-165 | NO | GitMapping.swift:37-40 | joined into a failure string: "+ abc1234 subject" lines in one chip |
| Image preview | none in GPUI | n/a | — | |

---------------------------------------------------------------------------------------------------

## 3. History tab

| State / item | GPUI file:line | Swift | Swift file | Note |
|---|---|---|---|---|
| Not a repository / "Reading history…" | view.rs:66-86 (text :80-84) | PARTIAL | S:PanelUI/History/HistoryView.swift:66-67 | Swift: "Reading the log…", "This folder isn't in a git repository." |
| git missing / git failed: problem card (same as Changes, headline "History needs git, and it isn't installed.") | problem.rs:42-50, view.rs:67-77 | NO | HistoryView.swift:68 (plain muted note) | |
| Branch deleted / log unreadable: danger caption row under the head ("git's words"), head + picker stay | view.rs:250-256, state.rs:195-198 | PARTIAL (wrong) | HistoryView.swift:68, HistoryStore.swift:57 | Swift replaces the whole card body with a muted note, in the same style as "empty" |
| Empty log | only Head row, no note (state.rs:411-435) | WRONG | HistoryView.swift:77 | Swift invents "No commits yet." |
| Head band "History" + count (grouped) + file-filter chip + branch picker (no prefix; branch mark only when a branch is chosen; label "HEAD" when none) | view.rs:227-248 | PARTIAL | HistoryView.swift:57-63 | Swift label = `model.branch ?? "HEAD"` (shows the current branch name, GPUI shows HEAD); head has no chevron (Section::Empty) ✓; filter chip is a separate row under the head, not in the head |
| File History filter chip: pill "<name>" or "N paths" + X, tooltip "Show all commits", click clears | view.rs:205-223, text key `history-filter` | PARTIAL | HistoryView.swift:111-120 | different shape (row with filter icon + BandButton); same words |
| Log limit | `LOG_LIMIT = 300`, no "show more" | YES (core) | via core `git.log` | no paging in either |
| Commit row: 44 tall, chevron in lead, subject (body, truncates, semibold + selected_text when open), 2nd line: short hash (code font, primary; selected_text when open) · "merge" for merges · author · age at right (tabular) ; tooltip with whole subject only when cut; `aria_expanded` | G-style rows/commits.rs:75-140, view.rs:257-307 | YES | S:PanelUI/History/CommitRowView.swift | chevron is the same glyph rotated (GPUI swaps ChevronRight→ChevronDown); Swift `.help` always; value strings "open"/"folded" |
| Age text "now 12m 3h 2d 5w 4mo 2y"; refreshed by a one-shot timer when a label changes | C:view/panel.rs:101-152, def.rs:36 `time_words` | PARTIAL | S:PanelModel/History/Age.swift, PanelContentView.swift:18 | text yes; `now` is read only when the body re-renders, no timer, labels go stale |
| Click/→ opens commit (accordion: opening another closes the first); ← or click closes | state.rs toggle, view.rs:289-293 | YES | HistoryStore.swift:63-78 | |
| Opened commit block (x 40, caption): heading = full subject (semibold) + "Copy message" button | details_view.rs:67-92 | YES | S:History/CommitDetailsView.swift:27-31 | |
| Message compact (~3 lines of the first paragraph) with "Show more"/"Show less" (chevron), whole view = prose + trailers + "The message goes on: copy it to read all of it." + "too long to read in full here: git show <short> prints it." | details_view.rs:97-145, shown.rs | YES | CommitDetailsView.swift:46-64,147-165 | compaction width measured by Swift's own sampler (CaptionMeasure) vs GPUI `caption_chars` |
| "Reading details…" / red failure text while details load or fail | details_view.rs:105-113 | YES | CommitDetailsView.swift:59-63 | |
| Ref chips (HEAD primary, tags `git.tag`, branches with branch chip), up to 8 then "+N more", wrapping | details_view.rs:285-296, shown.rs:15 | PARTIAL | S:History/CommitRefs.swift | single line HStack, no wrap; chip look differs (filled 15 % vs outlined `status_badge`) |
| Facts under a rule: Author · Committer (if different) · Co-authors (≤ 4) with date + age; "Commit <short> [copy hash][⋯ more copies]"; Parent(s) as links (only those the log lists; ≤ 4 then "+N more"); Changes "2 files +4 −1" tinted | details_view.rs:149-234, shown.rs:141-200 | YES | CommitDetailsView.swift:77-114, CommitFact.swift (72 label column ✓) | parent link has no hover underline (hash_link underlines) |
| Details block right-click / ↩ menu: [Show more/Show less, —] Copy Hash · Copy Short Hash · Copy Message · Copy Subject · Copy Author [—, Open Parent <short>…] | menu.rs:41-72 | PARTIAL | CommitDetailsView.swift:43 (`CommitCopies` only), CommitMenu.swift | no Show more item, no Open Parent in the details menu |
| Commit ROW menu: the five copies ONLY | menu.rs:36-38 | WRONG | CommitMenu.swift:38-44 | Swift adds "Open Parent <short>" to the row menu |
| Copy actions read the log first; message/author fetch details when never opened; notice chips ("Copied hash" …, cut message wording) | copy.rs:63-109 | PARTIAL | S:PanelModel/History/CommitCopy.swift, HistoryStore.swift:82-87 | copies work incl. fetch; no notice chip (`host.copy` posts nothing) |
| Files under the open commit: "Loading files…" / "No file changes." / red files error; rows = same file row at depth 1; click opens the diff over the terminal with the commit as context; right-click path menu (Open/Reveal enabled only if the file is in the working tree now) | view.rs:309-369, state.rs:411-435 | PARTIAL | HistoryView.swift:94-101 | files arrive WITH the details: a details failure leaves "Loading files…" absent and no files error row; menu enable rules (`present`) missing; no commit context in the diff header |
| Palette's "open commit" reaching the tab | history/api.rs, refresh.rs:119-165 | NO | missing | |
| Keys: ↑ ↓ over commits, the opened commit's details row and its files; → ← show/fold files; ↩ on a commit folds, on details opens the menu, on a file opens its diff; ⌘C copies the hash of the commit under the cursor; list keeps keys | G:history/keys.rs:15-111 | NO | S:History/HistoryView.swift:25-28 (`step` only) | Swift arrows walk the OPEN commit's files only; no way to move over commits from the keyboard |
| Tab shows → refresh; tick polling; refresh when window comes to front | def.rs:24-26, mod.rs:121,151 | PARTIAL | HistoryStore.swift:41-43 (git watch) | event-driven (fine) but no on-show refresh |
| Settings | none beyond `size_units` etc. | n/a | | |

---------------------------------------------------------------------------------------------------

## 4. Worktrees (no tab: action + dialog + close question + env)

| State / item | GPUI file:line | Swift | Swift file | Note |
|---|---|---|---|---|
| Entry point: Shell → New Terminal in New Worktree… (keymap action `NewTerminalInWorktree`), bound only when the active terminal has a git status (greyed otherwise); palette | G:worktree/def.rs:49-59 | NO | S:PlatformModel/Resources/keymap.json:78 (id exists) | no handler for the action anywhere in S: (grep `newTerminalInWorktree` finds only the JSON) |
| Command line `new-terminal --worktree <branch> [--command]`: dialog opened and filled, refuses if a dialog is up / not a repo / not allowed | worktree/ask.rs:104-139, api.rs | PARTIAL | S:WindowUI/Hub/WindowHub+Worktree.swift:8-11 | opens the panel tab + sheet (`ask(branch:in:)`); `--command` still refused (parity row X08); the sheet is shown inside the panel's Changes tab (`send(.showPanelTab(.changes))`) |
| Not allowed here: alert "Can't make a worktree here" with the reason (untrusted `.wormhole` project) | ask.rs:76-82,115-118 | PARTIAL (wrong) | S:PanelUI/Stores/WorktreeStore.swift:62-65 | reason goes to `host.notify` = VoiceOver announcement only, nothing visible |
| Allow rule: project folders off, or no `.wormhole/` project in the repo → allowed; project present → needs "Settings" feature trusted ("Trust the project first: a terminal in its worktree would run with the project's settings") | G:project/api.rs:90-106 | WRONG | S:Wormhole/PanelBackendsFactory.swift:52-57 | Swift refuses unless a project exists AND is trusted: every ordinary repository is refused with "Worktrees are made only in a trusted project…" |
| Dialog "New terminal in a new worktree": description names the repo ("A worktree of <repo> on a branch of its own, in a folder of its own, so work there touches nothing in the repository's folder."), label "Branch (new, or one the repository has)", field placeholder "fix/login", caption "Folder: <~ path>" (placeholder `<branch>` before typing), reason in warning colour under the field, footer key hint "esc cancels", Cancel, Create (primary); ↩ = Create | ask.rs:190-246 | PARTIAL | S:PanelUI/Worktree/WorktreeSheets.swift:31-61 | text says "the repository" not its name; "Folder:" line shows only after a plan, absolute path (not `~`); error/refusal in `statusDangerText` (GPUI `warning_text`); Create is disabled until a plan is ready (GPUI keeps it enabled and answers with the reason); sheet 420 wide, heading font vs Kit dialog title |
| Branch plan/normalise (git's rules, folder beside repo or under `worktree_base`) | C:git/worktree plan, ask.rs:259-266 | YES | S:GitBridge/RealWorktreeBackend.swift:33-38 | `worktree_base` / `worktree_clone_build_folders` never passed from settings (`makeBackends(worktreeBase: nil, cloneBuildFolders: false)`; `setOptions` has no caller) |
| Trust re-checked at Create | ask.rs:249-261 | YES | RealWorktreeBackend.swift:40-42 | |
| After Create: terminal opens beside the selected one in the new folder; the project's `worktree.setup` and the asked `command` are pasted as one line (`setup && command`) and submitted; own ports in the env; worktree recorded in the app | ask.rs:331-362 | PARTIAL | S:Wormhole/PanelBackendsFactory.swift:40-41 | terminal opens (`.newTerminalIn`); no env/ports ("not passed on yet"), no setup line, no command |
| Git refuses Create: alert "Couldn't create the worktree" with git's words (dialog closed) | ask.rs:364-367 | PARTIAL | WorktreeStore.swift:84-92 | Swift keeps the sheet up with the text under the field |
| Close question for a terminal in a worktree Wormhole made: note "Its worktree <branch> at <~path> was made by Wormhole. Close keeps it; Close and Remove Worktree deletes its folder (the branch stays)." / "It has uncommitted changes, so it stays."; extra button "Close and Remove Worktree" (not when dirty) | G:worktree/close.rs:37-56 | NO | S:Stores/WorktreeStore.swift:95-107 | `askRemoval()` has no caller (only tests); `dirty` is never set; the Swift "Remove the worktree?" alert (WorktreeSheets.swift:21-26) is not a GPUI surface |
| Same for closing a GROUP (each worktree once, removable only if no changes and no terminal outside the group in it; plural wording) | close.rs:61-107 | NO | missing | |
| Removal runs after the terminal closes; git refusal shown in the front window "Couldn't remove the worktree" | close.rs:111-131 | PARTIAL | WorktreeStore.swift:102-106, RealWorktreeBackend.swift:56-63 | failure only announced, not shown |
| Spawn env: a terminal in a worktree Wormhole made starts with its ports | def.rs:38-44 | NO | missing | |
| Worktree only written on user action, never in untrusted project, never forced, branch stays | module doc ask.rs/mod.rs | YES | RealWorktreeBackend.swift:1-6 | (subject to the allow-rule bug above) |

---------------------------------------------------------------------------------------------------

## 5. Cross-cutting

### Settings (how GPUI reads them vs Swift)

| Key | GPUI | Swift |
|---|---|---|
| `changes_tree`, `group_untracked` | read per frame (`sync_list`), the view menu writes them (`update_settings`) | NO: local `ChangeRows` only |
| `diff_split`, `diff_wrap` | read per frame; header buttons write them | NO: `DiffSession.split/wrap` local; wrap default true vs false |
| `tab_width` | `BuildKey.tab_width` rebuilds rows | NO: constant 4 |
| `code_font_family` / `code_font_size` (empty → terminal font; 13) | `crate::fonts::code_font_in` in diff, tooltips, details | NO: `\.codeFont` is never set outside PanelUI (grep) → always 13 pt system mono (S:PanelUI/Shell/CodeFont.swift:10-12) |
| `size_units` | diff notes' sizes | NO: `binarySizeUnits` never passed; sizes not drawn anyway |
| `icon_theme` | file icons | YES (`fileIcons` env) |
| `sidebar_changes*` | sidebar | PARTIAL (one of five read) — not the panel |

### Motion
- GPUI features/git: none. Panel rows are static; selection is an instant fill.
- Swift adds: list animation + row insert transition (ChangesListView.swift:23,36), card `.transition(.opacity)` (:38), selection fill that slides between rows (RowSurface.swift:40-47), detail pane transition and `.animation(motion.fold)` (SelectableListSplit.swift:30,33), History accordion `withAnimation` + `.transition(motion.hunk)` (HistoryView.swift:81,103), review bar move transition (ReviewBar.swift:22), diff content `.transition(.opacity)` (DiffView.swift:70). Reduce Motion fallback exists (PanelMotion.swift). Needs a decision: either confirm against the GPUI motion table or cut.

### VoiceOver / spoken
- GPUI: tab `spoken::panel_tab`, band `section_band(title, n, Files)`, commit row `Role::Button` + expanded, details `Role::Group` with `spoken::commit_details`, parent link `history_open_parent`, submodule row label, LFS/copy buttons by tooltip, list role `Role::List` named after the tab with the cursor row as active descendant (focus/panel.rs:182-200).
- Swift: labels on rows/bands/commit rows/details (`details.spoken`), parent links (`parent.spoken`), list label ("Changed files"/"Commits"). Missing: tab spoken text, list "active descendant" for the cursor, submodule label, "no files" wording.

### Notices (chips)
GPUI posts `Copied::Path/Error/CommitHash/…/ObjectId` and `Failed::CopyCommit` (path_menu.rs:33, problem.rs:124, copy.rs:128-131, notes.rs:137). Swift `host.copy` posts nothing (S:WindowUI/SystemPanelHost.swift:24-27). NO.

---------------------------------------------------------------------------------------------------

## MISSING or WRONG in Swift — prioritized

Size: S = under a day of one file, M = a feature in a few files, L = needs a new FFI return/model plus UI.

P1 — wrong behaviour (fix first)
1. Worktree allow rule refuses every ordinary repo (needs a trusted project; GPUI allows when flag off or no `.wormhole/`). S:Wormhole/PanelBackendsFactory.swift:52-57. **S**
2. Review "Paste into Terminal" never checks bracketed paste (`SystemPanelHost.paste`/`WindowHub+PanelHost.swift:14-18` always true): a plain shell would run each comment line. Needs a `bracketedPaste` read from the terminal view and the clipboard fallback + "Copied the comments instead" alert already in `ChangesView`. **S–M**
3. A failed refresh throws the whole list away and shows plain text (ChangesStore.swift:60-61, ChangesListView.swift:19); needs the problem card (git missing, dubious ownership, generic) with Retry / Copy error, the "Couldn't refresh: …" line over a kept list, and the compare-missing banner (`GitRepoState.compareMissing` already arrives). Same for History (error row in danger colour under the kept head). Needs typed GitError from the FFI. **M**
4. Open diff is not kept in step: edited file never re-read, file leaving the list closes the diff instead of the "No longer changed" / "Out of date" badges (ChangesStore.swift:59). **M**
5. Untracked grouping ignores compare (`untracked_apart`) — ChangeRows.swift:21-25. **S**
6. Wrong text where GPUI has none: "No differences from <rev>." (ChangesListView.swift:30) and "No commits yet." (HistoryView.swift:77); row menu has "Open Parent" (CommitMenu.swift:40-43); ‹ › clamp instead of wrap (ListNavigation.swift:6-10); `wrap` defaults true (DiffSession.swift:17). **S each, S together**
7. Status badge look: outlined in colour, not 15 % fill (DiffHeader.swift:78-81, CommitRefs.swift:33-35). **S**

P2 — missing features
8. Diff special cards: too-large (Load diff, Reveal), LFS (pointer summary, copy oid), submodule log (sections of commits), binary sizes, per-column notes (New file / Deleted / Binary / LFS / Empty file / No content changes), mode-change badge, "No highlighting" badge. Today all arrive as one `GitFailure` string (GitMapping.swift:35-40). Needs a `DiffOutcome` return and `loadAnyway`. **L**
9. Keyboard model: one ⇥ stop per section, ↑ ↓ over bands/dirs/files and over commits, → ← fold/open (← to parent), ↩/Space press, a visible cursor ring, diff follows the cursor only while open, ⌘C copies a commit's hash, ↩ on details opens its menu. Swift only steps files and opens the diff each time (SelectableListSplit.swift:44-48). Core already has `walk`/`list_key` in `view::focus` (FFI exists: apple/ffi/src/focus.rs). **L**
10. Settings never read or written: `changes_tree`, `group_untracked`, `diff_split`, `diff_wrap`, `tab_width`, `code_font_*` (env not set), `size_units`, `worktree_base`, `worktree_clone_build_folders` (GitBridge.makeBackends args and `setOptions` have no caller). View-options menu and diff header toggles must write config. **M**
11. Worktrees: menu/palette action and its enable rule, the close question (note + "Close and Remove Worktree" for a terminal and for a group, dirty/shared rules), spawn env with ports, project `worktree.setup` + command line after create, visible "Can't make a worktree here" / "Couldn't create/remove" alerts (now VoiceOver only), repo name in the description, `Folder:` line with `~` and `<branch>` placeholder. **L**
12. Multi-select: ⌘-click marks, multi-file path menu wording (`Copy Paths`, `History of These Files`), folder-row menu. **M**
13. Open in editor everywhere: `editorName` is nil so the menu item never shows (SystemPanelHost.swift:21), no row hover icon, no header button, no per-line icon; Reveal/Open must be disabled for deleted files; History commit files need the `present` rule. **M**
14. Submodule row (folder icon, "enter" button that opens a terminal in the submodule, label, no file menu) and nested-repo "dir/" rows. **M**
15. Diff header: path crumb (not only the name), "File N of M" always, commit short hash as context for History diffs, Open-in-editor button, wrap shown as selected, wrap button whenever split fits. **S–M**
16. Picker wording/hints: "HEAD (uncommitted changes)" / "HEAD (current branch)", hints "Loading branches…" / "Changes since this branch split off from it", History chip label "HEAD", popover width 320, close on tab change. **S**
17. View options menu structure: `List` then `Tree` as check items, separator, "Group untracked files separately". ViewOptionsMenu.swift:11-19. **S**
18. Notice chips for every copy (Copied path/hash/message/object id, Failed copy). Needs the window's notice call in `SystemPanelHost.copy`. **S–M**

P3 — look and detail
19. Name/folder elision with the real rule (`elide::middle` keeping extension, folder at front sized to the remaining room, measured) — FileRowView.swift:28-37. Could come through the FFI (`view::elide`). **M**
20. Diff look: wrap rows a full row tall per wrapped line, whole-column hatch for new/deleted file, hunk header limited to the old column in split, divider through empty space, sideways scroll with fixed gutters + edge fades + position bar, gutter width by 0.6 em rule or keep real advance and record it. **M**
21. History: refs and parents wrap (flex_wrap), details menu with "Show more/less" and parents, files error row, age labels refreshed by a one-shot timer (`next_age_change`), filter chip inside the head band. **S–M**
22. Tooltips only when the path was cut or a rename (Swift `.help` always); nested "about" line format uses core words. **S**
23. Motion: Swift animates things GPUI does not in these tabs (list inserts, selection slide, detail pane, accordion). Decide: keep or cut. **S**
24. VoiceOver: tab spoken text, list active-descendant for the cursor, "no files" wording, submodule row label. **S–M**
25. Loading note without head and with GPUI words ("Reading git status…", "Loading diff…", "Reading history…"), not-a-repository words ("This terminal's folder isn't inside a git repository."), band spoken "no files". **S**

## Metrics where Swift visibly differs (both sides quoted)

| Thing | GPUI | Swift |
|---|---|---|
| Status badge | outlined, no fill: `badge(ui).px_2().outlined(ui, color)` (G-style/labels.rs:49-51) | filled: `.background(color.opacity(0.15), …)` (S:PanelUI/Diff/DiffHeader.swift:80) |
| Ref chips in details | `style::status_badge(ui, ui.primary, "HEAD")`, `ui.tag` (details_view.rs:290-291) | 15 % fills (CommitRefs.swift:16-17,35) |
| Details warning/error text | refusal line `ui.warning_text` (ask.rs:233), history/files errors `ui.danger_text` (view.rs:254,311) | refusal `statusDangerText` (WorktreeSheets.swift:45) ; history failure muted (HistoryView.swift:68) |
| Picker popover | `menu_width(px(320.0))` (picker.rs:273) | `.frame(width: 260, …)` (RevPicker.swift:90) |
| Diff default wrap | `diff_wrap: false` (C:config/settings/mod.rs:347) | `public var wrap = true` (DiffSession.swift:17) |
| Wrapped diff line height | `.line_height(px(style.row_height))` (render.rs:203) | natural line height, `minHeight` only (DiffRowView.swift:35,120) |
| Gutter width | `ceil(digits*font*0.6 + 20)` (render.rs:177-179) | `ceil(digits*glyphAdvance + 20)` (DiffStyle.swift:18) |
| Hunk header | full text row in old column when split (render.rs:206-211) | full-width row (DiffRowView.swift:21-24) |
| Header badge/controls spacing | `gap_2`, `pr_2` (header.rs:152-155) | `PanelMetrics.gap` 8, no trailing pad (DiffHeader.swift:24) |
| Commit/file/band metrics (rows 28, band 32, two-line 44, inset 8, indent 24, radius 8, lead 16, kind letter 12) | scale.rs:27,36,25,271 | same values (PanelMetrics.swift, Metric.swift:13-27) — YES |
| File-row mark for moved | `IconName::FolderInput` at 12 (rows/files.rs:80) | `folder.badge.gearshape` 12 (FileRowView.swift:26) |
| Review editor | Kit dialog, textarea auto-grow 3–10 rows (review.rs:30,91) | popover 340 wide, fixed 4 rows (CommentEditor.swift:35,54) |
| New-worktree dialog | Kit dialog; key hint `style::key_hint(ui,"esc","cancels")` (ask.rs:289) | sheet 420 wide with `KeyCap` + text (WorktreeSheets.swift:50-52,58) |
