# S-panel checklist: right-panel shell (P01), Files tab, Usage tab

Read-only comparison of the GPUI app (design truth) with the Swift app. Every Swift claim was checked by reading
the Swift file named. Path prefixes:

- `app:` = `crates/wormhole/src/` (GPUI app)
- `core:` = `crates/wormhole-core/src/`
- `sw:` = `apple/swift/Sources/`
- `ffi:` = `apple/ffi/src/`

Status: YES = same behaviour, PARTIAL = exists but differs, NO = missing.

Structure on the Swift side: the tab list, the open tab, the panel width and the columns come from core
(`ChromeSnapshot.tabs`, `panel_tab`, `Panes`, read in `sw:WindowUI/Hub/WindowHub+Focus.swift:7`). That is the right
seam, but core's `tab_enabled` (`core:workspace/panel.rs:12`) only asks "is the feature on". GPUI also asks the
tab's own `shown` rule (`app:window/feature_host.rs:271`), which core does not know. That one gap explains the
Usage and Project rows below.

---

## 1. Right-panel shell (P01)

### 1a. Tabs, order, hiding

| State / interaction | GPUI file:line | Swift | Swift file | Note |
|---|---|---|---|---|
| Six tabs, order Changes, History, Notes, Usage, Project, Files | `core:config/tab.rs:36` (`TabId::ALL`) | YES | `sw:PanelModel/Tabs/PanelTab.swift`, `sw:WindowUI/PanelTabBar.swift:106` (`barOrder`); live list from core via `hub.panelTabs` | Same order. |
| Tab hidden when its feature is off | `core:config/feature_table.rs:92-157` (`FEATURES`), `app:window/feature_host.rs:271` | YES | `sw:WindowUI/Hub/WindowHub+Focus.swift:7` (`filter(\.enabled)`) | Core's `set_feature` (`core:workspace/panel.rs:36-46`) falls back to Changes when the open tab's feature goes. |
| Tab hidden by release flag (Notes `NotesAndTasks`, Project `ProjectFolders`, Files `Files`) | `core:config/feature_table.rs` `gate:` fields | YES (through core mask) | same | Menu lines hidden by flag: `sw:WindowUI/Menu/MenuStateMaker.swift:68` (flags only, not the person's own switch for an optional feature). |
| Usage tab offered only while a coding agent is detected in a terminal on screen | `app:features/usage/def.rs:36` (`shown: agent_on_screen`), `app:window/facts.rs:120` | NO | missing | Swift always shows Usage while the feature is on (always on). Needs an "agent on screen" fact. |
| Usage remembers the tab it came from and goes back when the agent leaves | `app:features/usage/def.rs:43` (`returns_to_previous`), `app:window/focus/bars.rs:77` (`leave_missing_tab`) | NO | missing | Depends on the row above. |
| Usage menu line (Ctrl-Cmd 4) hidden while no agent is on screen | `app:features/usage/def.rs:47-51` | NO | missing | `MenuStateMaker.swift` hides by flag only. |
| Project tab offered only while the terminal has a folder (stays while it is the open tab) | `app:features/project/def.rs:36-41` | NO | missing | Swift always shows Project when the feature is on. |
| Ctrl-Cmd 1..6 select tabs | `core:keys/keymap/table.rs:183-188` | YES | `sw:Wormhole/Actions/ActionRouter+View.swift:22-27`, `sw:PaletteModel/Shortcuts.swift:21-32` | The P01 evidence says a real key press was not injected. A request for a disabled tab: core refuses (`core:workspace/panel.rs:25`), GPUI falls back to Changes (`app:window/panel.rs:96-107`); nearly the same. |
| Key of a tab whose feature is off is ignored, its menu line hidden | `core:config/feature_table.rs:158-165` (`hidden_actions`) | YES | `MenuStateMaker.swift:68` | |
| Saved layout holds a tab whose feature is off: panel draws Changes | `app:window/panel.rs:29-31` | PARTIAL | `sw:WindowUI/PanelBody.swift:25` (`controller.tab = PanelTab(rawValue: tabName)`) | Swift draws whatever core reports as open; relies on core never reporting a disabled tab. Unverified on a restored layout. |
| Unknown saved tab name reads as Changes | `core:config/tab.rs:11-24` | YES | `sw:WorkspaceBridge/WindowMapping.swift:16-17` (core `TabId`) | Done in core. |

### 1b. Tab bar look

| State / interaction | GPUI file:line | Swift | Swift file | Note |
|---|---|---|---|---|
| Bar is a 38 pt title row, tabs from 6 pt, 4 pt apart, room on the right for the panel toggle | `app:window/chrome.rs:232-262`, `app:style/scale.rs:58`, `chrome.rs:34-39` (`TITLE_ROW_PAD` 6, `PANEL_TOGGLE_SLOT`) | YES | `sw:WindowUI/Chrome.swift:35-42,52,55`, `sw:WindowUI/PanelTabBar.swift:27,36-38` | Numbers equal. |
| Tab = 28 pt square (`CONTROL_H`), radius `ui.radius`, min width 28, `px_2` and `gap_1` with a count | `app:style/controls.rs:72,100-103` | YES | `PanelTabBar.swift:61-63` (`Chrome.iconTab` 28) | |
| Tab icon size 1 rem (16 pt) | `app:style/icon_motion/tab_glyph/mod.rs:76,83` (`Size::Medium` is a rem) | NO (14 pt) | `PanelTabBar.swift:96` (`Metric.glyph` 14) | WRONG by 2 pt. |
| Icons: Changes git-compare, History git-commit-horizontal, Notes file-text, Usage gauge, Project folder, Files folder-tree | `app:features/git/changes/def.rs:37`, `history/def.rs:30`, `notes/def.rs:27`, `usage/def.rs:30`, `project/def.rs:31`, `files/def.rs:25` | PARTIAL | `PanelTabBar.swift:107-118` | Swift: Changes `arrow.triangle.pull`, History custom `CommitGlyph` (ok), Notes `doc.text`, Usage SF gauge, **Project `folder.badge.gearshape` (GPUI: plain Folder)**, **Files `folder` (GPUI: FolderTree)**. Its comment ("folder-cog") is wrong. The Lucide SVGs should be bundled. |
| Selected: `ui.selected` fill, icon `selected_text`; hover `band_hover` with text colour; pressed `ui.active` | `app:style/controls.rs:96-103` | PARTIAL | `PanelTabBar.swift:66-72,92-97` | Selected and hover yes; **no pressed state**. |
| Selection is one shape that springs between tabs (400/30) when Motion is on | `app:window/chrome.rs:274,295-302`, `core:view/motion/tabs.rs:19` | YES | `PanelTabBar.swift:23,48-50,69` (`matchedGeometryEffect`, spring 400/30) | Reduce Motion cuts it. |
| The active tab's icon plays a small motion once when it becomes active | `app:style/icon_motion/tab_glyph/mod.rs:73-87` | NO | missing | Per-icon motions for Changes, History, Gauge, FileText, Folder, FolderTree. |
| Count badge only on Changes (changed-file count, hidden at 0); `999+` cap; selected badge tint | `app:features/git/changes/def.rs:47`, `app:style/labels.rs:30-44` | PARTIAL | `sw:PanelUI/Shell/TabCountBadge.swift`, `sw:WindowUI/RightPanel.swift:44-49`, `sw:PanelUI/Controller/PanelController.swift:57-64` | Badge itself matches. **Swift adds a count on Usage (agents that reported); GPUI Usage has `count: None`** (`usage/def.rs:37`). Remove. Notes, History, Project, Files have none on both sides. |
| Tooltip: label plus its key ("Changes ⌃⌘1") | `app:window/chrome.rs:279` (`keymap::with_key`) | PARTIAL | `PanelTabBar.swift:87` (`.help(tab.title)`) | No key in the tooltip. |
| Buttons the open tab brings sit beside the tabs (Notes: stash prompt, an icon-tab with the Inbox glyph) | `app:window/chrome.rs:263-265`, `app:features/notes/view.rs:404-430` | NO | `sw:PanelUI/Notes/StashPromptButton.swift` exists but is **not placed anywhere** (no use found) | The action is still reachable by its menu key (`ActionRouter+View.swift:28`). |
| Drag the window by the bar's empty part | `app:window/chrome.rs:258` (`title_drag`) | YES | `PanelTabBar.swift:52` (`WindowDragArea`) | |
| Content slides in 10 pt from the side of the old tab, 140 ms, ease-out; old content gone at once; Reduce Motion keeps the fade | `app:window/panel.rs:51-66`, `app:window/panel_switch.rs`, `core:view/motion/tabs.rs:21-80` | YES | `sw:WindowUI/RightPanel.swift:17-18,28-33,59-64` | Swift orders by `PanelTab.allCases`, GPUI by the *offered* tab list; the side can differ while a tab is hidden. S. |

### 1c. Show / hide, slide, width, handle

| State / interaction | GPUI file:line | Swift | Swift file | Note |
|---|---|---|---|---|
| Panel toggle: one glyph, tooltip says what a click does, a dot while wanted without room | `app:window/chrome.rs:121-157` | YES | `sw:WindowUI/WindowControls.swift:34-39,46-50`, `Chrome.swift:49-62` | |
| Toggle tooltips show the keys Ctrl-Cmd S (sidebar), Option-Cmd I (panel) | `core:keys/keymap/table.rs:175-177`, `app:window/chrome.rs:116-129` | NO (wrong keys) | `WindowControls.swift:44,49` hard-codes **"(⌘B)" and "(⌥⌘B)"** | Wrong text. S. |
| Zen mode: the button says "Zen mode hides the right panel", a click leaves Zen | `app:window/chrome.rs:112-114`, `app:window/layout.rs:78-86` | YES | `WindowControls.swift:33-36,47`, `sw:WindowUI/WindowBody.swift:37-39` | |
| Slide: in 250 ms ease-out over the terminal, out 200 ms ease-in, terminal resized once at the end; Reduce Motion cuts | `app:window/slide.rs:1-60`, `core:view/motion/pose/column` | YES | `sw:WindowUI/Columns/ColumnSlide.swift`, `sw:WindowUI/SlideGeometry.swift` | Classic variant only; GPUI also has Soft/Content variants chosen in settings (not checked). |
| Panel yields first in a narrow window (down to 240, then hidden, comes back by itself) | `core:view/layout/mod.rs:14-30,60-90` | YES | `sw:WindowUI/Columns/ColumnFit.swift:19-35` (core columns) | |
| Width 360 default, range 240..720, chosen width kept while hidden or yielding | `core:view/layout/mod.rs:19-22` | YES | `ColumnFit.swift` (`panelChosen`, `limits`) | |
| Width and open state remembered per window and app-wide for new windows | `app:window/layout.rs:29-31`, `core:view/layout/remembered.rs` | YES | core state (P01 evidence: tab survives restart; dragged width across restart unverified) | |
| Open tab remembered per window (not per terminal) | `app:window/saved_layout.rs:18` (`right_tab`) | YES | core `panel_tab` | |
| Drag handle: 8 pt wide straddling the edge, resize cursor | `app:window/resize.rs:107-132`, `app:style` (`RESIZE_HANDLE_W` 8) | YES | `sw:WindowUI/Columns/ColumnHandle.swift:21-24`, `Chrome.swift:46` | |
| Handle hover: after a short delay a 2 pt line (`resize_hover`), solid line (`resize_active`) while dragging | `app:window/resize.rs:19,113-120,134-142` | NO | `ColumnHandle.swift:21` is `Color.clear` | Only the divider hairline shows. |
| Readout beside the pointer ("430 · max", "Close") | `app:window/resize.rs:150-` | YES | `ColumnHandle.swift:50-66`, `sw:WindowUI/Columns/WindowBody+Drag.swift` | Words from core. |
| Drag past min + 70 sticks then closes on release; dragging back reopens | `core:view/layout/resize.rs`, `core:view/layout/mod.rs:27` | YES | `WindowBody+Drag.swift:41-70` | |
| Double-click the handle resets the width | `app:window/resize.rs:48-56` | YES | `ColumnHandle.swift:29` | |
| Esc during a drag puts the width back | `app:window/resize.rs:90-100` (`cancel_resize`) | NO | missing (no cancel path in `sw:WindowUI`) | S. |
| VoiceOver / keyboard width steps on the handle | not in GPUI | extra | `ColumnHandle.swift:30-41` | Harmless extra. |

### 1d. Focus, keys, VoiceOver

| State / interaction | GPUI file:line | Swift | Swift file | Note |
|---|---|---|---|---|
| PanelTabs and PanelContent are two focus stops, skipped while the panel is hidden; F6 / Shift-F6 / Tab order | `core:view/focus/mod.rs:15-93`, `core:view/focus/tab.rs` | YES | `sw:WindowUI/Focus/WindowFocus.swift:64-75,128-139` (core rules `Focus.rules`) | |
| Left / Right on the tabs switch tab at once, wrapping; Return, Space and Down go into the list | `app:window/focus/bars.rs:109-130` | YES | `sw:WindowUI/Focus/WindowFocus+Keys.swift:15-29,52,64,77-81` | Return / Enter / Space are one case (line 28), Down on the tabs presses (line 27). |
| Focus ring on the active tab while the keys are on the bar | `app:window/chrome.rs:285-287` | YES | `PanelTabBar.swift:73` | |
| A list tab's rows: one stop per section; ring on the cursor row | `app:window/focus/panel.rs:44-62`, `app:features/files/view.rs:73` (`panel_cursor` ring) | PARTIAL | `sw:PanelUI/Shell/SelectableListSplit.swift`, `sw:PanelUI/Controller/PanelKeyboard.swift` | Swift has one stop for the list but **draws no ring on the cursor row** (no `focusRing` in `sw:PanelUI`); only the selected fill. |
| Esc ladder: popup, then detail (diff / viewer / find bar), then terminal | `core:view/focus/escape.rs:27-37`, `app:window/focus/mod.rs:188-228` | PARTIAL | `sw:WindowUI/Focus/WindowFocus.swift:146-168` | `popup:` is always `false` (line 151); menus and the rename field take their own Esc. |
| VoiceOver: tab list "Right panel"; each tab role Tab, selected, position in set; label with counts in words ("Changes, 3 changed files", "Notes, 2 open next steps, 1 note", "Changes, no changes") | `app:window/chrome.rs:267-307`, `core:view/spoken.rs:88-110` | PARTIAL | `PanelTabBar.swift:30-35,56-61` | Swift: button with a selected trait (not Tab), label = bare title, value "2 of 6, 3". Count words missing. |
| Panel content is a List named after the tab | `app:window/focus/panel.rs:187-201` | YES for Files | `SelectableListSplit.swift:51` | Notes and Usage have no walk on either side. |
| Panel with no terminal | a window always has one; Files returns an empty div (`app:features/files/view.rs:62`) | YES | Files shows its "no folder" note | n/a |

---

## 2. Files tab

State layout: GPUI has one `Files` entity per window and **a `FilesOf` per terminal** (tree, open folders, selection, folder chosen by hand) (`app:features/files/mod.rs:84-130`). Swift has **one `FilesStore` per window** (`sw:PanelUI/Controller/PanelController.swift:51`; `files.follow(folder:)`), which clears the tree whenever the folder changes (`sw:PanelUI/Stores/FilesStore.swift:67-79`).

### 2a. Tree and rows

| State / interaction | GPUI file:line | Swift | Swift file | Note |
|---|---|---|---|---|
| Per-terminal tree (open folders, selection, chosen folder survive switching terminals) | `app:features/files/mod.rs:84,109`; `selected()` at `mod.rs:157-160` clears only viewer and rename | NO | `FilesStore.swift:67-79` | Switching to a terminal in another folder rebuilds from scratch; the chosen folder is global to the window. M. |
| Card: path head then rows, 4-pt `PANEL_LIST_TOP`, card radius 12 | `app:features/files/view.rs:60-80`, `app:style/scale.rs:96` | YES | `sw:PanelUI/Files/FilesView.swift:31-45`, `sw:PanelUI/Shell/PanelMetrics.swift:9-14`, `PanelCard.swift:9-19` | Equal numbers. |
| No folder yet: card with "This terminal hasn't said which folder it is in yet." | `app:features/files/view.rs:56-67`, `files.ftl` | YES | `FilesView.swift:34-35` (`EmptyNote`; px 12 py 8 equals `app:style/labels.rs:183`) | |
| Lazy: a folder is read only when opened (root first), one read per folder at a time, stamps skip unchanged folders, off the main thread | `app:features/files/read.rs:30-135`, `keys.rs:60-70` | PARTIAL | `FilesStore.swift:84-96` (`reload` relists every wanted folder each time, no stamp, no in-flight guard); `sw:FilesBridge/Files/RustFilesBackend.swift:18-24` (`offMain`) | Reads are lazy per opened folder and off-main. A hung mount can pile up reads. S. |
| "Reading…" note while a folder loads | `app:features/files/row.rs:117-129` | YES | `sw:PanelUI/Files/FileEntryRow.swift:62-79` | |
| Notes: "Empty folder", "N more" / "N+ more", gone / denied / unreadable | `row.rs:117-129`, `core:view/file_tree/tree.rs` | YES | `FileEntryRow.swift:68-79`, `sw:PanelModel/Files/FileTree.swift:111-135` | Words equal (`files.ftl`). |
| Caps: 5,000 entries per folder (`ENTRY_CAP`), reader hard limit 200,000 | `core:view/file_tree/dir.rs:11`, `core:storage/files/listing.rs:23` | YES | `ffi:files_panel/files.rs:40-43` (core `Dir::from_listing`) | |
| Order: folders first, natural order, names cleaned of control and bidi characters | `core:view/file_tree/dir.rs`, `names.rs` | YES | same core call (`sw:FilesBridge/Files/FilesMapping.swift:6-12`) | |
| Hidden (dot) files and ignored files | all shown, no `.gitignore` filter (`.git` is listed) | YES | same | Neither side hides or marks them. |
| Row: min 28, gap 8, indent `CARD_INSET + steps*24` (folder: depth, file: depth+1, note: depth+2), radius 8, name body text, selected semibold | `row.rs:30-76`, `app:style/scale.rs:27,52,91`, `style/rows/mod.rs:31-44` | YES | `FileEntryRow.swift:23-39,59-61`, `sw:DesignSystem/Scale/Metric.swift:13,25-27`, `Radius.swift:19` | Equal. |
| Folder chevron in a 16-pt lead; folder icon from the icon theme (open / closed variants); a link has the link mark | `row.rs:50-78`, `app:style/rows/mod.rs:49` | PARTIAL | `FileEntryRow.swift:30,43-48`, `sw:PanelUI/Shell/FileIcon.swift:23-45` | Link uses SF `link` (GPUI Lucide `Link2`); fallbacks are SF `doc` / `folder` (GPUI Lucide `File` / `Folder`). |
| File icon theme in force (the person's choice in settings, including none) | `app:features/files/view.rs:96` (`SettingsStore.icons`), `core:theme/icon_theme/` | NO | `sw:WindowUI/PanelBody.swift:17,23` (`static let icons = BundledIconTheme()`) | Swift always draws the bundled theme. M. |
| Selected row: `selected` fill, `selected_muted` icons | `row.rs:49` | YES | `FileEntryRow.swift:21` | |
| Tooltip = the name | `row.rs:78` | YES | `FileEntryRow.swift:30` | |
| VoiceOver: "name, folder, expanded / collapsed", "name, link", expanded state | `row.rs:57-62`, `files.ftl:56-65` | YES | `FileEntryRow.swift:26-29,52-57` | |
| Click: folder selects and toggles, file selects and opens the viewer, link selects only | `app:features/files/view.rs:118-130` | YES | `FilesStore.swift:104-113` | |
| Virtualised rows | `app:style/panel_list.rs:1-9` | NO | `sw:PanelUI/Shell/PanelCard.swift:23-33` (`ScrollView` + `VStack` + `ForEach`) | Up to 5,000 views per open folder built eagerly. M. |
| Scroll edge: hairline under the row above while scrolled; system scrollbar style | `app:style/panel_list.rs:25-41` | NO | `PanelCard.swift:32` | |
| Room under the last row (16 pt) | `view.rs:100` | YES | `PanelMetrics.swift:25` | |

### 2b. Path head, path menu, pins

| State / interaction | GPUI file:line | Swift | Swift file | Note |
|---|---|---|---|---|
| Path head: 32 pt button, icon, name, chevron-down, hover fill, tooltip = the path, VoiceOver "name, folder, opens the path menu" | `app:features/files/head.rs:41-64` | YES | `sw:PanelUI/Files/FilesPathButton.swift:15-33` | Chevron is a 9-pt SF glyph; GPUI `ChevronDown` xsmall. |
| Head name and path-menu name for `/` come from the system's disk name | `head.rs:31-34` (`boot_volume_name`) | NO | `FilesStore.swift` `pathMenu`: `volume: "Macintosh HD"` hard-coded | Wrong on a Mac whose disk is named otherwise. S. |
| Path menu: the folder (checked) and each parent with Finder icons, separator, the pins, Pin / Unpin "“name”", Choose Folder…, Follow Terminal when a folder was chosen | `app:features/files/menus.rs:127-165` | YES | `FilesPathButton.swift:36-55`, `sw:PanelModel/Files/PathMenuModel.swift` | Includes twin-name disambiguation and the share-icon rule (`PathMenuModel.onBootVolume`). |
| Pins live in `config.json` `files_pinned`, per **window profile** | `app:features/files/pins.rs:11-24` | PARTIAL | `ffi:files_panel/files.rs:67-76` (`Config::load().settings`) | Swift reads and writes one list, not the window's own profile. S/M. |
| Choose Folder… = system panel, folders only, confirm button "Choose" | `menus.rs:90-104` | PARTIAL | `FilesStore.swift:55` (`host.chooseFolder()`) | The "Choose" button word is unverified. |
| Keys on the head: a Tab stop (a band), Return opens the path menu under it | `app:features/files/keys.rs:23-26,69-77` | NO | `FilesView.swift:26-28`, `SelectableListSplit.swift` | The head is not part of the key walk. |
| Pins follow a folder rename | `app:features/files/rename.rs:208-211` (`move_pins`) | PARTIAL | `ffi:files_panel/files.rs:91` moves them in core, but `FilesStore+Rename.swift:25-37` never reloads `pins` | `isPinned` stays stale until the root changes. S. |

### 2c. Row menu and Open in

| State / interaction | GPUI file:line | Swift | Swift file | Note |
|---|---|---|---|---|
| Order: Open in ▸ (submenu), Reveal in Finder, Copy Path, separator, Rename…, separator, Pin / Unpin Folder (folders only) | `app:features/files/menus.rs:176-224` | PARTIAL | `sw:PanelUI/Files/FileRowMenu.swift:13-33` | Swift order: Open in editor, Reveal, Copy Path, (folder) Pin, Rename. |
| "Open in" is a submenu of every app the Open-in list offers (icons, the chosen app checked); folders may use any app, files only known editors and Finder; a link gets the submenu disabled | `menus.rs:72-82,187-206`, `app:features/files/open.rs:55-65` | NO | `FileRowMenu.swift:14-16` | Swift has one line "Open in <editor>", only for files; none for folders or links. Its key `files-row-open-in-editor` is not in `files.ftl` (which has `files-row-open-in`). M. |
| Reveal in Finder (shows, never opens an `.app` folder) | `open.rs:96-99` | YES (host) | `FileRowMenu.swift:18` | |
| Copy Path: refuses a path with a control character and says so; otherwise copies and VoiceOver says "Path copied" | `open.rs:108-123` | PARTIAL | `FileRowMenu.swift:19-23` | No control-character refusal; "Path copied" is shown as a chip (`host.notify`), GPUI only announces it. S. |
| Open runs off the main thread, link-safe (`resolve_path_within`) | `open.rs:37-62` | YES (host side) | host / FFI | Not re-verified. |
| Right-click on a row opens the menu; note rows have none | `view.rs:105-114` | YES | `FileEntryRow.swift:33` | |

### 2d. Rename

| State / interaction | GPUI file:line | Swift | Swift file | Note |
|---|---|---|---|---|
| Rename… turns the name into the app's text field, name selected without extension (`stem_len`: last extension only, `.gitignore` all, folders all) | `app:features/files/rename.rs:73-110`, `core:view/file_tree/names.rs` | YES | `sw:PanelUI/Files/RenameField.swift:28-34`, `sw:PanelModel/Files/FileNames.swift:7-12` | |
| Field: `xsmall` input, VoiceOver "New name, text field" | `rename.rs:218-230` | YES | `RenameField.swift:20-26` | |
| Return commits; the same name only closes | `rename.rs:127-143` | YES | `sw:PanelUI/Stores/FilesStore+Rename.swift:20-24` | |
| Swift trims spaces before committing | not in GPUI (typed value goes to core, which validates) | WRONG | `FilesStore+Rename.swift:22` (`.whitespaces`) | S. |
| Esc or losing the keys cancels, unless the rename is already running (`busy`) | `rename.rs:35-37,115-126` | PARTIAL | `RenameField.swift:22-23` | No busy guard. S. |
| Refused: the field stays open with what was typed, the notice chip says why (five reasons) | `rename.rs:44-51,160-170`; `core text notices.ftl:98-110` ("Not renamed: that isn't a usable name", "… something with that name exists already", "… Wormhole isn't allowed to change it", "… it isn't there any more", "Couldn't rename it") | PARTIAL | `FilesStore+Rename.swift:42-51` | **All five sentences differ** ("That can't be a name." …) and are shown both under the field and as a chip. Use the catalog's notice words. S. |
| After a rename: selection, folder open state, pins, open viewer (also a file inside a renamed folder), folder relisted at once; VoiceOver "Renamed to X", no chip | `rename.rs:188-216` | PARTIAL | `FilesStore+Rename.swift:25-37` | Viewer follows only an exact path (`viewing == path`); the chip "Renamed to X" is extra. S. |
| Only on the person's action, never from a timer | `rename.rs:1-12` | YES | | |

### 2e. Keyboard and watching

| State / interaction | GPUI file:line | Swift | Swift file | Note |
|---|---|---|---|---|
| Up / Down over entries (no wrap); from nothing Down = first, Up = last | `core:view/focus/list.rs:68-88`, `app:features/files/keys.rs:14-36` | PARTIAL | `sw:PanelModel/Shared/ListNavigation.swift`, `FilesStore.swift:122-128` | Same, except the head is not an item. |
| Option-Up / Option-Down first / last | `list.rs:26-30` | NO | `SelectableListSplit.swift:46-50` | |
| Right on a closed folder opens it; Right on an open folder goes to its first child | `list.rs:100-106` | PARTIAL | `FilesStore.swift:130-134` | The open-folder case stays put. |
| Left on an open folder closes it; Left elsewhere goes to the parent | `list.rs:93-98` | YES | `FilesStore.swift:132-134` | |
| Return on a folder toggles, on a file opens the viewer; Space presses as Return | `keys.rs:59-95`, `app:focus_keys.rs:75-84` | PARTIAL | `SelectableListSplit.swift:51` | Return only; **Space is not bound**. |
| With the viewer up, walking onto a file shows it; a folder is not toggled by walking | `keys.rs:69-87` (`following`) | YES | `FilesStore.swift:124-127` | |
| Type-ahead | not in Files (only the sidebar, `app:sidebar/keys.rs`) | n/a | | |
| Click in the list takes the keys; F6 / Tab ask the list for them | `app:window/focus/panel.rs:185-201` | YES | `SelectableListSplit.swift:40-43` | |
| Refresh: every 5th tick (about 2 s) while the tab shows, once when the tab is shown or the window comes forward; nothing read while hidden | `app:features/files/read.rs:155-195`, `mod.rs:34` | PARTIAL | `FilesStore.swift:67-96`, `RustFilesBackend.swift:42-49` (FSEvents watch with slow-poll fallback, `ffi:files_panel/watch.rs`) | Event-driven (better), but **keeps reading while the tab is hidden** and does not re-read on show. S. |
| Follows the terminal's folder (its repository root, else the folder) until a folder is chosen | `read.rs:51-85` (`band_folder`) | PARTIAL | `FilesStore.swift:45-52` | Swift follows the sidebar's folder string; whether that is the repository root is unverified. |

### 2f. Viewer

GPUI: a card laid over the terminal (a `CoverDef`) whose header is the **window's title row** (path crumb, size, Open in icon, close X) (`app:features/files/def.rs:48-71`, `viewer/card.rs:83-130`). Swift also lays the cover over the terminal (`sw:WindowUI/TerminalColumn.swift:42`, `sw:PanelUI/Shell/PanelCoverView.swift`) but keeps the header inside the card.

| State / interaction | GPUI file:line | Swift | Swift file | Note |
|---|---|---|---|---|
| Opens over the terminal; closes with Esc, the X, another terminal or another cover; one viewer per window | `viewer/mod.rs:140-190`, `app:window/focus/mod.rs:209-221` | PARTIAL | `PanelController.swift:70-79,87-95`, `sw:PanelUI/Files/FileViewerView.swift:17-18`, `WindowFocus.swift:158-161` | Another terminal in the *same* folder keeps the viewer in Swift; GPUI `selected()` dismisses it (`mod.rs:157-160`). S. |
| Header: path crumb + size caption + Open in (default editor, tooltip from `open_file::tooltip`) + close; 38 pt title row | `viewer/card.rs:83-130` | PARTIAL | `FileViewerView.swift:36-53` | Swift: its own 40-pt strip inside the card with name, folder, size, Open in, **Reveal, Copy Path**, close; `PanelCoverTitle.carries` is false for files (`PanelCoverTitle.swift:14`). Height 40 vs 38, place and buttons differ. M. |
| Text: numbered lines; the number column stays put while the code scrolls sideways; a sideways wheel scrolls code only | `card.rs:196-218,260-285,300-330` | NO (numbers scroll away) | `sw:PanelUI/Files/FileViewerBody.swift:81-93` (`ScrollView([.vertical,.horizontal])`, the number is inside each row) | WRONG. M. |
| Gutter: at least 3 digits, digits × char width + 20, `diff.line_number` colour | `card.rs:20,195-196` | YES | `sw:PanelUI/Diff/DiffStyle.swift:19-20` | Real advance instead of 0.6 em: fine. |
| Row height `round(size × 1.6)`, min 20 | `card.rs:233` | NO | `DiffStyle.swift:16` (`max(22, round(size × 1.75))`) | Viewer reuses the diff's roomier rows. S. |
| Tabs expanded to the `tab_width` setting | `viewer/lines.rs:129` | NO | `sw:PanelUI/Diff/DiffCode.swift:10` (`tabWidth = 4`) | Setting ignored. S. |
| Control characters as Control Pictures (`␛`), bidi / C1 / line separators as `<U+202E>`, dimmed (Trojan Source guard) | `app:syntax/visible.rs:20-60`, `viewer/lines.rs:117-130,193-215` | NO | missing (no placeholder code in `sw:PanelUI` or `sw:PanelModel`) | Security-relevant for hostile files. M. |
| Long lines capped at 5,000 characters with a dimmed `…`; sideways range from the drawn part | `viewer/lines.rs:20-22,68-75` | NO | `FileViewerBody.swift:81-93` (`Text(...).fixedSize()`) | A single 2 MB line freezes the view. S/M. |
| Syntax colours made off the main thread after the text shows; follow the theme; no grammar = plain | `viewer/colors.rs`, `viewer/read.rs` | PARTIAL | `sw:PanelUI/Stores/FilesStore+Viewer.swift:23-37` (Swift lexer, not tree-sitter) | Already listed under P03. |
| Truncated file: footer "Showing the first {2 MB} of {size}", caption, hairline above | `card.rs:240-257`, `core:storage/files/viewing.rs:14` (`TEXT_CAP` 2 MiB) | PARTIAL | `FileViewerBody.swift:34-41` | Swift sums the shown lines' bytes (+1 per line) instead of the cap; can be a few bytes off. S. |
| Empty file: "This file is empty." | `card.rs:196-198` | YES | `FileViewerBody.swift:21` | |
| Binary: panel with File icon, "Binary file, 48.2 KB", **Open in…** (primary) and **Reveal in Finder** buttons | `card.rs:141-146,173-196` | PARTIAL | `FileViewerBody.swift:44-50` | No icon, no Reveal. S. |
| Image: scaled to fit (`ScaleDown`, 16-pt pad); decode refused as too large → "This picture is too large to show."; undecodable → "This picture can't be shown." with Open in… / Reveal | `card.rs:147-165`, `app:platform/image` | PARTIAL | `FileViewerBody.swift:52-58` | One message only (`files-viewer-image-too-large` unused); no buttons; `scaledToFit` can scale small images up (GPUI never upscales). S. |
| Caps: text 2 MiB, image 50 MiB, sniff 8 KiB | `viewing.rs:14-18` | YES | core through `ffi:files_panel/files.rs:47-49` | |
| Unavailable: gone / outside / not a file / denied / unreadable words; Open in… / Reveal shown unless gone or outside | `card.rs:166-186` | PARTIAL | `FileViewerBody.swift:59,86-95` | Words match; buttons missing. S. |
| The panel for those cases: centred icon, body text, gap 12, padding 16 | `card.rs:140-170` | NO | `FileViewerBody.swift:68-72` (text only) | |
| "Reading…" while the first read runs | `card.rs:139` | YES | `FileViewerBody.swift:20` | |
| Re-read when the file changes (stamp every 2 ticks ≈ 0.8 s while its terminal is on screen; a deleted file says so and returns if it comes back) | `viewer/read.rs:84-110`, `viewer/mod.rs:34` | PARTIAL | `FilesStore.swift:92-93` (re-read on the tree's watch) | No stamp compare; re-reads and compares content. |
| VoiceOver "name, file viewer"; close tooltip "Close file (esc)" | `card.rs:66`, `files.ftl:75-77` | YES | `FileViewerView.swift:21-23,50` | |

---

## 3. Usage tab

| State / interaction | GPUI file:line | Swift | Swift file | Note |
|---|---|---|---|---|
| Tab offered only while an agent is on screen | `app:features/usage/def.rs:36` | NO | see 1a | Largest gap. |
| Empty: one card, two paragraphs ("Claude and Codex plan limits, and Gemini response tokens, show here as you use them in Wormhole." / "Enable or update their integrations in Settings → Coding agents. …") | `app:features/usage/view.rs:25-45` | YES | `sw:PanelUI/Usage/UsageView.swift:20-29` | Words and padding (8 / 8 / gap 12) equal. The words are literals in GPUI too. |
| One card per agent that reported, in the order of first report | `core:agents/usage/mod.rs` (`record`), `view.rs:46-52` | YES | `UsageView.swift:30-34`, `sw:FilesBridge/Usage/RustUsageBackend.swift:38-41` (core `plan_limits()`) | |
| Card header: 32 pt band, agent **logo** in a lead, short name semibold, "Updated 3m ago" caption on the right, tabular digits | `view.rs:58-81` | PARTIAL | `sw:PanelUI/Usage/UsageCard.swift:14-20,60-72` | Swift draws a **monogram tile** (first letter) instead of the brand logo; the name comes from a hard-coded switch (`UsageReport.displayName`, `sw:PanelModel/Usage/UsageReport.swift:66-73`: claude / codex / gemini, else capitalised) instead of `agent.short_name()`. M (logos need the registry over FFI). |
| Plan windows: Claude "5-hour limit" and "Weekly · all models"; provider windows (Codex, others) with their own labels | `core:agents/usage/mod.rs:96-110,130-146` | YES | via `plan_limits()` (`ffi:files_panel/usage.rs` `UsageReportRecord`) | |
| Section label "Plan usage limits", only when there are limits | `view.rs:84-89`, `app:style/labels.rs:14-16` (`to_uppercase`) | PARTIAL | `UsageCard.swift:21-24`, `sw:DesignSystem/SectionLabel.swift:15-30` | GPUI draws it **UPPERCASE**, caption semibold, `ui.muted`, no min height; Swift draws mixed case with a 28-pt min height and `Palette.textSecondary`. S. |
| Row: label (body, truncating) left, words caption right (tabular), 8 pt card inset, py 8, gap 8 | `view.rs:104-135` | YES | `UsageCard.swift:41-50` | |
| Words "25% · resets 34m"; reset passed → 0 % bar and "Awaiting update"; no `resets_at` → no "resets" part | `core:agents/usage/mod.rs:73-83` | YES | `UsageReport.swift:21-27` (Swift copy of the rule) | Copy, not a core call; same logic including the clamp. |
| Duration words "45s", "34m", "2h 5m", "6d" (nearest day) | `core:agents/usage/mod.rs:263-275` | YES | `UsageReport.swift:46-57` | Copy. |
| "Updated just now" under a minute, then "Updated 3m ago" | `core:agents/usage/mod.rs:277-282` | YES | `UsageReport.swift:61-63` | |
| Bar: gpui-component `Progress`, **8 pt pill** (Size::Medium default), value = share, colour by level, accessibility label "label, words" | `view.rs:136-146`, `gpui-component-0.7.0/src/progress/progress.rs:96-129` | PARTIAL | `UsageCard.swift:51-52` (`Gauge … .gaugeStyle(.linearCapacity)`) | Native gauge: different height, ends and track. Should be a drawn 8-pt pill. S. |
| Colours: normal `ui.primary`, warn from 80 % `ui.warning`, danger at 100 % `ui.danger` | `core:agents/usage/mod.rs:149-170`, `view.rs:111-115` | YES | `UsageCard.swift:67-71`, `UsageReport.swift:29` (primary, statusWarning, statusDanger) | `ui.danger` = `status.danger` assumed, not diffed. |
| Over limit (≥ 100 %) | `Level::Danger` | YES | same | |
| Gemini: no limits; "Latest model response", "N tokens", caption "Latest report across Gemini terminals. Plan limits are not reported." | `view.rs:92-103` | PARTIAL | `UsageCard.swift:26-34` | Swift groups the number (`formatted()`, "12,345 tokens"); GPUI prints it raw. S. |
| Several agents: stacked cards, 8 pt apart | `view.rs:50-52` | YES | `UsageView.swift:30-34` | |
| Several accounts / profiles: one report per agent; each profile has its own set (kept per profile, dropped on profile delete) | `app:features/usage/state.rs:13-19,86-95` | PARTIAL | `ffi:files_panel/usage.rs` (`in_dir`, one list) | Per-profile reports not verified on Swift. M. |
| States no data / stale | "Updated Nm ago" and "Awaiting update" only; no separate stale state | YES | same | |
| Words that move with time redraw at the exact second (under a minute) or minute boundary | `core:agents/usage/mod.rs:285-` (`next_change`), `app:features/usage/def.rs:25-30,43` | PARTIAL | `UsageView.swift:33` (`TimelineView(.everyMinute)`) | "resets 45s", "Updated just now" and "Awaiting update" can be up to 59 s late. S. |
| Reports kept across restarts | `app:features/usage/state.rs:34-41` | YES | `ffi:files_panel/usage.rs` (store) | |
| Scrolls as a column with the top room and the edge hairline | `view.rs:21-24`, `app:style/panel_list.rs:55-` | PARTIAL | `PanelCard.swift:23-33` | No scroll-edge hairline. |
| VoiceOver: bar "label, words" | `view.rs:140-145` | YES | `UsageCard.swift:54-56` | Swift also names the card after the agent (fine). |
| Two limits with the same label | n/a | WRONG | `UsageCard.swift:23` (`ForEach(report.limits, id: \.label)`) | Duplicate ids when a provider reports two windows with one label. S. |

---

## 4. Metrics where Swift visibly uses something else

| Item | GPUI | Swift |
|---|---|---|
| Tab icon size | 16 pt (`Size::Medium`, `app:style/icon_motion/tab_glyph/mod.rs:76`) | 14 pt (`sw:WindowUI/PanelTabBar.swift:96`) |
| Viewer row height | `round(size × 1.6)`, min 20 (`app:features/files/viewer/card.rs:233`) | `max(22, round(size × 1.75))` (`sw:PanelUI/Diff/DiffStyle.swift:16`) |
| Viewer tab width | setting `tab_width` (`viewer/lines.rs:129`) | fixed 4 (`sw:PanelUI/Diff/DiffCode.swift:10`) |
| Viewer header | window title row, 38 pt (`app:style/scale.rs:58`) | in the card, 40 pt (`sw:PanelUI/Files/FileViewerView.swift:50`) |
| Usage bar | 8-pt pill (`Progress`, Medium) | native `Gauge` linearCapacity (`UsageCard.swift:57`) |
| Usage section label | uppercase caption semibold, no min height (`app:style/labels.rs:14`) | mixed case, min height 28 (`sw:DesignSystem/SectionLabel.swift:15-30`) |
| Toggle tooltip keys | Ctrl-Cmd S, Option-Cmd I (`core:keys/keymap/table.rs:175-177`) | "⌘B", "⌥⌘B" (`sw:WindowUI/WindowControls.swift:44,49`) |
| Tab press state | `ui.active` while pressed (`app:style/controls.rs:100-102`) | none (`PanelTabBar.swift:66-72`) |
| Resize handle | hover 2-pt line, active line (`app:window/resize.rs:19,113-142`) | transparent (`ColumnHandle.swift:21`) |

Equal and checked: title row 38, tab 28, tab gap 4, leading pad 6, toggle slot, panel 240 / 360 / 720, card 10 / 4 / 12 / 8, band 32, row 28, indent 24, lead 16, count badge, tab content slide 10 pt / 140 ms, column slide 250 / 200 ms.

---

## 5. MISSING or WRONG in Swift, prioritised

Size: S = hours, M = about a day, L = several days.

1. Usage and Project tabs offered by the tab's own rule (Usage: agent on screen, returns to the previous tab, menu line hidden; Project: terminal has a folder or it is open). Needs the two facts in the core chrome snapshot or the Swift window state, then `hub.panelTabs` filters and a `leave_missing_tab` fallback. Size M.
2. Viewer hostile-content guards: placeholders for control, bidi and C1 characters (dimmed), 5,000-character line cap with `…`. Security-relevant. Size M.
3. Viewer text layout: gutter fixed while the code scrolls sideways, row height `round(size × 1.6)`, `tab_width` setting, truncated footer from `TEXT_CAP`. Size M.
4. Files per-terminal state (open folders, selection, chosen folder kept per terminal; viewer dismissed on terminal switch). Size M.
5. Files row menu: Open in ▸ submenu (apps, icons, chosen checked, disabled for links, any app for folders), order Open in / Reveal / Copy Path / Rename / Pin, Copy Path control-character refusal. Size M.
6. List keyboard: path head as a Tab stop with Return opening the path menu, Option-Up / Option-Down, Space, Right into the first child, and the focus ring on the cursor row (all list tabs). Size M.
7. Viewer non-text panels: icon, Open in… and Reveal buttons for binary, undecodable, denied, unreadable; "too large" picture message; no upscaling; title-row header (38 pt) instead of the 40-pt in-card header. Size M.
8. Tab icons and look: correct Lucide glyphs (Changes git-compare, Project folder, Files folder-tree), 16 pt, pressed state, key in the tooltip, icon motion on activation. Size M (S without the motion).
9. Wrong words and keys: rename failure sentences (use `notice-failed-rename-*`), no "Renamed to" chip (VoiceOver only), no trim; toggle tooltips Ctrl-Cmd S / Option-Cmd I; path menu volume name from the system (`"Macintosh HD"` is hard-coded). Size S.
10. Usage polish: remove the Usage count badge; uppercase section label without min height; 8-pt pill bar; brand logos and `short_name` from the agent registry; token count without grouping; exact-second refresh; unique ids for limits. Size S (logos M).
11. Tab VoiceOver: Tab / TabList roles, label with counts in words (`spoken::panel_tab`). Size S.
12. Resize handle: hover line, active line, Esc cancels the drag. Size S.
13. Notes stash button beside the tabs (the view exists, it is not placed). Size S.
14. The person's file icon theme (and none) instead of the bundled one. Size M.
15. Files list performance: virtualised rows (up to 5,000 per folder), in-flight guard and unchanged-folder stamps, no reads while the tab is hidden, re-read on show. Size M.
16. Pins: per window profile, refresh after a rename. Size S.
17. Smaller: Esc ladder `popup` flag always false; the tab-content slide side uses `allCases` instead of the offered list; Swift usage reports per profile unverified. Size S.
