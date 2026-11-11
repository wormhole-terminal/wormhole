# Feature audit: rows graded works_real 1, GPUI against Swift

Branch `exp/swift` (worktree `wormhole/.claude/worktrees/swift`, at 5ee3c590c; it contains `main`, so this
worktree's `crates/` is the GPUI reference). Read-only: no file in the repository changed, nothing built or run.

Method, per row: the GPUI feature was read in its source (the app crate's module, the core and term code it
calls, `repo-docs/docs/` where a user doc exists) and written down as behaviours; each behaviour was looked up in
`apple/swift/Sources` and `apple/ffi/src` and counted present only with a named file and function reached from the
user's action (a function with no caller is absent). Seven readers worked one area each; the findings that carry
the most weight were checked again by hand (`option_as_meta` hard-coded `.left` against core's default `Off`;
`ffi/src/terminal.rs` `spawn_options` ignoring Settings and no `set_default_colors`/`set_color_scheme` anywhere;
the sidebar row's × sending `.close`; `ThemeSync.apply` looking picks up among bundled themes only; no document
types in the Swift `Info.plist`; `WorkspaceHost.make` running the sample workspace without `--shell`).

Paths: GPUI paths are relative to `crates/wormhole/src/` unless they start with `crates/` or `wormhole/src/`;
Swift paths to `apple/swift/Sources/`; FFI paths to `apple/ffi/src/` (written `ffi/src/...`).
Sizes: S under 60 lines, M 60 to 250, L more.

## Result

62 rows are graded works_real 1. The 59 in the requested areas were audited (not audited, outside the list: O02,
W08, P09).

- **12 complete as far as the source shows**: T06, T11, T14, T22, T23, T26, S04, X06, X08, X10, STX3, and S06,
  which has nothing to match (GPUI removed sidebar search and the Show filter in a9f61dd52: the row's 640 weight
  counts deleted code, and Swift's search is an addition).
- **47 have gaps**: T01-T05, T07-T10, T12, T13, T16, T17, T19, T20, T21, S01, S02, S03, S05, S07, S09, SD01-SD03,
  A01-A06, X01, X02, X05, X09, X11, W01, W03, W04, W06, W09, W11, O01, O03, ST01, ST09, STX1.

Of the 47, about a third are small and mostly about how things look (W01, W04, W09, W11, T08, T10, A02, STX1,
X05, X11). The ones that change what a person can do are in the ranked list at the end.

Two findings cut across many rows and are the largest:
1. **Terminals start without the person's settings, shell or theme.** `ffi/src/terminal.rs:358-372`
   `spawn_options` fixes scrollback at 10,000 and leaves every other engine option at its default; the program is
   always `--shell`/`WORMHOLE_SHELL`, run as a non-login shell, with no profile or project environment; without
   `--shell` the app runs the sample workspace. Nothing gives the engine the theme's colours or light/dark, and
   nothing carries a program's own colours (OSC 4/10/11/12) back to the renderer. Touches T01, T03, T09, X02 and
   several features no row names.
2. **Settings that Swift shows but nothing applies**: `option_as_meta` (hard-coded Left; GPUI's default is Off),
   `scrollback_lines`, `image_memory_mib`, `allow_window_ops`, `allow_title_report`, `allow_screen_checksum`,
   `default_shell`, `icon_theme`, `terminals_in` (switched in memory only), a user theme picked in `config.json`
   (only bundled ones are looked up), and the theme a Studio save makes (set in memory, never written).

Correction to Part 2's T21 verdict: Part 1 found the pane's drop highlight missing (the pane tints while files
are dragged over it, `terminal_view/render/mod.rs:134,292`; Swift: `MetalTerminalView` `draggingEntered`/`draggingExited`
overlay; S, visible). That is T21's file drop, so T21 is counted with gaps.

| Row | Verdict | Row | Verdict | Row | Verdict |
|---|---|---|---|---|---|
| T01 | gaps (M) | S01 | gaps | A01 | gaps |
| T02 | gaps (M) | S02 | gaps (S) | A02 | gaps (S, looks) |
| T03 | gaps (S×4) | S03 | gaps (M) | A03 | gaps (release has no updater) |
| T04 | gaps (S) | S04 | complete | A04 | gaps (M) |
| T05 | gaps (S×3) | S05 | gaps (tabs side) | A05 | gaps (security) |
| T06 | complete | S06 | nothing to match | A06 | gaps (S) |
| T07 | gaps (Option as Meta) | S07 | gaps (M) | X01 | gaps (icon motion) |
| T08 | gaps (S) | S09 | gaps (many S) | X02 | gaps (M) |
| T09 | gaps (S) | SD01 | gaps (L) | X05 | gaps (S) |
| T10 | gaps (S) | SD02 | gaps (M) | X06 | complete |
| T11 | complete | SD03 | gaps (user theme not shown) | X08 | complete |
| T12 | gaps (S) | W01 | gaps (S) | X09 | gaps (fonts folder) |
| T13 | gaps (S) | W03 | gaps (M) | X10 | complete |
| T14 | complete (test-instance sound is X11's) | W04 | gaps (S, looks) | X11 | gaps (test instances) |
| T16 | gaps (S×3) | W06 | gaps (L) | ST01 | gaps (S) |
| T17 | gaps (M) | W09 | gaps (S) | ST09 | gaps (S+M) |
| T19 | gaps | W11 | gaps (S) | STX1 | gaps (S) |
| T20 | gaps (M) | O01 | gaps (close without asking) | STX3 | complete |
| T21 | gaps (S, drop highlight) | O03 | gaps | | |
| T22 | complete | | | | |
| T23 | complete | | | | |
| T26 | complete | | | | |

The per-row sections follow, area by area (terminal, sidebar, studio and crosscut, aux and settings, window and
overlays), then the ranked list of every gap and the features no row covers.

---

## terminal rendering and input (T01–T09) and the input/platform sweep

Read-only audit of exp/swift (worktree `.claude/worktrees/swift`). GPUI paths are relative to `crates/`, Swift to
`apple/swift/Sources/`, FFI to `apple/ffi/src/`. "Present" means a named function reached from the user's action.

A finding that cuts across several rows, put first because it is the largest: **the FFI starts every terminal with
`SpawnOptions` that ignore Settings and the theme** (`ffi/src/terminal.rs:361-372` `spawn_options`: fixed
`SCROLLBACK_LINES = 10_000`, everything else `SpawnOptions::default()`; hosted terminals use the same function). GPUI
fills them from the profile's settings and palette (`wormhole/src/terminal_view/mod.rs:284-291`) and keeps the engine's
colours in step on theme changes (`terminal_view/mod.rs:654-665` `share_palette`). In Swift nothing calls
`set_default_colors` / `set_color_scheme` (no hit in `ffi/src` or `Sources`). The rows below cite it where it bites.

### T01 Grid drawing: rows; runs; styles; wide cells; underline styles; strikeout — verdict: gaps

GPUI does:
- cells, runs, bold/italic, dim, inverse, hidden, wide cells [TerminalMetal/RowBuilder.swift `colors`, `build`]
- underline single/double/curly/dotted/dashed, SGR 58 colour [RowBuilder.swift underline switch; Shaders.swift patterns; ffi frame_extras.rs]
- strikeout, overline [RowBuilder.swift `flushStrike`, overline at :225]
- SGR 5 text blink [TerminalMetal/TextBlink.swift]
- double-width / double-height rows [TerminalCore/PackedFrame.swift `LineSize`; Pointer/CellGrid.swift; RowLayout.swift]
- box drawing / block elements as shapes [core cell_shapes via TerminalCore/CellShapes.swift, BoxDrawing.swift]
- selection colours incl. theme selection foreground [TerminalCore/Palette.swift slots 259/260; RowMarks.swift:68]
- program-set colours (OSC 4 / 10 / 11 / 104…) resolved over the theme: `palette.cell_colors(cell, selected, &snapshot.colors)` (`terminal_view/render/frame.rs:119`, `palette.rs:97`) — ABSENT
- a program's OSC 11 background also painted behind the pane padding (`render/mod.rs:145,260`) — ABSENT
- engine answers colour queries OSC 4/10/11/12 and `CSI ? 996 n` with the theme's colours (spawn `colors`, `color_scheme`, `share_palette`) — ABSENT

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Colours a program sets at run time (OSC 4 palette entries, OSC 10/11 fg/bg, OSC 104/110/111 resets) are drawn: the FFI frame carries colours unresolved and no override table (`ffi/src/frame.rs:17-19`, nothing reads `snapshot.colors` except `blocks.rs:181`), so Swift always resolves against the theme | wormhole/src/palette.rs:97-133; terminal_view/render/frame.rs:119; render/prepare.rs:184 (cache key) | new frame section in ffi/src/frame.rs (overrides when changed) + TerminalCore/Palette.swift override slots + MetalRenderer invalidation | M | yes (base16-shell / theme scripts, programs that recolour) |
| OSC 11 background behind the padding, not just the cells | terminal_view/render/mod.rs:145-146,258-260 | MetalRenderer clear colour from the override above | S | yes |
| Theme colours and light/dark given to the engine at spawn and on theme change, so OSC 4/10/11/12 queries and `CSI ? 996 n` answer the real theme (vim/neovim/delta pick light vs dark from them) | terminal_view/mod.rs:289-291, 654-665 | ffi/src/terminal.rs `spawn_options` (+ hosting) and a `set_palette` call from TerminalMetal `setPalette` | S | yes |

### T02 Ligatures; font fallback; emoji; font features — verdict: gaps

GPUI does:
- font family / size / line height / ligatures from Settings, per profile [Wormhole/Look/FontSettingsMaker.swift `settings`, Settings/SettingsApplier.swift:53, TerminalMetal/FontSet.swift]
- ligatures off turns off calt/liga [FontSet.swift:118-122]
- fallback for CJK, emoji, symbols, colour emoji [FontSet.swift cascade, `id(of:)`, `hasColorGlyphs`; WordCache.swift]
- bundled JetBrains Mono [FontSettingsMaker `.jetBrainsMono(directory:)`]
- the user's font files (fonts folder) registered at launch and re-registered when the folder changes, so a custom family can be the terminal font (`fonts.rs:50 init`, `:75 poll_custom`, `:106 register_all`) — PARTIAL: Swift registers them only inside the font picker (`FontPickerModel/FontLibrary.swift:64`); `FontSettingsMaker` asks CoreText for `.system(family:)`, which is unregistered at launch

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Custom font files registered at launch and when the fonts folder changes (a custom family set as the terminal font draws in a fallback face until Settings' font picker has been opened; no folder watch) | wormhole/src/fonts.rs:50-136 | Wormhole/Look (launch: scan core `custom::scan` over the FFI, `CTFontManagerRegisterFontURLs`), folder watch beside the config watch | M | yes |

### T03 Cursor styles; blink; unfocused hollow — verdict: gaps

GPUI does:
- block / underline / beam, wide cursor, hollow block when unfocused [RowBuilder.swift `addCursorShape`, `colors(underBlockCursor:)`]
- blink only when the program asks (DECSCUSR), Reduce Motion steady, typing holds it on [TerminalMetal/CursorBlink.swift `updateBlink`, `holdCursor`; TerminalFocus.swift `accessibilityOptionsChanged`]
- blink only in the focused pane (`render/prepare.rs:164`: `blinking && self.focused`) — PARTIAL: Swift checks only `window?.isKeyWindow` (CursorBlink.swift:21) and `resignFirstResponder` does not call `updateBlink`
- character inside the block in the theme's `cursor_text`, else the cell background (`render/cursor.rs:85-87`) — PARTIAL: Swift always uses the cell background (RowBuilder.swift:43-45); `TerminalPalette.cursorText` is read nowhere but the Studio
- cursor colour from OSC 12 (`render/cursor.rs:154-155`) — ABSENT (T01's override table)
- no cursor once the shell has exited (`render/cursor.rs:66-68` `drawn`) — ABSENT: the FFI packs the engine's cursor regardless (`ffi/src/terminal.rs:464` `pack_frame`), and nothing in TerminalMetal hides it on `.exited`

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Theme's cursor-text colour for the character under a block cursor | terminal_view/render/cursor.rs:85-87,142-143 | TerminalCore/Palette.swift (a slot) + RowBuilder.swift `colors` | S | yes (themes that set cursorText) |
| OSC 12 cursor colour | terminal_view/render/cursor.rs:154-155 | with T01's override slots | S (after T01) | yes |
| Cursor not drawn after the program exited | terminal_view/render/cursor.rs:66-68 | MetalTerminalView `handleEvents` `.exited` → renderer hides cursor | S | yes |
| A pane that is not focused in a key window stops blinking | terminal_view/render/prepare.rs:163-164 | CursorBlink.swift `updateBlink` (use first-responder focus); call it from `resignFirstResponder`/`becomeFirstResponder` | S | yes (split panes) |

### T04 Selection: cell; word; line; block; drag; autoscroll; Cmd-A — verdict: gaps

GPUI does:
- click = cells, double = word, triple = line; drag; ⇧ forces selection over a mouse program [Pointer/TerminalPointer.swift `press`, ffi interaction/selection.rs]
- drag past the edges scrolls the history by the distance [Pointer/DragSelection.swift `extendSelection`, CellGrid `dragScroll`] (Swift adds a repeat timer: improvement)
- ⌘A select all [Pointer/TerminalShortcuts.swift → `selectAll`]
- ⌘C copies and the notice chip says "Copied N characters/lines" (`input.rs:175-180`, core `notice::Copied::text`) — PARTIAL: Edit ▸ Copy through the router posts it (Wormhole/AppController+Open.swift:66 `copySelection`), but ⌘C typed in the terminal is taken by `performKeyEquivalent` (TerminalShortcuts.swift:41-44) → `copy(nil)` → `onCopied`, which no one assigns (grep: declared MetalTerminalView.swift:22, never set)
- (Option-drag block selection is Swift only; GPUI has none)

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| ⌘C from the keyboard shows the "Copied …" chip (only the menu path does) | terminal_view/input.rs:175-180 | assign `MetalTerminalView.onCopied` in the window hub (post `.copied(text:)`), or route ⌘C through `copySelection` | S | yes |

### T05 Mouse reporting modes 1000-1006 — verdict: gaps

GPUI does:
- report press/release/motion per mode, SGR/pixels (1016), locator, ⇧ takes the pointer back, motion only on a new cell [ffi interaction/pointer.rs `report_pointer`, `locator`; TerminalPointer.swift `send`]
- right / ⌃-click open the menu only when the program does not take the mouse [TerminalMenu.swift `opensMenu`, `menu(for:)`]
- a release outside the pane still ends the drag [AppKit delivers mouseUp to the view]
- pointer: hand over a link, arrow while a program tracks the mouse, else I-beam, recomputed every render (`render/mod.rs:267-273`) — PARTIAL: Swift `resetCursorRects` (TerminalLinks.swift) is only invalidated on a link change or scroller style change, so the shape is stale when a program turns mouse tracking on or off
- middle and right press forget the program's own selection (`input.rs:313-315`) — ABSENT: `report_pointer` only tracks Left (ffi interaction/pointer.rs:123-132)
- only left/middle/right are buttons (`input.rs:375-380`, `_ => return`) — WRONG: Swift `otherMouseDown/Up/Dragged` report every other button (back/forward, buttons 3+) as middle (TerminalPointer.swift:38,68,82)

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Pointer shape follows the program's mouse mode (arrow ↔ I-beam) as it changes | terminal_view/render/mod.rs:267-273 | MetalTerminalView after a frame: `invalidateCursorRects` when `takesPointer` flips | S | yes |
| Extra mouse buttons (back/forward) are not sent as a middle click | terminal_view/input.rs:375-380 | TerminalPointer.swift `otherMouseDown` (check `buttonNumber == 2`) | S | yes (to mouse programs) |
| Middle/right press forgets the program's selection (⌘V replace logic) | terminal_view/input.rs:313-315 | ffi/src/interaction/pointer.rs `report_pointer` | S | no |

### T06 Wheel and precise scroll into scrollback; alt-screen wheel — verdict: complete

GPUI does: line/pixel deltas with remainders [TerminalWheel.swift `scrollWheel` → ffi pointer.rs `scroll_wheel`]; wheel buttons to a mouse program, at most 10 per event; horizontal wheel only to a mouse program; alternate-scroll arrows on the alt screen (APP_CURSOR aware); ⇧ forces history; otherwise history scroll — all present. (`forget_block_cursor` on scroll is T13's.) Complete as far as the source shows.

### T07 Keyboard: key table; Option as Meta (sides); kitty keyboard; modifyOtherKeys — verdict: gaps

GPUI does:
- every key through the engine's encoder by mode (app cursor, kitty, modifyOtherKeys) [Keyboard/KeyDown.swift `send` → ffi terminal.rs `send_key` → `keys::encode`]
- the terminal's ⌘ keys: copy, paste, select all, ⌘K clear, ⌘⌫/⌘←/⌘→ line editing [Pointer/TerminalShortcuts.swift]; ⌘↑/⌘↓ prompts (T13's, via menus)
- a typed key returns to the live screen, holds the cursor, forgets the program's selection [ffi `send_key`/`write`]
- Option as Meta by the `option_as_meta` setting (off/left/right/both, side-aware) (`input.rs:139-142`, default `Off`: core config/settings/choices.rs:123-125) — ABSENT/WRONG: `MetalTerminalView.optionAsMeta` is hard-coded `.left` (MetalTerminalView.swift:47) and never assigned from Settings (no assignment in Sources)
- ⌘ key not in the terminal's table and not an app key is retried with ⌘ alone (`input.rs:105-110`, e.g. ⇧⌘V pastes) — PARTIAL: Swift only for ⇧⌘C (TerminalShortcuts.swift:19)
- ⌘K Clear Buffer also erases the terminal's saved text in the database (`input.rs:215` → `window/panes/mod.rs:83-88` `clear_scrollback`) — ABSENT: Swift clears history and sends ^L only (TerminalSelection.swift:34-38); the minute's saved text can bring the cleared lines back after a crash
- (IME / dead keys / dictation / emoji picker via NSTextInputClient are Swift only: Keyboard/InputClient.swift, MarkedText.swift)

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| `option_as_meta` setting applied (default Off, not Left); Swift today makes left Option Meta for everyone, breaking Option-typed characters (@, {, [ on many layouts) | terminal_view/input.rs:139-142; wormhole-core config/settings/choices.rs:123-145 | Wormhole/Settings/SettingsApplier.swift → each `MetalTerminalView.optionAsMeta` (and its default) | S | yes |
| ⌘ chords with extra modifiers fall back to the plain ⌘ action unless the chord is an app key | terminal_view/input.rs:102-110 | TerminalShortcuts.swift `of(characters:keyCode:flags:)` using the keymap's app keys | S | yes (minor) |
| ⌘K erases the saved text too | terminal_view/input.rs:200-216; window/panes/mod.rs:83-88 | TerminalSelection.swift `clearBuffer` → workspace/store `clear_scrollback` over the FFI | S | barely |

### T08 Links: Cmd-hover underline; open; OSC 8 — verdict: gaps

GPUI does:
- ⌘-hover underlines the OSC 8 / detected URL under the pointer, hand cursor, follows ⌘ going down/up [Pointer/TerminalLinks.swift `updateLink`, TerminalPointer.swift `mouseMoved`/`flagsChanged`]
- ⌘-click opens through core `Openable` (http/https/mailto, `file:` revealed, others refused and logged by scheme), also in a mouse program [TerminalLinks.swift `openLink`, ffi interaction/links.rs `open_target`]
- link only when the pointer is over a cell (`links.rs:60,204-213` `over_grid`) — WRONG: a ⌘-click uses the clamped `cellGrid.hit` without `contains` (TerminalPointer.swift:45-46), so ⌘-click in the pane padding opens the link of the nearest edge cell
- (rule links / readout / file links: T24)

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| ⌘-click outside the grid (padding) opens nothing | terminal_view/links.rs:59-61,204-213 | TerminalPointer.swift `press`: require `cellGrid.contains(point)` before `openLink` | S | yes (minor) |

### T09 Inline images: kitty; Sixel; iTerm2 — verdict: gaps

GPUI does:
- placed pictures from the engine's store, clipped to the pane, scrolled with the history [Images/ImageLayer.swift, ImageLayout.swift, ffi inline_images/]
- kitty z bands under backgrounds / under text / over text [ImageLayer.swift `band(of:)`]
- kitty Unicode placeholders through virtual placements [ffi inline_images/placeholders.rs]
- textures converted once per image and dropped when the store forgets it [Images/ImageTextures.swift]
- the decoded-image memory cap from Settings `image_memory_mib` (`terminal_view/mod.rs:285`) — ABSENT: not passed (`ffi/src/terminal.rs:365-371`), the engine's default applies whatever the setting says

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| `image_memory_mib` setting caps the terminal's image memory | terminal_view/mod.rs:285; wormhole-core terminal/process.rs:23-25 | ffi/src/terminal.rs `spawn_options` (+ hosting) from the profile's settings | S | yes (setting has no effect) |

### Uncovered (no tracker row)

Sweep results. Not gaps (neither app has it): secure keyboard entry, Look Up / force click, middle-click paste,
copy-on-select, key-release reporting (kitty flag 2: no key-up path in GPUI either), Option-click to move the cursor,
cursor style/colour/blink settings, bold-as-bright, minimum contrast, scroll speed settings (no such keys in core's
settings schema). Services: both have only the menu (GPUI `app/menus.rs:105`, Swift `PlatformUI/Menus/NSMenuFactory.swift:39-42`);
neither terminal offers its selection to services. Swift-only improvements (not parity): IME, dead keys, dictation,
emoji picker (Keyboard/InputClient.swift), Option-drag block selection, text (not only file) drops, drag autoscroll timer.
Focus reporting (DECSET 1004) is present: TerminalFocus.swift `reportFocus` → ffi pointer.rs `focus_changed`.

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| `scrollback_lines` setting: Swift keeps a fixed 10,000 lines whatever Settings says (also for hosted terminals) | terminal_view/mod.rs:284 | ffi/src/terminal.rs:27,367 `spawn_options` from the profile's settings | S | yes |
| `allow_screen_checksum`, `allow_title_report`, `allow_window_ops` settings reach the engine (always off in Swift; the toggles in Settings do nothing) | terminal_view/mod.rs:286-288 | ffi/src/terminal.rs `spawn_options` | S | yes |
| xterm window operations (XTWINOPS move/resize/minimise/raise, DECCOLM) acted on when allowed: no FFI event or Swift consumer exists | wormhole/src/window_ops.rs (250 lines); terminal_view/status.rs:158; window/panes/mod.rs:104 | ffi/src/events.rs new `TermEvent` + WindowUI handler | M | yes (with the setting on) |
| Drop highlight: the pane tints (`state.highlight`) while files are dragged over it | terminal_view/render/mod.rs:134,292 | MetalTerminalView `draggingEntered`/`draggingExited` overlay | S | yes |
| Engine told the theme's colours / scheme (see T01) — no row names colour queries | terminal_view/mod.rs:289-291,654-665 | ffi/src/terminal.rs | S | yes |

## terminal tools (T10–T14, T16, T17, T21–T23, T26) and the escape-sequence sweep

Read-only audit of worktree `exp/swift` (5ee3c590c). GPUI paths are under `crates/`, Swift under `apple/swift/Sources/`,
FFI under `apple/ffi/src/`. Sizes: S < 60 lines, M 60–250, L more.

### T10 Minimap — verdict: gaps (small)

GPUI does (`crates/wormhole/src/terminal_view/render/minimap.rs`):
- draws core `minimap` runs, one block per run in its colours [TerminalUI/Minimap/MinimapView.swift `draw`, CoreMinimap.swift `snapshot`]
- slider in `scrollbar.thumb` when the history scrolls [MinimapView `draw`]
- click on slider grabs it; elsewhere centres that line and grabs; drag follows the pointer anywhere [MinimapView `drag`]
- tick at the left edge for each collapsed fold, in the thumb colour (minimap.rs:57-70) — ABSENT
- a press focuses the terminal (minimap.rs:101) — ABSENT (SwiftUI DragGesture only)
- program-set colours (`minimap.colors` overrides) used for runs — ABSENT (see sweep, OSC 4/10/11)
- View → Show Minimap / setting `minimap`, replaces the scroller [SettingsApplier.swift:122, MetalTerminalView+Minimap]

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Fold ticks on the minimap for collapsed output folds | terminal_view/render/minimap.rs:57-70 | ffi/src/minimap.rs (expose `minimap.folds`) + MinimapView.draw | S | yes |
| Press on the minimap gives the terminal the keyboard | render/minimap.rs:101 | MinimapView.drag onChanged → host makeFirstResponder | S | yes (minor) |

### T11 Scrollbar and overlay scroller — verdict: complete as far as the source shows

GPUI draws its own bar after `wormhole-term/src/scrollbar.rs`; Swift uses a real `NSScroller`
(TerminalMetal/Scroller/TerminalScroller.swift) fed each frame (ViewScroller.swift `updateScroller`): overlay/legacy
style from the system, thumb proportion and position, page/jump by system setting (NSScroller), hold 1 s + 250 ms fade,
Reduce Motion cut (`fadeOut`), near-edge zone 16 pt (`pointer`), minimap suppresses it (`isSuppressed`), find ticks
beside it (WindowUI/Overlay/TerminalMarks.swift:20). No behaviour missing found.

### T12 Find bar — verdict: gaps (small)

GPUI does (`terminal_view/find.rs`, `find_bar.rs`, `window/window_actions.rs:135-158`):
- ⌘F opens with the field focused and selected [WindowHub+Overlays.swift `find(.open)` → FindModel.open]
- ⌘F seeds the query with the selection's first non-blank line (window_actions.rs:138-140, find.rs:106-110) — ABSENT: `ActionRouter+View.swift:47` → `hub.find(.open)` → `find.open()` with no seed
- ⌘E selection becomes the query, keyboard stays [FindModel.useSelectionForFind]
- ⌘G/⇧⌘G, ↩/⇧↩ step round the ends; closed/empty opens instead [FindModel.step, FindField.swift:24]
- case / whole word / regex toggles [FindModel.toggle, FindOptions]
- count "n of m", "+" when capped, grouped digits, "No results" only after the search, "Not a valid pattern" [FindModel+Status.countText]
- search off the main thread on the scrollback, stale results dropped by generation [FindModel.search, ScreenSearching.search]
- re-search at most every 200 ms while output arrives [FindModel.outputChanged ← TerminalOverlay.contentChanged]
- highlights only where cells still hold the match; current one distinct [ffi/src/find.rs `find_spans` holds(), MetalTerminalView+Find.refreshFound]
- reveal opens a fold containing the match [ffi/src/find.rs:112-114]
- fold markers count matches inside [ffi/src/find.rs `find_fold_words`, TerminalOverlay `syncFolds`]
- ticks along the scrollback [FindTicks in TerminalMarks.swift:20]
- Esc / close returns keys to the terminal [FindBar.swift:30, FindField.swift:27, ScreenSearching.swift `closeFind`]

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| ⌘F (menu, key, context menu) seeds the query with the selection | window/window_actions.rs:136-141; terminal_view/context_menu.rs:162-163 | WindowHub+Overlays.swift `find(.open)`: pass `selectionText()` first line to `FindModel.open(seed:)` | S | yes |

### T13 Command blocks (OSC 133) — verdict: gaps (small)

GPUI does (`terminal_view/blocks.rs`, docs/command-blocks.md):
- gutter mark per block coloured by state (prompt/running faint, 0 success, other danger, none) [TerminalUI/Blocks/BlockMarks.swift]
- hover tint + readout (exit code, time, Copy output button), readout stays while hovered [BlockController.hover/readoutHovered, BlockReadout.swift:26]
- ⌘↑/⌘↓ prompt to top; down past last returns to live screen; VoiceOver hears the block [BlockController.jumpPrompt ← ActionRouter+View.swift:52-53]
- Select Output (selects + reveals), Copy Output, Copy Command, Put Command at Prompt (only at a prompt, refuses hidden text, chip says why; chip off → bar + VoiceOver) [BlockController.perform ← ActionRouter+View.swift:54-57, AppController+Context.swift:40-42; WindowHub+BarNotice.swift:13-15]
- target block: last jump's until scrolled or typed in, else newest that ran [BlockController.target]
- the jump cursor is forgotten on any typing, paste, or wheel scroll (control_input.rs:52, mod.rs:523, input.rs:507) — PARTIAL: Swift forgets only when the display offset changed (`BlockController.noteView`); `forgetCursor()` has no caller, so typing at the live screen keeps an old target for the Edit menu
- Fold Output action on a block (blocks.rs:45, 226) — ABSENT in Swift's `BlockController.Action` (belongs to T25 folds)

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Typing/paste/scroll forgets the jump target (Edit-menu block actions then act on the newest block) | terminal_view/control_input.rs:52; mod.rs:521-523; input.rs:506-507 | call `BlockController.forgetCursor()` from KeyDown/paste/TerminalWheel via TerminalOverlay | S | yes (edge case) |
| Fold Output on a block (T25's) | terminal_view/blocks.rs:45,226 | BlockController.Action + ActionRouter | S | yes |

### T14 Bell — verdict: complete (one test-instance nit)

GPUI does (`terminal_view/status.rs:526-551`): sound when `bell_sound` (one per glow), glow 150 ms when `bell_visual`
[TerminalMetal/TerminalBell.swift `ringBell`, SettingsApplier.swift:55-58]; bell marks the row / needs-you once a minute
(core status machine via ffi workspace_feed.rs:64-69); `wh events` Bell (ffi hook_server/tap.rs:227).

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| No bell sound from a background test instance | platform/sound.rs:6 | TerminalBell.ringBell guard on background mode | S | no |

### T16 Pane bar, exited, host banners, clipboard prompt — verdict: gaps (small)

GPUI does (`terminal_view/pane_bar.rs`, `render/exited.rs`, `hosted.rs`, `host_problem.rs`):
- clipboard read question: one at a time, only on screen, 30 s timeout denies, Allow once / Always / Deny, VoiceOver announces, names program in front [ffi/src/clipboard.rs `load`, TerminalOverlay+Clipboard.swift]
- write refused when `clipboard_write` deny [ffi clipboard.rs `store`]
- chip on a program copy: "Copied n" in the typing terminal, else "<program> copied n lines" naming the program (pane_bar.rs:89-98, notice/words.rs:49-55) — PARTIAL: Swift posts `.copied(text:byProgram:)` without the program's name (TerminalOverlay+Clipboard.swift:15, Notice.swift:79-84)
- no chip for a terminal not on screen (pane_bar.rs:86-88) — ABSENT: `onProgramCopy` posts to the window's NoticeCenter for any terminal's overlay
- chip off: program copy always said in that terminal's bar for 4 s (pane_bar.rs:100-104) — PARTIAL: Swift `WindowHub.sayInBar` writes to the *selected* overlay (WindowHub+BarNotice.swift:10), and a copy in the typing terminal is `.copied(.characters)` which falls to `default: break`
- host banners: unavailable (Try again, Open new terminal here, End host), not responding (Wait, End host), lost; host fallback/data folder/move-to-Applications banner with Dismiss [PaneBarView.swift:38-46, TerminalOverlay.showHost, WindowHub+BarNotice.swift:16-17]
- exited bar: "<shell> exited (with code n)" / "ended unexpectedly", Restart, Close terminal ⌘W [TerminalUI/Pane/ExitedBar.swift]

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Program copy chip names the program ("ssh copied 120 characters") | terminal_view/pane_bar.rs:89-98; notice/words.rs:49-55 | TerminalOverlay+Clipboard.swift `onProgramCopy` (ffi `foreground_program`) + Notice words | S | yes |
| No chip for an off-screen terminal's clipboard write | pane_bar.rs:84-88 | TerminalOverlay+Clipboard.swift: skip unless the view is in a visible pane | S | yes |
| Chip-off bar notice goes to the terminal that copied, and also for the typing terminal | pane_bar.rs:100-104 | WindowHub+BarNotice.swift `sayInBar` (route by terminal id, map `.characters/.lines` too) | S | yes (Notices flag off) |

### T17 Text zoom — verdict: gaps

GPUI does (`window/window_actions.rs:170-179`, `scaling/zoom.rs`, `window/terminals.rs:181-184,231-233`):
- ⌘+/⌘-/⌘0 within 8–36 pt [MetalTerminalView+Zoom.zoom/resetZoom ← ActionRouter+View.swift:32-34]
- eases 120 ms (Motion feature), cut under Reduce Motion, PTY told on arrival, size chip [MetalTerminalView+Zoom.setZoom/advanceZoom, WindowHub+Zoom]
- the zoom is the WINDOW's: every terminal of the window zooms, and a terminal opened in or moved into the window adopts it without easing — ABSENT: Swift zooms only `state.selected` (WindowHub+Zoom.swift:9); `setZoom(_:animated: false)` exists but nothing calls it for new/moved terminals

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Zoom applies to every terminal in the window; new and moved-in terminals adopt it | window/window_actions.rs:170-179; window/terminals.rs:181-184,231-233; scaling/zoom.rs:30-38 | WindowHub+Zoom.swift (window zoom state; loop over `terminals`), terminal creation/adoption in WindowHub | M | yes |

### T21 Paste, bracketed paste, file drop — verdict: gaps (drop highlight, see Result)

GPUI (`terminal_view/input.rs:182-236`, `mod.rs:551-562`): ⌘V through core `input::paste` (bracketed when mode 2004,
CR otherwise, ESC stripped), over a program's own selection Backspace first (`app_selection`), file drop pastes escaped
paths and focuses. Swift: KeyDown.swift `paste`/`insert` → `source.paste` / `interaction.pasteClipboard`, drop via
`performDragOperation` → ffi `droppedText`, `makeFirstResponder`. Swift also accepts text drops and pastes file URLs
from the clipboard (beyond GPUI). GPUI has no large-paste or newline confirmation, so none is missing.

### T22 Print and Export text — verdict: complete

GPUI has neither (the keymap reserves the items); Swift's are extra (ActionRouter+Output.swift, TextExporter.swift,
PrintableText.swift). Nothing to match.

### T23 Probe rules on screen — verdict: complete as far as the source shows

GPUI (`render/decor.rs`, `links.rs:104-145`): core `probe::runtime::Decor` with rules, generation, reveal and hook
block tones; reveal by hover/⌥/never per rule. Swift: ffi/src/probes/marks.rs (same `Decor::update` with
`block_tone`, `set_reveal` with alt), TerminalMetal/MetalTerminalView+Probes.swift:28, RowMarks.swift.

### T26 OSC 9;4 progress goes stale after 15 s — verdict: complete

GPUI `terminal_view/progress_timer.rs`; Swift/FFI: ffi workspace_feed.rs `ProgramStatus` → `shared.arm_progress(id)`,
workspace_pump.rs one-shot deadline. The percentage/Paused/Error words are the core status words.

### Escape sequence sweep

Only sequences that need something from the app beyond the engine.

| Sequence | GPUI consumer | Swift consumer |
|---|---|---|
| OSC 0/1/2 title, title stack, 250 ms title throttle | terminal_view/status.rs (TITLE_EVERY) | ffi workspace_feed.rs `Title`/`ResetTitle` → core workspace status (rules.rs:66 TITLE_EVERY) — present |
| CSI 20t/21t title report (`allow_title_report`) | spawn option, terminal_view/mod.rs:288 | **MISSING**: never passed (ffi terminal.rs:358-372); setting shown, no effect |
| OSC 7 / 9;9 cwd | PtyEvent::Cwd → row, new terminal folder | ffi workspace_feed.rs:62 pump probe — present |
| OSC 8 hyperlinks | terminal_view/links.rs | TerminalMetal/Pointer/TerminalLinks.swift (T08) — present |
| OSC 9 / 99 / 777 notification words | status.rs:477-494 `on_notice` → `ProgramNotice` → banner "in the program's words" (Kind::Notice) | **PARTIAL**: the bell rule runs (workspace_feed.rs:70 `StatusEvent::Notice`), but the text never reaches a banner: `NoticeFeed.noticeTabs` (Wormhole/Notices/NoticeFeed.swift:65-80) never fills `notice` (nor `failure`, `foreground`, `.program` source), so ffi notify/watch.rs:98 never sees a serial. PL03 row (0.5) territory |
| OSC 9;4 progress | progress_timer.rs | workspace_pump.rs — present (T26) |
| OSC 52 write / read prompt | pane_bar.rs | ffi clipboard.rs + TerminalOverlay+Clipboard.swift — present (gaps in T16) |
| OSC 133 marks | blocks.rs, status.rs `on_mark` | ffi blocks.rs, workspace_feed.rs `Mark` — present |
| OSC 7501 program status (+ `?` reply) | engine + status.rs | engine + workspace_feed.rs `ProgramStatus` — present |
| OSC 1337 File=, APC G, DCS q images | render/images.rs | TerminalMetal/Images (T09) — present |
| OSC 4/5/10–19 set colours, drawn in cells and behind the padding (OSC 11) | palette.rs:85-93 `resolve(color, overrides)`; minimap uses them | **MISSING**: `snapshot.colors` never crosses the FFI (frame.rs carries unresolved named colours only; FFI-CONTRACT.md:526 says so); programs' palettes/backgrounds (base16-shell, vim/neovim OSC 11) are not drawn |
| OSC 4/10/11/12 queries answered with the theme's colours | terminal_view/mod.rs:657-665 `share_palette` → `set_default_colors`; spawn `colors` (mod.rs:290) | **MISSING**: neither spawn `colors` nor `set_default_colors` in the FFI; queries answer xterm's defaults (programs pick wrong light/dark) |
| CSI ? 996 n / mode 2031 colour-scheme reports | `set_color_scheme` at spawn and on theme change (mod.rs:291, 664) | **MISSING**: no `color_scheme`/`set_color_scheme` in the FFI; queries go unanswered, theme switches unreported |
| XTWINOPS 1/2/3/4/5/6/8/9/10, DECSLPP/DECSNLS/DECSCPP/DECCOLM width (`allow_window_ops`) | window_ops.rs (250 lines) on `PtyEvent::WindowOp`; window_geometry.rs:66-100 | **MISSING**: `allow_window_ops` never passed; `PtyEvent::WindowOp` falls to `_ => return None` (ffi events.rs:168) |
| CSI 11t/13t/14;2t/15t/19t reports (window state, position, window/screen pixels, max cells) | window_geometry.rs:42-65 `share_window` → `Pty::set_window` each frame | **MISSING**: no `set_window` in the FFI; reports answer zeros/defaults |
| CSI 14t/16t/18t, mode 2048 in-band resize (cell size) | resize with cell size | ffi terminal.rs:271 `resize(cols, rows, cell_width, cell_height)` — present |
| DECRQCRA checksum (`allow_screen_checksum`) | spawn option mod.rs:286 | **MISSING**: never passed (setting has no effect; safe default off) |
| DECSCUSR cursor shape, blink | render/cursor.rs | T03 RowBuilder/CursorBlink — present |
| BEL | status.rs `ring` | TerminalBell.swift — present |
| DECSET 1004 focus reports | input focus | TerminalFocus.swift:46 → ffi interaction/pointer.rs:210 — present |
| DECSET 2004 bracketed paste | mod.rs:558 | ffi paste through core `input::paste` — present |
| 2026 synchronized output, 2027 | engine | engine — present |
| kitty keyboard, modifyOtherKeys | engine encode | ffi `send_key` (T07) — present |
| ESC c (RIS) forgets program status | engine + term | engine + term — present |
| CSI ? 2 l VT52, DECLOCATOR | engine / input | engine; ffi interaction/pointer.rs:235 `locator` — present |

### Uncovered (no tracker row)

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Terminal start options from Settings: `scrollback_lines` (Swift fixed 10,000; Settings offers 1k–200k), `image_memory_mib`, `allow_window_ops`, `allow_title_report`, `allow_screen_checksum` — shown in Swift Settings, no effect | terminal_view/mod.rs:284-288 | ffi/src/terminal.rs:358-372 `spawn_options` (read the settings handle), hosting/desk.rs:205,237 | S | yes |
| Which program a terminal runs and with which environment: `default_shell`, a terminal's saved shell, a trusted project's shell and env, worktree ports, the profile's env/git identity (`apply_profile`); Swift always runs `--shell`/`WORMHOLE_SHELL` (WorkspaceBridge/RustWorkspaceBackend+Hosting.swift:196) unless "New terminal: <shell>" | window/terminals.rs:188-204 (`spawn_adds`, `shells::for_terminal`, `apply_profile`) | ffi terminal.rs `spawn_options` + RustWorkspaceBackend.request | M | yes |
| Theme colours and light/dark scheme told to the engine at spawn and on every theme change (OSC 10/11 queries, 996/2031) | terminal_view/mod.rs:290-291, 657-665 | ffi terminal.rs (spawn `colors`, `color_scheme`) + a `set_theme_colors` call from TerminalLook on theme change | S | yes (programs pick wrong light/dark) |
| Program-set colours (OSC 4/10/11/12) drawn in cells, padding and minimap | palette.rs:85-93; render paths passing `snapshot.colors` | ffi frame.rs (a colours section), TerminalMetal renderer palette resolve, MinimapView.color | M | yes |
| xterm window operations carried out (move, resize to cells, minimise, raise/lower, maximise, full screen, DECCOLM width) and window geometry reported | window_ops.rs:1-250; window_geometry.rs:1-101 | ffi events.rs:168 (translate `WindowOp`), a TermEvent → WindowHub/NSWindow; `set_window` from MetalTerminalView on move/resize | L | yes (with the setting on); reports always |
| Program notification words (OSC 9/99/777) and failed-command / long-command-finished banners: Swift never fills `NoticeTab.notice/failure/foreground` | terminal_view/status.rs:477-494; failed.rs:40-58 | Wormhole/Notices/NoticeFeed.swift:65-80 (+ the workspace facts to fill them) | M | yes (covered by PL03's row in name, not in its notes) |
| ⌘K Clear erases the saved copy of the terminal's text in the store | window/panes/mod.rs:83-89 | TerminalSelection.swift `clearBuffer` → workspace store `clear_scrollback` | S | yes (cleared history returns after a restart) |

## agent status, persistent terminals, launch and macOS integration

Read-only audit of the worktree `exp/swift` (GPUI reference: this worktree's `crates/`, which contains main).
Swift = `apple/swift/Sources`, FFI = `apple/ffi/src`. Paths below are relative to the worktree.

Key fact for both rows: the status rules of `terminal_view/status.rs`, `settle.rs`, `reported.rs` were ported to core
as `crates/wormhole-core/src/workspace/status/` (StatusMachine) and are fed by `apple/ffi/src/workspace_feed.rs`. The
rules themselves are faithful; the gaps are in what the shell feeds them and what reads their facts.

### T19 Agent status per terminal (summary, needs-you, working) — verdict: gaps

GPUI does:
- Output → busy unless it answers us (typing 300 ms, resize 700 ms, focus report 700 ms) or the shell is at its prompt [present: workspace_feed.rs on_event Output → core status/mod.rs on_output; typing/resize via Touch]
- 1.8 s quiet → done/idle (idle when looked at), finished mark settles at once, failure from exit code [present: core status/settle.rs, marks.rs; pump deadline workspace_pump.rs]
- Agent hooks → busy/needs input (with question words)/done/failed/quiet; Wakes (background tasks keep a turn open); the prompt's words beat the later notification [present: hook_server/messages.rs → StatusEvent::Hook → core status/hooks.rs]
- A key answering the agent's question (Return/choice/Esc) moves it on before the hook comes [present: ffi terminal.rs:250 Touch::Key → core hooks.rs answer]
- Agent detection by foreground process, forgotten when the shell is back, hook-driven status handed back to heuristics when the agent dies [present: workspace_feed.rs probe → core status/front.rs]
- OSC 7501 program status (records, blocked stays until it moves on, acknowledge reports) [present: workspace_feed.rs ProgramStatus + serve_asks → core status/reports.rs]
- Bell rules (agent without hooks asks; plain terminal marks bell, needs-you at most once a minute) [present: core status/rules.rs on_bell]
- Title gate (one rename per 250 ms) [present: core status/rules.rs TitleGate + Tick]
- Seen: looking clears done/failed/attention/bell, not an open question [present: core status_feed.rs feed/acknowledge_on_screen; Swift setWindowState]
- Hide from Agents [present: keymap hideFromAgents → ActionRouter+Edit.swift → core toggle_agent_hidden; hook_server/terminals.rs filters]
- Gauge (model, context %) per terminal in the title row [present: messages.rs Gauge → HookHost.perform .gauge → GaugeBoard]
- Usage reports for the Usage tab [present: messages.rs Usage → core.usage]
- Program's own notification (OSC 9/99/777) as the bell with words, told in its words [partial: status part present, words absent, see gaps]
- `wh notify` told in the caller's words without touching the status [wrong, see gaps]
- "Done" restored for a hosted terminal whose program finished while nobody looked (done_unseen) [absent]

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| A focus report (DECSET 1004) is never fed to the status machine: `StatusEvent::FocusReported` has no producer, so an agent without hooks (or any TUI) that redraws when Wormhole goes behind counts as work, settles into "Done" and can post a "finished" notification for nothing | terminal_view/mod.rs:460-474 (`last_focus_report`), status.rs:796-800 (`answers_us`) | ffi interaction/pointer.rs:210 `focus_changed` → `events.touched(Touch::FocusReported)`; events.rs `Touch`; workspace_feed.rs:90-98 | S | yes |
| A program's own notification (OSC 9 / 99 / 777) is never banner'd in its own words: `PtyEvent::Notify(text)` only feeds `StatusEvent::Notice` (workspace_feed.rs:70), the text is dropped (events.rs:156-170 `translate`), and `NoticeFeed.noticeTabs` never fills `NoticeTab.notice`, so the core rule `Kind::Notice` never fires | status.rs:477-493 (`on_notice`, `ProgramNotice`), notifications sweep | a notice serial+title+body in core `TerminalFacts` (or FFI), read in Wormhole/Notices/NoticeFeed.swift:65-79 | M | yes |
| Command finished / command failed notifications (long command ended by itself, `command_after` seconds; failing exit code with the command's name) never fire: NoticeFeed builds `NoticeTab` without `failure` and `foreground`, which `notify/watch.rs` needs | status.rs (failure, foreground in `TerminalSummary`) → notifications watch (ported in ffi notify/watch.rs:113-140) | Wormhole/Notices/NoticeFeed.swift:65-79 (fill `failure`, `foreground` from the row's facts) | S | yes |
| A terminal whose status a program reports (OSC 7501, e.g. `brew`) is `source .plain` with no `who` in notifications and the ambient surfaces (only `.agent`/`.plain` are mapped), so its banners are worded as a plain terminal's; per-profile notification settings (`settings: nil`) and `reported` are never set | reported.rs:1-10 (`Source::Program`), status.rs:40 | NoticeFeed.swift:73; Ambient/AmbientController.swift `ambient(_:)` | S | yes |
| `wh notify` changes the status and skips the rules: FFI feeds `StatusEvent::Notice` (bell rules: a plain terminal turns needs-you and gets the bell mark) and Swift posts the banner directly (`notifier.post`), so it shows even while the person looks at that terminal or with notifications off. GPUI: no status change, told through the notification sweep | status.rs:495-501 (`notice_from_control`), app/control/mod.rs:239-266 | ffi hook_server/control/listing.rs:63-64; Wormhole/Notices/NoticeFeed.swift:35-37 | S | yes |
| "Done" is not restored for a hosted terminal whose program finished unseen before quit: core `workspace/saved.rs` never writes `done_unseen`, nothing reads it at attach | window/session.rs:168, window/terminals.rs:225-239, status.rs:287-291 | core workspace/saved.rs + an action after `connected` in RustWorkspaceBackend+Hosting.swift (attach only) | S | yes |
| A gauge is not forgotten when the shell has its terminal back: a new run of the same agent shows the previous run's model and context % until its first status line | window/title/extras.rs:114-129 (`forget_gone_gauges`) | WindowUI/Hub/GaugeBoard.swift:22-30 | S | yes (minor) |

### T20 Persistent terminals: host attach, restore, checkpoint — verdict: gaps

GPUI does:
- New terminal in a session host when the flag is on, off the main thread, pane blank, falls back to the app with a once-per-run notice; "move to Applications" from a disk image [present: RustWorkspaceBackend+Hosting.swift start/connect, HostConnect.run, told()]
- Attach saved terminals at launch (visible ones fed a repaint, the rest headless), restart a gone host's terminal in its folder above its checkpoint or saved text [present: HostedSessions.launchPlan, restoreSaved, desk.rs plan_at_launch/attach/start/seed]
- Unavailable host: one quiet retry after 1.5 s, then the bar; Try again; Open a new terminal here; End host (soft/hard), why it failed [present: +HostProblems.swift reconnect/retryAfter/endHost/startAgain, TerminalOverlay endFailure]
- Not responding: Wait / End host; Wait spent when it answers [present: TerminalOverlay.swift:75-78]
- Lost host: "This terminal ended unexpectedly", Restart/Close [present: hostProblem .lost]
- Orphans adopted into the Recovered group (protocol fits and answers a hello) [present: desk.rs adoptable_orphans, +Hosting adopt]
- Quit lets go of every host; Quit and End All ends them; a start/attach finishing after quit is let go [present: finishQuit, connected `book.quitting`]
- Close question for a host that cannot be shown (End the Host / Leave It Running) [present: AppController+Quit.swift askToEndHost]
- Visible/headless follows what is on screen [present: syncVisibility]
- Checkpoints (Restore after restart): plan sent with the key, restored terminal's old checkpoint spent when the new one is confirmed, forgotten on close and again 5 s later, launch sweep [present: ffi hosting/checkpoints.rs]
- Screens saved each minute and at quit; erased at once on ⌘K, on "Don't keep this terminal's text", and when the setting goes off [partial: hosted terminals only, no erasing]
- Checkpoints stopped when the setting goes off at runtime [absent]

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Restore terminal text after restart (setting on by default) does nothing for terminals in the app's process, which is every terminal in a release (Persistent terminals is off in release): no text is saved each minute or at quit, and a restored terminal starts blank instead of above its saved text | window/save.rs:47-66 (`save_scrollback`), window/window_lifecycle.rs:345-361, window/terminals.rs:198 + terminal_view/mod.rs:294-301 (`seed_from_text`) | RustWorkspaceBackend+Hosting.swift `start` (the `guard let hosted` branch) + a store-level save/restore over core's `Store::save_scrollbacks` for local terminals (ffi hosting/texts.rs only covers hosted ones) | M | yes |
| ⌘K Clear does not erase the terminal's saved text at once (it stays on disk until the next minute's save) | window/panes/mod.rs:80-89 (`TerminalEvent::Cleared`) | TerminalMetal/Pointer/TerminalSelection.swift:35 → a `clear_scrollback` call through the FFI | S | no (privacy) |
| "Don't keep this terminal's text" does not erase what was saved: the core's `SavedTextForgotten` event is only logged ("The Swift app saves no terminal text", which is untrue since hosted texts are saved) | window/save.rs:68-80 (`toggle_forget_text`) | Wormhole/AppController+Windows.swift:100 | S | no (privacy) |
| Turning "Restore terminal text" off (or switching to a profile that has it off) does not erase the stored texts | app/saved_text.rs:28-48, app_state/profiles.rs:245-247 | SettingsApplier / profile switch → core `Store::erase_saved_text` through the FFI | S | no (privacy) |
| Checkpoints are not stopped (and their files not removed) when Restore after restart, Persistent terminals or the text setting goes off while running | window/mod.rs:488-496, window/restart_restore.rs:177-183, terminal_view/checkpoint_plan.rs:141-150 | Settings/SettingsApplier.swift → `HostedTerminals` stop for each made terminal | S | no |
| Persistent terminals flag read once per window (`HostedSessions.open` in the backend's init): turning it on or off applies only to windows opened later / after relaunch; GPUI decides per new terminal | window/hosting.rs:209-214 (`may_host`) | WorkspaceBridge/RustWorkspaceBackend.swift init / +Hosting.swift `start` | S | yes (flag is dev/unstable only) |
| done_unseen after attach (see T19) | window/terminals.rs:225-239 | see T19 | S | yes |

### Uncovered (no tracker row)

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| No real terminal without a developer flag: the app runs the sample workspace and mock sources unless `--shell <program>` or `WORMHOLE_SHELL` is given; GPUI always runs the person's login shell | core terminal/process.rs:123-125 (`ShellCommand::login(&login_shell())`), window/terminals.rs:190-195 | Wormhole/Options+Parse.swift:39,69; Wormhole/WorkspaceHost.swift:29-37 | S | yes |
| The shell runs as a non-login shell (`ShellCommand { args: [] }`, no `-l`), so `.zprofile` / `.bash_profile` (Homebrew's PATH, etc.) are not read | core terminal/shells/mod.rs:40-46 | ffi terminal.rs:357-372 `spawn_options` | S | yes |
| Shell choice: Settings' default shell and shell profiles, the terminal's own shell kept across relaunch (`record.shell`), a trusted project's shell. Swift keeps "New Terminal with <shell>" only in memory (`programs[id]`), and maps `newTerminalRunning` to plain `.newTerminal` for the core, so it is not saved | window/terminals.rs:184-195 (`shells::for_terminal`), features/project/detect.rs:130-152 | ffi terminal.rs `spawn_options`; WorkspaceBridge/ActionMapping.swift:13; RustWorkspaceBackend.swift:102-113 | M | yes |
| Terminal environment: the profile's environment, git identity and agent logins, `WORMHOLE_PROFILE`, the profile's start folder (`apply_profile`), and a trusted project's `env` | core terminal/process.rs:54-66, window/terminals.rs:196-201, features/project/detect.rs:130-152 | ffi terminal.rs `spawn_options` | M | yes |
| Engine options from Settings are ignored: `scrollback_lines` (fixed 10,000), image memory budget, `allow_screen_checksum`, `allow_window_ops`, `allow_title_report`; and the theme's colours and light/dark scheme are never given to the engine (at spawn or on theme change), so OSC 4/10/11/12 colour queries and `CSI ? 996 n` answer xterm's defaults (programs such as vim/neovim or Claude Code pick the wrong background) | terminal_view/mod.rs:285-292, 653-665 (`share_palette`) | ffi terminal.rs:27 (`SCROLLBACK_LINES`), 357-372; a `set_default_colors`/`set_color_scheme` call from the theme store | M | yes |
| xterm window operations (CSI t: move, resize, minimise, raise when `allow_window_ops`) and the window geometry reports a program may always ask for (position, size in pixels, Dock state): `PtyEvent::WindowOp` is dropped in the FFI and the engine is never told the window's place | window_ops.rs (whole), terminal_view/window_geometry.rs, status.rs:656 | ffi events.rs:156-170 (`translate` drops it), WindowUI | M | yes |
| Default terminal / opening files: GPUI declares shell scripts, programs and folders (`CFBundleDocumentTypes`) and opens each in a new terminal (`on_open_urls`), so `.command` double-clicks, `open -a Wormhole <folder>`, Open With and dropping a folder on the Dock icon work. Swift's Info.plist has no document types and no `application(_:open:)`, yet Settings' "Make Wormhole the default terminal" calls `setDefaultApplication` for the script types: a double-clicked `.command` would then bring up Wormhole and run nothing | packaging/Info.plist:44-74, app/default_terminal.rs:13-63 | apple/swift/Sources/Wormhole/Info.plist; AppController (an `application(_:open:)`), Platform/DefaultTerminal.swift:26-31 | M | yes |
| Privacy usage strings for programs run in the terminal (microphone, camera, contacts, calendars, reminders, photos, location, Desktop/Documents/Downloads, removable and network volumes, Apple Events): without them macOS silently denies, or kills, a CLI tool that asks | packaging/Info.plist:76-108 | apple/swift/Sources/Wormhole/Info.plist | S | yes |
| SIGTERM (`kill`) quits as ⌘Q does (saves the windows, lets go of the hosts); Swift has no handler, so the process just ends | app/sigterm.rs:11-30, app/launch.rs:143 | AppController (a `DispatchSource.makeSignalSource` → `terminateUnattended`) | S | no |
| A second launch's "Activate" opens a window when none is open | app/tick.rs:37-46 | HookHost.swift perform `.activate` | S | no (the app quits with its last window, as GPUI does) |
| Legacy upgrades at launch (agent hooks of the andromeda codename, the old default-terminal bundle id, the data folder's move): absent, moot while `Isolation.swift` refuses any data folder but `Wormhole-swift*` | app/launch.rs:55-56, 199-215 | Wormhole/AppController launch | S | no |

Checked and present: single-instance lock (HookHost.startHooks, `AlreadyServed` → the first comes forward), `WORMHOLE_TAB_ID`/`_SOCKET`/`_BIN` exported (core `launch_options`, used by the FFI's spawn), `wh`/hook command lines answered by the same executable (main.swift → `HookBridge.exitCodeIfCommandLine`), `new-terminal --command` waits for the prompt (ffi hook_server/control/open.rs), quit when the last window closes (both). GPUI injects no shell integration and has no URL schemes or Services, so neither is a gap.

## Sidebar rows (S01–S07, S09): GPUI against Swift

Read-only audit of the exp/swift worktree. GPUI paths are relative to `crates/wormhole/src/` unless they say otherwise. Swift paths are relative to `apple/swift/Sources/`. Rows, row words, second lines, VoiceOver labels, drop plans, fences and numbers come from core's headless `workspace` module through the FFI (`crates/wormhole-core/src/workspace/`), the same core the GPUI app's `view::sidebar` uses, so those are counted as present.

---

### S01 Sidebar list: rows, agent tiles, status, hint chips, count badges, wash. Verdict: gaps

What GPUI does, with the Swift code that does the same in brackets:
- Each row shows a tile (agent logo or terminal symbol) with the state glyph: working, needs-you, done, failed, bell, unread or quiet [SidebarUI/Tile/StatusTile.swift, TileState.init from core `glyph`].
- With `sidebar_status_dot` off, every tile shows the quiet state [TerminalRowView.tileState].
- The name is medium weight, or semibold when the row is selected or has unread output. A name that does not fit fades out instead of ending in an ellipsis [RowLines.name, NameText].
- The second line comes from core: agent words, the command in the code font, `exit N` in red with how long ago, the place, or "Shell". At its end sits a clock or "new output" [SecondLineView; FFI `second_line`].
- After the name: a pin mark [PinMark].
- After the name: a lock mark when the terminal is hidden from agents, with a two-line tooltip. **Absent.**
- After the name: a worktree mark (git-fork icon) with the tooltip "<name> · worktree of <repo>". **Absent.**
- At the end of line 1: the chain mark (with tooltip), the notes count and ±N, shown once per repository in repository scope [RowDetails, RowChanges.counts].
- When the name or the second line is cut off, a tooltip shows the name and "place · detail". **Absent.**
- The × appears on hover and on the selected row, with the tooltip "Close terminal", and hides while the ⌘ hints show [CloseButton].
- The × goes through the window's close question (`request_close_session`), which asks when an agent or a command runs. **Wrong in Swift:** both the × and the SwiftUI row menu's "Close terminal" send `.close(id)` straight to the core, so nothing is asked.
- A row whose menu is open wears the focus ring until the menu closes. **Absent.**
- A row moved to another group flashes for 600 ms. **Absent.**
- Status wash and line roll by `motion.status` [StatusWash, SecondLineView rolls] (the motion itself is S08).
- Rows of one split share a card, with the split's line behind the tiles [SidebarSectionView.cards, SplitLine].
- VoiceOver: core `row_label_in_split` [row.spoken], plus Pin, Rename and Close actions [TerminalRowView accessibilityAction].
- Row context menu, in GPUI's items and order [SidebarUI/Menu/RowMenu.swift; for ⌃↩, PlatformModel/Context/RowMenu.swift and Wormhole/Menus/RowMenuKeyboard.swift]:
  - The groups in "Add terminal to group ▸" are disabled when the group's fence refuses the terminal. **Absent:** every group is enabled, and the core silently refuses.
  - "Show beside…" lists only the terminals the panes have room for. **Partial:** Swift lists every terminal outside the split.
  - "Copy path" posts the "Copied" notice. **Absent.**
- Settings that change the list are all read: density, show_groups_as_cards, fade_group_colors, show_group_folders, sidebar_status_dot, sidebar_changes, sidebar_changes_scope, sidebar_notes, sidebar_cmd_hints, restore_terminal_text and motion.* [Wormhole/Settings/SidebarSettingsMaker.swift].

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| The row's × and the row menu's "Close terminal" skip the close question (agent or command running, worktree removal, unavailable host) | sidebar/row/mod.rs:144-146 → window/sidebar_host.rs:498-502 | SidebarUI/Row/TerminalRowView.swift:33 and Menu/RowMenu.swift:32 send `.close`; they should call `hub.actions.closeTerminal` (Wormhole/AppController+Quit.swift:87) through a `SidebarContext.closeTerminal` closure, as `closeGroup` already does | S | yes (a terminal running an agent closes without asking) |
| Lock mark after the name for a terminal hidden from agents, with the tooltip "Hidden from agents / Programs that control Wormhole can't see it" | sidebar/row/face.rs:200-217 | SidebarUI/Row/RowLines.swift beside PinMark (`terminal.isAgentHidden` is already in the model) | S | yes |
| Worktree mark (git-fork icon) after the name, with the two-line tooltip "name · worktree of X / path" | sidebar/row/face.rs:180-199; window/sidebar_host.rs `worktree_of` | RowLines.swift, plus a `worktreeOf` field on TerminalSummary and the FFI row (core `git::worktree_of`) | M | yes |
| Tooltip on a row whose name or second line is cut off: name, then "place · detail" | sidebar/row/face.rs:230-263, row/mod.rs:90-92 | TerminalRowView.swift `.help`, using NameText's `isCut` | S | yes |
| A row moved to another group, by a drop or a menu, flashes for 600 ms | sidebar/row/mod.rs:36-37, 62-68; sidebar/groups.rs:189-192; window/sidebar_drops.rs:80 | TerminalRowView.swift overlay, keyed by a flash id from SidebarView | S | yes |
| Focus ring on the row (or group head) whose menu is open | sidebar/row/mod.rs:94-96; group_head.rs:124-135 | TerminalRowView.swift / GroupHeadView.swift: SwiftUI `.contextMenu` reports no open state; it needs an NSMenu delegate | S | yes |
| "Add terminal to group ▸" disables groups whose fence refuses the terminal | sidebar/row_menu.rs:158; sidebar/group_head.rs:203-215 (`admits`) | Menu/RowMenu.swift:24 and RowMenuKeyboard.swift:24 (`allowed` is never set); expose core `admits` (workspace/fence.rs:65) over the FFI | S | yes |
| "Show beside…" lists only terminals the pane has room to take (`can_join` with `pane_room`) | window/sidebar_drops.rs:104-122 | RowMenu.swift:56 and RowMenuKeyboard.swift:25 filter only by split; ask the core | S | yes (a choice that silently does nothing) |
| Row menu "Copy path" posts the "Copied" notice | sidebar/row_menu.rs:222-226 | RowMenu.swift:46-49 and RowMenuKeyboard.swift:53-54 (`hub.notices.post(.copied(.path))`, as WindowHub+Title.swift:19 does) | S | yes |
| "Add terminal to new group" puts the new group's name in a field at once | sidebar/groups.rs:89-94 | RowMenu.swift:20 and RowMenuKeyboard.swift:41 send `.newGroup` only; then open the group's rename (or popover) | S | yes |

### S02 Inline rename. Verdict: gaps

What GPUI does:
- Rename… (row menu, ⌃↩ menu) turns the name into a field with the whole name selected [RenameField, RowMenu, RowMenuKeyboard `.rename` → `sidebarSearch.rename`].
- ↩ or a click elsewhere keeps the typed name, ⎋ keeps the old one, and an empty name falls back to the program name [RenameField.finish; core `rename` trims and treats empty as none].
- Afterwards the keyboard goes back to the terminal (`SidebarEvent::Renamed` → `focus_terminal`). **Absent.**
- VoiceOver has a "Rename" action [TerminalRowView].

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| After a rename (↩, ⎋ or blur) the keyboard returns to the terminal | sidebar/rename.rs:36-43; window/sidebar_host.rs:545-557 | TerminalRowView.swift `rename` closure: call `hub.focus.focusTerminal()` (needs a context hook) | S | yes (typing after a rename goes nowhere) |

### S03 Groups: bands, fold, group colour, folded summary, pinned folders. Verdict: gaps

What GPUI does:
- The head shows the name, the pin, the folder line (with show_group_folders), and a colour dot when there are no cards [GroupHeadView].
- Folded, the head shows tiles (loudest first), the count and an alert dot [FoldedSummary].
- A pinned, empty group shows "No terminals" and a + [GroupHeadView].
- The + appears on hover [HeadAddButton] and asks the folder question [SidebarView+Group.newInGroup, GroupFolderQuestion].
- A click on the head folds or opens it [GroupHeadView Button → `.toggleGroup`].
- A click on the name opens the group popover. **Absent.**
- Resting the pointer on the name shows the folder as a tooltip. **Absent.**
- The group menu [Menu/GroupMenu.swift]:
  - "Rename…" renames the group in place on its head. **Different:** Swift opens the popover.
  - The colours are a row of swatch dots. **Different:** Swift has a "Colour ▸" submenu of names.
  - "Other colour…" opens the system Colours panel directly. **Different:** Swift opens the popover's ColorPicker.
  - No colour, the folder line (greyed), Set folder…, Clear folder, Only terminals in this folder, New terminal in group, Ungroup, Close group (asks), Pin/Unpin group, Move group to new window [all present].
- The group popover: name field, a "No colour" ring and nine swatches, the custom colour, the folder items, then New terminal in group, Ungroup, Close group, Pin group, Move group to new window. **Partial:** Swift GroupPopover has the name, nine swatches, ColorPicker, Pin, Ungroup and Close.
- When the popover closes, the keyboard goes back to the terminal. **Absent.**
- ⌃↩ on a group head opens its menu. **Absent:** SidebarView+Keys.swift:20 handles rows only.
- The group head is dragged and dropped through the core's group_drop_plan [GroupHeadView.dragSource, SidebarView+Drag].
- VoiceOver: core `band_label` [head.spoken].

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| A click on the group's name opens its popover; the rest of the head folds it | sidebar/group_head.rs:101-104, 185-199 | Group/GroupHeadView.swift `label`: a separate tap target that calls `context.openPopover(head.id)` | S | yes |
| In-place group rename on the head (Rename… from the menu); ↩ or blur keeps, ⎋ cancels, blank keeps the old name, then the keyboard goes to the terminal | sidebar/rename.rs:63-140; group_head.rs:106 | GroupHeadView.swift `label`: a field like RenameField; GroupMenu.swift:15 opens the popover instead | M | yes |
| Popover lacks the folder lines (folder, Set folder…, Clear folder, Only terminals in this folder) and the actions New terminal in group and Move group to new window | sidebar/group_popover.rs:115-158, 198-201 | Group/GroupPopover.swift (the GroupMenu.swift `folder` view can be reused) | S | yes |
| Popover lacks the empty "No colour" ring first among the swatches | sidebar/group_popover.rs:64-89 | GroupPopover.swift:28-35 | S | yes |
| The keyboard returns to the terminal when the popover closes (unless a rename took it) | sidebar/group_popover.rs:202-211 | SidebarSectionView.swift popover `onDismiss` | S | yes |
| Group menu: the colours as an inline swatch row; "Other colour…" opens the system Colours panel directly | sidebar/group_popover.rs:224-229 | Menu/GroupMenu.swift:17-25 (a native NSMenu custom-view item; `NSColorPanel`) | M | yes (looks) |
| Tooltip on the group name with its full folder | sidebar/group_head.rs:81-86 | GroupHeadView.swift `.help(head.folder)` | S | yes |
| ⌃↩ on a group head opens the group menu | sidebar/row_menu.rs:71-79; group_head.rs:148-157 | SidebarUI/Keys/SidebarView+Keys.swift:20 (rows only); AppController needs an `openGroupMenu` like RowMenuKeyboard | S | yes |
| The + tooltip names the group ("New terminal in <name>", core `spoken::new_in_group_label`) | sidebar/group_head.rs:72 | Group/HeadControls.swift:39-40 says "New terminal in group" | S | yes (minor) |

### S04 Densities Regular, Compact, Dense. Verdict: complete as far as the source shows

- The three densities' row height, tile, gap, inset and card metrics follow the `density` setting, and only Regular has the second line [DesignSystem/Scale/SidebarMetrics.swift, RowLines `density.twoLines`, SidebarSettingsMaker]. Compared with GPUI renders per the tracker's evidence. No gaps found.

### S05 Pins; ⌘1–9 numbering; ⌘-hold hint. Verdict: gaps (tab-bar side)

What GPUI does:
- Pin to top / Unpin pins a split as a whole and lists it first [core `toggle_pin`; RowMenu, the `pinTerminal` action].
- Pin mark [PinMark].
- ⌘1–9 select by core's number, which follows `sidebar_numbers` [ActionRouter+Terminals `.terminal(n)` → `selectNumber`].
- Holding ⌘ alone for 250 ms shows keycaps in place of the ×; any key, a release or blur hides them; `sidebar_cmd_hints` turns this off [CommandHold, CommandHoldMonitor, RowLines.trail].
- The same keycaps on the tabs in tabs mode. **Absent.**

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| In tabs mode, holding ⌘ shows the ⌘N keycaps on the tabs (in place of the ×) | sidebar/tab_bar.rs:264, 277-281 | WindowUI/TerminalColumn.swift:37 never passes `hints:` to TabBarView, so it is always false; pass the CommandHoldMonitor state | S | yes |
| A tab's number is core's ⌘ number (stable per terminal, folded rows included) | sidebar/tab_bar.rs:111-121 | WindowUI/Hub/WindowHub+Sidebar.swift:31 uses the position in `state.terminals` (visible rows only), which is wrong with folded groups or `sidebar_numbers` = terminal; use `SidebarRow.number` | S | yes (when hints show) |

### S06 Show filter and search. Verdict: no GPUI feature to match (the row overstates parity)

- The GPUI app has no sidebar search field and no Show filter. Commit a9f61dd52 ("drop folder grouping, the filter and tab layouts") removed them; `view/sidebar/listing.rs:17-19` says a saved filter is ignored. The only search in GPUI's terminal list is the tab bar's group switcher field (S09).
- The Swift sidebar search (SidebarUI/Search/*, core workspace `Action::Search`) is a Swift-only addition. Its ⌘F, while the sidebar has the keyboard, opens this search, where GPUI's ⌘F is the terminal's Find. The tracker's 640 weight counts GPUI lines that no longer exist.

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| (divergence) ⌘F with the list focused opens the sidebar search, where GPUI opens the terminal's Find bar | keys/keymap/table.rs (Find, terminal scope) | SidebarUI/Search/SidebarSearch.swift | S | yes |

### S07 Drag reorder, drop into group, split drop, tear off. Verdict: gaps

What GPUI does:
- Dragging a row lifts it to 40 %, shows a ghost and marks where it would land. The landing, joining a split and fences are core's [SidebarView+Drag, DropMarkView, `dropPlanner.dropPlan`].
- A group head is dragged the same way [`groupDropPlan`].
- ⎋ cancels a drag [DragEscape].
- A row dragged onto the panes joins at a pane edge [PaneArea+SidebarDrag].
- Tear off: a chip outside the window, then a new window or another window's list [AppController+TearOff.swift, behind the `tear_off` flag].
- A pane's header dragged over the list lands like a row, and dropped below the rows takes the terminal out of its split. **Absent:** the Swift header is a system `.draggable` and the list has no drop target.
- The moved row flashes (see S01).

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| A pane header dragged onto the sidebar list moves or joins like a row drop; dropped below the rows (or on a row that takes nothing) it leaves its split | sidebar/drop.rs:6-8; sidebar/terminal_drag.rs:23 (`DragOrigin::PaneHeader`); window/sidebar_drops.rs:17-55 | SidebarUI/SidebarView.swift: a `.dropDestination` for PaneHeader's payload (PanesUI/Panes/PaneHeader.swift:46) that maps the point to DropLines and the core's dropPlan | M | yes |

### S09 Terminals as tabs: tab bar, group switcher, profile chip. Verdict: gaps

What GPUI does:
- The bar shows when there is more than one set, or the current set has more than one terminal [TabBarModel.make].
- Tabs scroll and bring the selected tab into view [TabBarView].
- A tab has its tile, name with a fade, the × with the close question [TerminalTabView; `tabActions.close` → `closeTerminal`].
- The + makes a new terminal in the current group [TabBarView `actions.newTerminal(currentGroup)`].
- Right-click on the + gives the "New terminal with" shells menu. **Absent.**
- A tab's context menu (the row menu). **Absent.**
- The tab tile shows the agent logo and the status glyph or light. **Partial:** Swift draws the terminal symbol with an activity corner (PanesUI TerminalTile).
- Tab VoiceOver label: name, state, "n of m". **Partial:** Swift gives the name and selection only.
- Opening a set goes to its last-selected terminal; an empty pinned group gets a new terminal. **Partial:** Swift goes to the first member, and does nothing for an empty group.
- ⌃⌥[ ⌃⌥] ⌃⌥1–9 step through the sets in both layouts, with Ungrouped last, folded groups included and the last selection remembered. **Partial:** Swift WindowHub+Menu.selectGroup counts open headed groups only and goes to the first row.
- The switcher popover: search with hits and ↩ to the first [GroupSwitcherView], cards with an alert dot and count [SetCardView], "Show all groups in the sidebar", New group.
- The switcher card's name opens the group popover, a right-click opens the group menu, a pinned empty card has a +, and the cards carry the group's wash, its colour and agent-logo tiles (loudest first). **Absent.**
- The footer key hint ("⌃⌥[ / ] previous, next group · ⌃⌥1–9"). **Absent.**
- "New group" makes a new terminal in a new group of its own. **Wrong:** Swift sends `.newGroup([])`, which the core refuses (workspace/groups.rs:24-26), so nothing happens.
- "Show all groups in the sidebar" and View › Show Terminals in Sidebar write `terminals_in` to the settings (every window, saved). **Wrong:** Swift flips `look.tabsOnTop` in memory only.
- The profile chip on the tab bar [ProfileChip + ProfileMenuItems: switch, open window in, move window to, new, manage, settings].
- The profile chip in the sidebar's title row (sidebar mode, two or more profiles). **Absent.**

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Switcher "New group" opens a new terminal in a new group (`NewTerminal::InNewGroup`); Swift's `.newGroup([])` is refused by the core, so nothing happens | sidebar/switcher.rs:197-200; window/sidebar_host.rs:503-512 | WindowUI/Hub/WindowHub+Sidebar.swift:40 should send `.newTerminalInNewGroup` (already mapped, never sent) | S | yes (button does nothing) |
| "Show all groups in the sidebar" and the Show Terminals in Sidebar/Tabs toggle persist `terminals_in` | sidebar/switcher.rs:495-503; app/handlers.rs:138-145 | Wormhole/AppController+Windows.swift:89 and Actions/ActionRouter+View.swift:18 set `look.tabsOnTop` only; write the setting | S | yes (back to tabs at next launch) |
| A tab's context menu (the row menu) | sidebar/tab_bar.rs:309 | PanesUI/Tabs/TerminalTabView.swift: `.contextMenu`, or reuse `openRowMenu` (Wormhole/Menus/RowMenuKeyboard.swift) | S | yes |
| Profile chip in the sidebar's title row (sidebar mode, two or more profiles) | window/chrome.rs:168-174; sidebar/profile_chip.rs | WindowUI/SidebarTitleRow.swift (holds only the +) | S | yes (with two profiles, no chip unless tabs mode) |
| Right-click on the + (sidebar title row and tab bar) opens "New terminal with" and the shells, the default checked | sidebar/shells.rs:21-53; window/chrome.rs:176-186; sidebar/tab_bar.rs:354-375 | WindowUI/SidebarTitleRow.swift and PanesUI/Tabs/TabBarView.swift:45 (the choices are in SettingsModel/ShellChoice) | S | yes |
| Opening a set goes to its last-selected terminal; an empty group gets a new terminal | sidebar/tab_bar.rs:204-219 | WindowUI/Hub/WindowHub+Sidebar.swift `openSet` (first member; empty does nothing); core keeps `last_selected` but exposes no action | S | yes |
| ⌃⌥[ ⌃⌥] ⌃⌥1–9 walk the tab sets (Ungrouped last, folded groups included, last-selected terminal, empty group gets a new terminal) | sidebar/tab_bar.rs:227-245 | WindowUI/Hub/WindowHub+Menu.swift:34-50 (open headed sections only, first row) | S | yes |
| Switcher cards: name opens the group popover (and the rename field), right-click opens the group menu, + on a pinned empty card, the group's wash, its colour, agent-logo tiles loudest first | sidebar/switcher.rs:380-460 | PanesUI/Tabs/GroupSwitcherView.swift `SetCardView` | M | yes |
| Switcher footer with the keys ("⌃⌥[ / ] previous, next group", "⌃⌥1–9") | sidebar/switcher.rs:278-299, 505-516 | GroupSwitcherView.swift footer | S | yes |
| A tab's tile shows the agent logo and the status glyph or light, as the row's tile does | sidebar/tab_bar.rs:276 (`tile_for`) | PanesUI/Tabs/TerminalTabView.swift:22 uses PanesUI TerminalTile(activity:) | S | yes (looks) |
| A tab's VoiceOver label carries its state and "n of m" (core `spoken::tab_label`) | sidebar/tab_bar.rs:267-268 | TerminalTabView.swift:40 | S | VoiceOver only |

### List keyboard (shared by S01/S03/S05). Gaps found while auditing

What GPUI does:
- ↑ ↓ ← → move, open and fold; ⌥↑ ⌥↓ go to the ends; type-ahead jumps by name [SidebarView+Keys, core rules].
- ⌃↩ opens the row menu [present for rows].
- ↩ opens the terminal and gives it the keyboard.
- Space shows the terminal but keeps the keyboard in the list (browsing).

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Space on a row shows the terminal and keeps the keyboard in the list; ↩ gives the keyboard to the terminal, even when it is already the selected one | sidebar/keys.rs:152-160; window/sidebar_host.rs:488-497 | SidebarUI/Keys/SidebarView+Keys.swift:39-41 sends `.select` for both keys. WindowUI/Overlay/TerminalHost.swift:57 takes first responder whenever a new terminal is mounted, so Space loses the list, and ↩ on the already-selected row keeps it | S | yes |

---

### Uncovered (no tracker row)

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| "New terminal with ▸" shells menu on a right-click of the sidebar's and the tab bar's + (Terminal.app's "New Tab with Profile") | sidebar/shells.rs:1-53 | WindowUI/SidebarTitleRow.swift, PanesUI/Tabs/TabBarView.swift | S | yes |
| Worktree mark on rows: a terminal sitting in a git worktree shows the fork icon and "worktree of <repo>" (no row covers worktree awareness in the list; P05 is the panel's worktree actions) | sidebar/row/face.rs:180-199; window/sidebar_host.rs:66-76 | SidebarUI/Row/RowLines.swift + an FFI row field | M | yes |

## Studio and crosscut (SD01-SD03, X01, X02, X05, X06, X08-X11)

GPUI paths are under `crates/wormhole/src/` unless they start with `crates/`; Swift paths under `apple/swift/Sources/`, FFI under `apple/ffi/src/`.

### SD01 Theme Studio — verdict: gaps

GPUI does:
- Opens on the theme in use: own file in place, a built-in as "Untitled (from X)" with a copied `base` layer (format version 1), a trusted project's `theme.json` as the project's theme (`studio/opening.rs:50-170`) [partly: `StudioModel/Document/StudioDocument+Open.swift:open/untitled`, own file or a v0 `extends` copy only]
- Token list: filter field, All / Yours / Base / Low contrast, bands, swatch, origin tag, reach chip clickable on the row (`studio/token_list.rs`, `token_rows.rs`) [partly: `StudioUI/List/TokenListHeader.swift`, no Base]
- Token editor: name, group, purpose, colour picker or metric stepper + slider, expression field checked per keystroke with completions (arrows, Tab, Return, Esc), rule, used by, contrast pairs, low-contrast card with fixes and Show (`token_editor.rs`, `completion.rs`, `contrast.rs`) [present: `StudioUI/Editor/*`; completions take Tab and a click only]
- Undo/redo with step names, Revert to Saved, Versions sheet (preview on select, Restore) [present: `UndoButtons.swift`, `StudioDocument+Edit.swift:undo/redo/revertToSaved`, `VersionsSheet.swift`]
- Save (⌘S), name asked first for a copy, changed-on-disk question, Export JSON, Copy JSON, Reveal in notice [present: `StudioSession.save/write`, `StudioSession+Export.swift`, `NoticeRow.swift`]
- Light | Dark control for a pair [present: `Title/AppearancePicker.swift`]
- Close with edits asks Save / Don't Save / Cancel [present: `StudioWindowController.windowShouldClose` -> `StudioSession.requestClose`]
- Theme menu: Your Themes (checked), This Project, New Theme…, New Theme From ▸ (yours, then built-ins by family, Wormhole's under one submenu), Duplicate (⇧⌘S), Rename…, Move to Trash…, Add Light/Dark Appearance, Update from X…, Revert to Saved, Revert To ▸ Browse All Versions (`studio/theme_menu.rs:708-779`) [partly: `Title/ThemeNameMenu.swift`: your themes, flat built-ins, Rename, Save copy, Revert, Versions, Trash]
- Save menu: Export JSON, Copy JSON, Use in this project (`title.rs:52-56`) [partly: `Title/SaveButton.swift`, no Use in this project]
- Title chips: Update-from button, "newer Wormhole" chip, "Preview" chip with Use (`title_chips.rs:27-127`) [absent]
- Saving sets the theme as the pick of its appearance in `config.json` unless it already is one; a rename makes picks follow (`save.rs:166, 230`; `settings/picks.rs:27`) [wrong: `StudioDocument+Save.swift:39` sets `ThemeStore` in memory only]
- Esc and ⌘W close; ⌥⌘, toggles the window (`studio/mod.rs:196, 538-563`) [partly: ⌘W via the window's close; `ActionRouter+App.swift:18` only shows]
- Window text follows `settings_text_size` (`studio/shell.rs:26`) [absent]

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Theme file format version 1 (copied `base` layer, `id`): Swift skips any file with `version` when opening (opens a GPUI-made theme only as a "Custom" copy) and writes `extends` copies; GPUI copies built-ins with a base | `studio/opening.rs:122-170` (`untitled`, `with_base`, `editable_in_place`); `crates/wormhole-core/src/theme/tokens/file.rs:334` | `StudioModel/File/ThemeFile.swift`, `StudioDocument+Open.swift:open` (guard `root["version"] == nil`) | L | yes |
| Update from the source theme (changed values list, apply as one step, menu item states, title button) | `studio/update_from.rs:47-204`; `theme_menu.rs:758`; `title_chips.rs:89` | new `StudioModel/Document/StudioDocument+UpdateFrom.swift`, `ThemeNameMenu`, `TitleBar` | M (needs v1 format) | yes |
| Project theme: open on it when in effect, This Project menu entry, Use in this project (writes `.wormhole/theme.json` in a trusted project), Save of a project subject | `studio/opening.rs:79`, `mod.rs:254`, `save.rs:101-108, 342-377`, `theme_menu.rs:717` | `StudioSession`, `ThemeNameMenu`, `SaveButton` + an FFI call to the project folder | M | yes |
| New Theme… (neutral starter of the appearance in use) | `studio/opening.rs:145`, `theme_menu.rs:738` | `StudioDocument+Open.swift`, `ThemeNameMenu` | S | yes |
| New Theme From ▸ (your themes, then built-ins grouped by family, Wormhole families in one submenu) | `studio/theme_menu.rs:684-706` | `ThemeNameMenu.swift` | S | yes |
| Add Light / Dark Appearance (make a single theme a pair) | `studio/commit.rs:126-170`, `theme_menu.rs:727` | `StudioDocument+Edit.swift`, `ThemeNameMenu` | M | yes |
| Preview chip with Use (makes the edited theme the pick, saving first) and "Use with Match system" in the notice for a saved pair | `studio/title_chips.rs:109`, `use_theme.rs:76-111`, `save.rs:396` | `TitleBar`, `NoticeRow`, `StudioSession` | M | yes |
| Saved theme becomes the pick in `config.json` (only when not a pick already); picks follow a rename. Swift only sets `ThemeStore` (lost at relaunch, Settings shows the old pick, and `setTheme` also moves the mode, which GPUI never does) | `studio/save.rs:166-176, 230-251`; `settings/picks.rs:27` | `StudioDocument+Save.swift:save` -> write `theme`/`theme_light` through the settings backend | S | yes |
| Duplicate (⇧⌘S) then edits the copy; Swift writes the copy and keeps editing the original | `studio/save.rs:253-281` | `StudioDocument+Save.swift:saveCopy`, `StudioSession.saveCopy` | S | yes |
| Saving under a name the folder has asks Replace / Cancel; Swift refuses with a notice | `studio/save.rs:116-131, 463-490` | `StudioSession.write`, `StudioQuestions` | S | yes |
| Newer-format guard: a file from a newer Wormhole is never written back, "newer" chip with the count skipped, banner offers Duplicate | `studio/save.rs:143-149, 414`; `title_chips.rs:32` | `StudioDocument+Save.swift:save`, `TitleBar` | S | yes |
| Base filter and the third origin "base" (Yours / Base / worked out) | `studio/token_list.rs:394`, `draft.rs:201`, `origin_tag.rs:23-48` | `StudioModel/Tokens/TokenFilter.swift`, `TokenOrigin.swift`, `TokenRowView` | S | yes |
| Esc closes the window (after leaving rename or the menu); ⌥⌘, closes an open Studio | `studio/mod.rs:196-205, 551-560` | `StudioView` `.onExitCommand`, `StudioLauncher.show` -> toggle | S | yes |
| Completion list keys: ↑ ↓ move, Return takes, Esc closes the list before the window | `studio/completion.rs:52-100` | `StudioUI/Editor/ExpressionField.swift` | S | yes |
| Studio text size follows `settings_text_size` | `studio/shell.rs:26, 60, 71`; `style/text_scale.rs` | `StudioView` (as `SettingsView.swift:42` does) | S | yes |
| Export remembers the last folder (store meta `EXPORT_DIR_KEY`), notes when only the shown appearance went out | `studio/save.rs:284-323, 535-551` | `StudioOutputs.system`, `StudioDocument+Save.swift:exported` | S | yes |
| Edit menu names the step ("Undo Change “primary”") while the Studio is in front | `studio/mod.rs:336`, `publish_edit_titles` | `MenuController` | S | yes |
| Theme Studio failing to open says so (Swift writes stderr only) | `studio/mod.rs:240` (log), `studio-open-failed` in `studio.ftl:48` for a theme that cannot be opened | `StudioSession.open` (`try?` drops it) | S | yes |

### SD02 Theme Studio component gallery — verdict: gaps

GPUI does: 20 specimens (sidebar, tiles and tabs, right panel, diff, chrome, find, panes, tabs and chips, labels, shape, settings, buttons, inputs, menus, window sketch, windows, notifications, terminal, code, metrics) filtered by Area; each element marked with its token (`Env::mark`, 52 marks) so the selected token's uses get a dashed outline and a click on a labelled element selects its token; virtualised (`gallery/layout.rs`). [present: `StudioUI/Gallery/GalleryCatalog.swift` 24 specimens, area filter `GalleryView.AreaChips`, uses count chip]

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Per-element use marks: GPUI outlines the element drawn with the selected token; Swift outlines the whole specimen frame | `studio/gallery/mod.rs:59` (`mark`), used 52 times across `gallery/*.rs` | `StudioUI/Gallery/SpecimenFrame.swift`, each specimen | M | yes |
| Click on any marked element selects its token; Swift only in the Terminal and Code swatches | `studio/gallery/mod.rs:59-80` | `GalleryContext.select` in each specimen | M | yes |
| Settings specimen: setting rows in their states under a group head, the nav items, the arranged (reorder) list | `studio/gallery/settings.rs:151, 282, 446` | new `SettingsSpecimen` (Swift has controls, inputs, buttons, menus only) | M | yes |
| Right panel specimen parts: Project tab two-line rows and trust callout, file and folder rows, icon tabs, line counts | `studio/gallery/panel.rs` | `PanelSpecimens.swift` (notes, changes, history, diff only) | M | yes |

### SD03 Theme library — verdict: gaps

GPUI does: bundled and user themes (themes folder, JSONC, pairs by `variants`), picks `theme`/`theme_light` by name, path or slug, mode Dark/Light/System, picks persisted in `config.json`, user theme files and `config.json` followed by file events and re-resolved on change (`app/own_files.rs`, `crates/wormhole-core/src/config/mod.rs:265-288`), a trusted project's `.wormhole/theme.json` layered over (`settings/mod.rs:463-486`), import and trash (Settings), icon themes from `icon-themes/` chosen by `icon_theme` (`crates/wormhole-core/src/theme/icon_theme`). [present: bundled + user list `DesignSystem/Theme/ThemeStore+Resolve.swift:reloadUserThemes`, resolution through `FfiThemeResolver`, mode from `ThemeSync.apply`, import/trash `SettingsUI/Rows/ThemeFileActions.swift`]

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| A user theme picked in `config.json` is shown. Swift's `ThemeSync.apply` looks the pick up in the bundled library by name only, so a user theme chosen on the Settings card (value = its id), by a hand edit, or present at launch never reaches `ThemeStore` | `settings/mod.rs:239` (`derive` from `config.shown_theme`); `crates/wormhole-core/src/config/mod.rs:290` | `SettingsUI/ThemeSync.swift:14-18` (also match `store.userThemes` by id, name and path) | S | yes |
| Editing the theme file in use (editor, script) re-colours the app. The FFI follower notices (`settings/state.rs:poll` -> `config.poll` stamps the theme) but `SettingsApplier.apply` reacts only to changed keys, so `ThemeStore` never re-resolves | `app/own_files.rs:213-221`; `crates/wormhole-core/src/config/mod.rs:265-288` | `Wormhole/Settings/SettingsApplier.swift:48` (refresh the store on a revision with no key change) or a revision flag from the FFI | S | yes |
| Project theme overlay: a trusted project's `.wormhole/theme.json` (theme switched on) colours its windows; Studio edits it | `settings/mod.rs:463-486`; `features/project` hub | `ThemeStore` + project hub (`FilesBridge/Project/RustProjectBackend.swift`) | M | yes |
| Icon themes: user `icon-themes/` folder (Zed format and extension folders) and the `icon_theme` setting, including "none"; Swift always uses the bundled Wormhole Icons | `crates/wormhole-core/src/theme/icon_theme`; `/Users/benjamin/Projects/terminal/repo-docs/docs/icon-themes.md` | `WindowUI/PanelBody.swift:23` (`static let icons = BundledIconTheme()`), `FileIconTheme` | M | yes |
| Picks set outside Settings are kept: the onboarding Look step also calls `store.setTheme` without writing `config.json` (A01's row; same fix as SD01's) | `onboarding` look step -> `SettingsStore::update` | `AuxWindows/Onboarding/LookStep.swift:27` | S | yes |

### X01 Design language — verdict: gaps

GPUI does: `style/` shared components (rows, pills, section labels, count badges, cards, icon buttons, banners, key caps, segmented, tabs, sliders, setting rows, choice cards, tiles, path crumb, size readout, window sketch, rolling number, icon motion), tokens, 4-pt grid, `TextRole`, radii. [present: `DesignSystem/` (Row, IconButton, Banner, KeyCap, CountBadge, SectionLabel, Segmented, StatusBadge, StatusDot, Scale/*, Motion) and module-local views; screens read `@Environment(\.tokens)`]

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Icon motion on buttons, gated by "Animate icons" (`icon_animations`) and Reduce Motion: + quarter turn, refresh/reset arrows spin, copy fills to a check, split icons fill the half the new pane takes, icon swap shrink/grow, menu chevron flips while open, right panel tab glyph plays. Swift has only the column strip fill (`WindowBody.swift:57`), a copy check in Settings (`CopyButton.swift`) and a symbol replace on the pane button | `style/icon_motion/mod.rs:36-61`, `style/buttons.rs:145, 186, 317`, `style/controls.rs:94`, `window/title/mod.rs:333`, `window/panes/render.rs:296` | `DesignSystem/IconButton.swift` (motion by icon, read `WindowLook.iconAnimations`), `WindowUI/PanelTabBar.swift` | M | yes |

### X02 Theme application — verdict: gaps

GPUI does: tokens to colours (`theme_bridge::token`, magenta for a missing name), GPUI Kit theme from tokens, syntax highlight theme, terminal palette with program overrides, translucent-text judgement, accent picks, Borders, interface font; and tells each terminal's engine the theme's 16+3 colours and light/dark so the engine answers OSC 4/10/11/12 queries and `CSI ? 996 n`, and notifies mode-2031 subscribers on a change (`terminal_view/mod.rs:290-291, 653-665`; `palette.rs:137-160`). [present: `ThemeStore+Resolve.swift:refresh` over the Rust resolver, `ThemeSync.applyAccents`, `TranslucentText.swift`, `TerminalCore/Palette.swift`]

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Engine gets the theme's default colours and colour scheme at spawn and on every theme change; without it OSC 10/11 queries are answered with the engine's defaults, `CSI ? 996 n` has no answer and mode 2031 programs (Neovim, Helix) never hear of a light/dark switch. Nothing in `apple/ffi/src` calls `set_default_colors` or `set_color_scheme`, and `spawn_options` leaves `colors`/`color_scheme` unset | `terminal_view/mod.rs:290-291, 657-665`; `crates/wormhole-term/src/handle/view.rs:125` | `apple/ffi/src/terminal.rs:358` (`spawn_options`) plus an exported setter; `Wormhole/Look/TerminalLook.swift` calls it when the theme changes | M | yes |

### X05 Reduce Motion, Reduce Transparency, Increase Contrast — verdict: gaps

GPUI does: Reduce Motion stops cursor and text blink, text zoom easing, scroller reveal fade, readout fade, notice entrance, column slide, tile and status motion, status item animation, sidebar motion; Reduce Transparency and Increase Contrast make every window opaque and pick the high-contrast token variant; Increase Contrast also outlines buttons, banners, badges, cards, segmented/tabs, sliders, setting rows, the shortcut recorder, text fields, choice cards, selects and draws the title row's project tint as a line. [present: Reduce Motion consumers across Swift (`TerminalMetal/CursorBlink.swift`, `TextBlink.swift`, `MetalTerminalView+Zoom.swift`, `SizeReadoutChip.swift`, `NoticeTiming.swift`, `WindowMotion.swift`, `StatusItemController.swift`, sidebar); opacity `WindowUI/WindowLook.swift:60`, `SecondaryWindow.swift:40`; high-contrast tokens `Theme.swift:14`; outlines on `Banner`, `KeyCap`, `StatusBadge`, `SettingGroup`]

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Increase Contrast outlines on the custom components Swift draws itself: choice cards (theme cards, onboarding answers, chosen one in the primary), segmented controls and tab selections, cards and section labels/badges, the diff preview card | `style/choice_card.rs:39-40`, `style/controls.rs:204, 271, 384`, `style/labels.rs:76, 218`, `features/git/diff/preview.rs:72` | `DesignSystem/Segmented.swift`, `SettingsUI/Rows/ThemeCards.swift` (`ThemeCard`), `AuxWindows/Onboarding/*` cards, `DesignSystem/SectionLabel.swift` | S | yes |
| Increase Contrast turns the title row's project colour tint into a line | `window/title/mod.rs:151` | `PanesUI/Title/TitleRowView.swift` | S | yes |

### X06 Light, dark, system appearance — verdict: complete as far as the source shows

GPUI does: `theme_mode` Dark / Light / Match system from Settings, picks per appearance, the system's appearance followed live, native views (blur, title bar) in the theme's appearance not the window's. [present: `ThemeSync.apply` mode last, `SystemLook` live, `ThemeEnvironment.swift` `.preferredColorScheme(store.appearance.colorScheme)`, `WindowBackdrop.swift:followTheme`, `AppController+Look.swift:runLook appearance`]. The one wrong part (Swift's `ThemeStore.setTheme` moves the mode, GPUI never does) is listed under SD01, where it bites.

### X08 CLI and control socket handlers — verdict: complete as far as the source shows

GPUI does: every `Action` of `crates/wormhole-core/src/ipc/control` through the gate (agent access, `control_read`/write switches, peer check, budgets): list, doctor, status, notify, send-text, send-keys, mouse, new-terminal, new-window, split, focus, rename, close, events, read, blocks, wait, run, welcome (`app/control/mod.rs:88-182`); `wh` install, keep-current at launch (`main.rs:109`). [present: `apple/ffi/src/hook_server/control/mod.rs:97-133` handles the same set with the same refusals (`input.rs:114` mouse), `Wormhole/Platform/PlatformSetup.swift:30` keep-current, Settings `CommandLineControl`]

### X09 Events not polling — verdict: gaps

GPUI does: `config.json`, the theme files and the custom fonts folder followed by one FSEvents watch with a slow poll backstop (`app/own_files.rs`); git through `window/repo/watches.rs`; time words through one-shot timers (`window/clock.rs`); settle timers. [present: `apple/ffi/src/settings/watch.rs` (config + themes), git and files watches in the FFI, display link pausing `MetalTerminalView.swift`, one-shot sleeps; the theme half is the SD03 gap]

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Custom fonts folder followed: a font added, changed or removed is picked up and the windows redraw | `app/own_files.rs:213-221`; `fonts.rs:75` (`poll_custom`) | `apple/ffi/src/settings/watch.rs` `Roots` (add `fonts_dir`) and a font re-registration on the Swift side | S | yes |

### X10 Performance guards — verdict: complete as far as the source shows

GPUI: counting tests (words shaped, rows rebuilt, allocations), `perf.sh`, `profile-app.sh`, devctl `perf-profile` (`terminal_view/render/guards.rs`, `perf/`). Swift: `apple/scripts/perf-guards.sh`, `Tests/TerminalCoreTests/FrameWorkGuardTests.swift`, `TerminalMetal/AllocationCounter.swift`, `Wormhole/Driving/PerfProfile.swift`.

### X11 Window activation rules and test-instance isolation — verdict: gaps

GPUI does: every window through `window_open::open`, which in background mode opens it unfocused and sends it behind (`background.rs:43-67`); no Dock icon, notifications to the log, no bell sound, no VoiceOver announcements, no global hotkey, no status item (`background.rs:1-12`); comes forward once the person brings it forward. [present: `main.swift:33` accessory policy, `AppController+WindowOpening.swift:73` orderBack for workspace windows, `NoticeRoute.swift:16`, `Announcements.speaks`, hotkey/status item/Dock/folder picker gated by `policy.touchesSystem`, `Isolation.swift`]

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Settings, Theme Studio and About open in front in a background instance (`makeKeyAndOrderFront` / `NSApp.activate` unconditionally) | `background.rs:43-67`; `window_open.rs:24-55` | `SettingsUI/SettingsWindowController.swift:49`, `StudioUI/Window/StudioWindowController.swift:20, 39`, `AuxWindows/About/AboutPanel.swift:13` | S | no (test instances) |
| The bell's sound stays silent in background mode | `background.rs:8` | `TerminalMetal/TerminalBell.swift:25` (`NSSound.beep()` ungated) | S | no (test instances) |

### Uncovered (no tracker row)

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Icon themes (user `icon-themes/`, Zed format and extension folders, `icon_theme` setting incl. none, light/dark family) — no row names them (P02/P07 draw the bundled icons) | `crates/wormhole-core/src/theme/icon_theme`; `repo-docs/docs/icon-themes.md` | `FileIconTheme`, `WindowUI/PanelBody.swift:23` | M | yes |
| Light/dark reporting to programs (mode 2031, `CSI ? 996 n`) and theme colours for OSC 10/11/12 queries — listed under X02, but no terminal row (T01-T26) covers it | `terminal_view/mod.rs:653-665` | `apple/ffi/src/terminal.rs` | M | yes |

## aux windows and settings (A01–A06, ST01, ST09, STX1, STX3)

Read-only audit of the worktree `exp/swift` (GPUI reference: this worktree's `crates/`, which contains `main`).
Paths: GPUI under `crates/wormhole/src/`, Swift under `apple/swift/Sources/`, FFI under `apple/ffi/src/`.

### A01 Onboarding window, wormhole intro (Metal), welcome page — verdict: gaps

GPUI does:
- Welcome page then three steps (Folder, Look, Agents with trust; Primer when no agent is on PATH) [AuxWindows/Onboarding/OnboardingView.swift `stepBody`, AuxModel/Onboarding/OnboardingFlow.swift]
- Keys ← → ⏎ R Esc, Tab between buttons; a key or click while the page comes in cuts the run short [OnboardingView.swift `handle`, `.onTapGesture`, `.onExitCommand`]
- Replay button in the corner [OnboardingView.swift IconButton → OnboardingWindow.replay]
- Folder step: pick (⌘O) and drop a folder [FolderStep.swift `.keyboardShortcut("o")`, `.dropDestination`]
- Look step: theme, import offer, "How much to show" walk with digit keys [LookStep.swift, OnboardingView.swift digit branch]
- Agents step: per-agent Install, folder trust with Touch ID [AgentsStep.swift, TrustCardView.swift]
- Eight intro styles and three welcome handovers [AuxModel/Intro/IntroStyle.swift; Wormhole/Actions/ActionRouter+Edit.swift playIntro*/playWelcome*]
- Terminal welcome program `Wormhole welcome` [Wormhole/Aux/AuxController+Intro.swift `playWelcome`]
- After Start on the primer, a quiet "no agents" note at the sidebar's bottom while no terminal has an agent; its × then offers "Don't show again"; Settings → Advanced → Reset tips undoes it — ABSENT

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Sidebar "no agents" note after Start on the primer, × then "Don't show again" (dismissed tip, undone by Reset tips) | onboarding/mod.rs:500 (`render_no_agents_note`), :560 (`render_note_offer`); window/onboarding_host.rs:113; window/sidebar_host.rs:370; window/welcome_host.rs:232 | SidebarUI (bottom of the list) fed by AuxController's onboarding end; the tip key in the store the FFI's `dismissed_tips_count` already reads | M | yes |
| The welcome run's ship does not land on the program's real `>` cell (the tracker's own open item: the run plays, the landing is not aimed) | core onboarding/intro/handover.rs (Handover::Cursor target) | Wormhole/Aux/AuxController+Intro.swift `playWelcome` → IntroPlayer target | S | yes |

### A02 About and Licenses — verdict: gaps (minor)

GPUI does:
- About: icon, name, version (debug note), copyright; Esc/⌘W close [AuxWindows/About/AboutPanel.swift: the system's panel, on purpose]
- Licenses: list with search (every word in name, version or license), "N of M packages", selection kept while listed, detail with license line, repository link, license files; "No package matches."; no-file note; Esc/⌘W [AuxWindows/About/LicensesView.swift, PackageDetail.swift, AuxModel/Licenses/LicenseList.swift `search`, LicenseBrowser.swift `refilter`; AuxWindows/Shared/AuxWindow.swift `cancelOperation`]
- License texts are drawn in the person's code font and size — PARTIAL (Swift uses the system monospaced font at caption size)

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| License text in the configured code font (`fonts::code_font`), not the system monospace | about/licenses.rs:139, :190-197 | AuxWindows/About/PackageDetail.swift `body` | S | yes |

### A03 Update — verdict: gaps

GPUI does:
- Feed URL and minisign key built into a release (`WORMHOLE_UPDATE_URL`, `packaging/updater.pub`), team id, GitHub releases fallback — ABSENT in a shipped Swift build (only with `--update-feed`; key looked up in the repository)
- Check (manual and the schedule's: at launch if due, then every 6 h), anonymous count, skipped version, auto-install when `auto_update`, failed install not retried automatically [ffi aux_backends/updater.rs `check`, `finish_check`; AuxBridge/UpdateBridge.swift `apply(autoUpdate:)`]
- Download with progress, verify, swap, relaunch [updater.rs `start_install`, `restart`; Settings UpdateStatus.swift meter]
- Channel switch: call off a download of the left channel, say a build stays installed [updater.rs `set_channel`] — the note is never shown (see table)
- Security update: can't be skipped [UpdateStatus.swift `actions`]; severity line and advisory links — ABSENT
- Release notes as lines under "What's new" [UpdateStatus.swift `notes`]
- Ready chip in the front window, click restarts (asks first when something runs), × closes it for that version [Wormhole/Aux/UpdateFollower.swift `phase`, `requestRestart`] — PARTIAL
- Settings "Restart Now": asks the restart question when something runs; "Restart when idle" pending state with "Don't Restart" — WRONG in Settings
- Bundle replaced while running (Homebrew, another copy) → Installed + chip — ABSENT
- Updates window (Swift only; GPUI shows this in Settings) [AuxWindows/Update/UpdateView.swift asks the question correctly]

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| A release build has no updater: the feed URL is only `--update-feed`, the public key is looked for in the repo (`Options.findInRepository`), no `teamID` or `fallbackURL` passed | update/mod.rs:209 (`feed_config`), :27 (GITHUB_RELEASES) | Wormhole/Aux/AuxBackends.swift `updates()`, `publicKey()`: bake feed + key into the bundle (Info.plist/resources), set teamID, fallbackURL | S | yes |
| Settings → Updates "Restart Now" restarts-when-idle without asking when something runs; no "waiting to restart" status, no "Don't Restart" (cancel) button | settings_window/sections/updates.rs:198-201, :237-262; update/mod.rs `restart_when_idle`/`cancel_restart_when_idle` | SettingsUI/Panels/UpdateStatus.swift `actions` (call UpdateFollower.requestRestart; expose pending + cancel from UpdateFollower) | S | yes |
| Channel note "the Preview build was removed / stays installed" never shown (FFI returns `channel_note`, Swift drops it) | settings_window/sections/updates.rs:170-173, :306 | AuxBridge/UpdateBridge.swift `apply(_ state:)` + UpdateStatus.swift | S | yes |
| Security line with severity colour and advisory links (GHSA ids opening the advisory page) | settings_window/sections/updates.rs:141-160, :286-328 | ffi aux_backends/update_records.rs (add severity, advisories), UpdateStatus.swift | M | yes |
| App bundle replaced while running is noticed (Installed phase + chip) | update/mod.rs:288 (`watch_bundle`, core `Launched::replaced`) | ffi updater.rs + UpdateFollower | M | yes |
| Ready chip follows the front window and a closed chip stays closed for that version across windows (Swift posts once to `app.hub`, the window current then) | update/prompt.rs:36 (`keep_offered`) | Wormhole/Aux/UpdateFollower.swift `phase` | S | yes |

### A04 Session Hosts window — verdict: gaps

GPUI does:
- Header: title, Copy all, persistent-terminals flag on/off badge, host binary (found/disposable/missing), hosts folder, off note, tally attached/detached/orphan/stale [AuxWindows/Hosts/HostsHeader.swift]
- A card per host: badge, note, id, program, problems; End (asks, a sheet naming the attached terminal), Forget (stale), Copy with "Copied" [HostCardView.swift, HostsView.swift `ask(toEnd:)`, HostsViewModel.swift `copy/end/forget`]
- Folder watch plus a slow fallback look while open [HostsViewModel.swift `start`; ffi aux_backends/hosts.rs `follow`]
- Hosts named by any profile's saved terminals are not orphans [hosts.rs `named_hosts`]
- Cards fold: closed shows the badge, note, id, program and only the problem lines; ↩/Space and click open and close them — ABSENT (Swift always shows every fact)
- Connection facts for an attached host (Link state or why unavailable, Heard/Quiet) — ABSENT
- Which terminals hold a host is read at each look — PARTIAL (snapshotted once at window open)
- The flag badge is the persistent-terminals setting — WRONG (Swift: whether the selected terminal is hosted)

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Card fold/unfold with disclosure, problems kept on a folded card, ↩/Space, aria-expanded | hosts_window/row.rs:48-110, mod.rs:253 (`toggle`) | AuxWindows/Hosts/HostCardView.swift, HostsViewModel.swift | M | yes |
| Link facts: connection state, unavailable kind, "heard N ago" / "quiet for" (FFI always passes `link: None, unavailable: None`) | hosts_window/detail.rs:104-121; gather.rs:35-50 (`host_attachments`) | ffi aux_backends/hosts.rs `report` (needs the hosted client's link state), WorkspaceBridge hosting | M | yes |
| Attachments refreshed at each report (Swift sets them once when the window opens; a terminal attached or ended later shows the wrong standing) | hosts_window/gather.rs:35 | Wormhole/Aux/AuxBackends.swift `hosts()` → per refresh | S | yes |
| Flag badge reads the persistent-terminals setting (`persistent_terminals_on`), not the selected terminal's hosting | hosts_window/gather.rs:46 | Wormhole/Aux/AuxBackends.swift:55 | S | yes |

### A05 Agent access sheet — verdict: gaps

GPUI does:
- Gate per control request: mode Off/Ask/Open, identify the program, held/stored approvals, cooling after a deny, terminal scopes [ffi hook_server/gate.rs `gate`, `within_scope`]
- Question sheet: program and terminal questions, warnings (unusual signer, unsigned, ad hoc, changed since allowed, root), facts block, buttons Deny/Session/Until Quit/Always [AuxModel/Access/AccessDisplay.swift `of`, AuxWindows/Access/AccessSheet.swift]
- Queue one at a time; a sheet closed without answer is "not answered" [Wormhole/Aux/AccessDriver.swift `present`; ffi aux_backends/access/mod.rs `answer`, `withdraw`]
- "Claude Code is typing / reading" chip [ffi hook_server/drive.rs; WindowUI/Hub/WindowHub+Driven.swift]
- A mode widened outside Settings (config.json edited: Off→Ask, Off→Open, Ask→Open) keeps the earlier mode and asks "keep it?" — ABSENT (`AccessDisplay.modeChanged` exists, never called; the FFI gate reads `settings.agent_access` as is)
- A question nobody answers settles as "timed out" after `ASK_WAIT_S` (60 s) and the sheet is withdrawn — ABSENT
- The sheet goes on a window of the profile the request reaches, withdrawn when none is open — PARTIAL (AccessDriver uses `app.window` for every profile's desk)

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Widened mode from outside Settings held back and asked about (Keep / Don't Keep, 60 s, not answered = not kept) | agent_access/change.rs:41 (`reconcile`), :75 (`answered`); ask.rs:117 (`show_change`), :162, :171 | ffi hook_server/gate.rs (effective mode), ffi settings watch, AccessDriver (show `AccessDisplay.modeChanged`) | M | yes (security) |
| Question deadline: after 60 s it answers "timed out" and the sheet goes away | agent_access/ask.rs:153 (`start_timer`), :215 | ffi aux_backends/access/mod.rs (timer per shown question) + AccessDriver (dismiss the sheet) | S | yes |
| Sheet on a window of the request's profile (front if it is that profile's), withdrawn if none | agent_access/ask.rs:85 (`window_for`), :92 | Wormhole/Aux/AuxController.swift `startAccess` (window per profile) | S | yes |

### A06 Trust window and project trust notice — verdict: gaps

GPUI does:
- Header `name/.wormhole`, badge, the state's sentence; every line with what runs in `trust.command`; "N lines"; "Nothing here would run."; problem line (not open / doesn't load); approve button with Touch ID / password / "Waiting for approval…"; follows the hub (git pull) [AuxWindows/Trust/TrustView.swift, AuxModel/Trust/TrustViewModel.swift]
- Opened for another project while open: shows that project — WRONG (Swift keeps the first project's window; `AuxWindowController.show` returns the existing window unchanged)
- Lines in the code font with its features — PARTIAL (system monospaced)

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Opening the window for a second project shows that project (Swift reuses the first project's window and content) | features/project/trust_window.rs:42-55 (`open_trust_window` → `show`) | AuxWindows/Trust/TrustWindow.swift `show` (replace the model), AuxWindow.swift | S | yes |
| Lines in the configured code font and features | features/project/trust_window.rs:126, :198-200 | TrustView.swift `TrustLineView` | S | yes |

### ST01 Settings window shell — verdict: gaps

GPUI does:
- Nav (profile first, Changed, groups), search with hit counts per page, results page, no-results page, ↑↓ in nav, F6 areas, Esc ladder (menus/sheets, then search, then close), sizes, no full screen [SettingsUI/Nav/SettingsNav.swift, SettingsView.swift `escape`, `cycle`, SettingsWindowController.swift]
- Reveal a row from the palette and highlight it [SettingsUI/Pages/PageScaffold.swift `revealed`]
- Preview-flag banner with a way to Feature flags [Pages/SettingsPage.swift]
- "Set by project/.wormhole/config.json" marks [Rows/EntryRowView.swift `projectNote`, ffi files_panel/project/overrides.rs]
- Changed page: rows that differ, Reset All (confirmed) [Pages/ChangedPage.swift]; its summary count and "Copy as JSON" — ABSENT
- Problem banners: config.json error on Advanced with an "Open" button (PARTIAL: no Open button); "config.json can't be read, so changes aren't saved" on every other page — ABSENT; profiles list unreadable on Profile — ABSENT

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Changed page: "Copy as JSON" of the changed keys (with "Copied" and a VoiceOver announcement) and the "N settings changed" summary | settings_window/changed.rs:53-100, :153-160 | SettingsUI/Pages/ChangedPage.swift (+ a host call for `values.json(changed_keys)`) | S | yes |
| "config.json can't be read, so changes made here aren't saved" on every page while unreadable | settings_window/window.rs:525-533 | Pages/SettingsPage.swift banners | S | yes |
| "Open" button on the Advanced page's config.json error banner (opens it in the editor) | settings_window/window.rs:514-523 | Pages/SettingsPage.swift:19 | S | yes |
| Profiles list unreadable banner on the Profile page | settings_window/window.rs:536-547 | ffi profile.rs (surface the recovery message) + Profile page | S | yes (rare) |

### ST09 Font picker — verdict: gaps

GPUI does:
- Current font as a select drawn in its own face with "Bundled" tag; size stepper; menu search; Monospace | All for terminal and code; groups bundled/custom/installed each in its own face; hover shows in the specimen; ↑↓ ↩ Esc [FontPickerUI/FontPicker.swift, FontListView.swift, FontRowView.swift `onHover`, SettingsUI/Rows/FontControl.swift; order and scan are core's via ffi fonts.rs]
- Bundled and custom fonts registered before the first window, so a terminal set to a custom font never starts in a fallback — ABSENT (Swift registers the fonts folder only when Settings creates its `FontLibrary`)
- Fonts added/removed in the fonts folder picked up while running — ABSENT

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Register the fonts folder's fonts (and the bundled face, unless a custom one replaces it) at launch, before the first terminal | fonts.rs:48 (`init`) | Wormhole app start (call `FontLibrary`'s register path, or CoreTextFonts.register on `scanCustomFonts()` files) before TerminalMetal FontSet resolves | S | yes |
| Follow the fonts folder (added, changed, removed fonts; unregister removed) | fonts.rs:76 (`poll_custom`); app/own_files.rs watch | Wormhole (folder watch like config.json's) → re-register, FontLibrary.reload | M | yes |

### STX1 Extensions page — verdict: gaps (minor)

GPUI does:
- Probes list with state word/colour (off, on, changed, invalid, unsupported, unsealed), switch on opens the sheet, off turns off, "Approve again…", "Forget approval", notes per state, key problem banner, empty state with Show folder, Refresh header button, every Probe word through `reveal` [SettingsUI/Extensions/ProbesPanel.swift, ProbeRowView.swift, Pages/HeaderAction.swift; ffi settings/probes/{mod,words,sheet}.rs]
- Sheet: asks-for list with a switch each, Touch ID badge, combinations, "Since you approved it", Cancel/Approve, Touch ID when needed; approval failures said [ProbeSheetView.swift; ffi probes/mod.rs `approve`]
- Terminals' rules and hooks follow an approval [ffi probes/mod.rs:251, :263 `approvals_changed`]
- The folder is read again when the window comes forward with the page showing — ABSENT (Swift reads on appear and Refresh only)

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Rescan Probes when the Settings window becomes active on the Extensions page | settings_window/window.rs:183-191 | SettingsUI/Extensions/ProbesPanel.swift (didBecomeKey → `probes.refresh()`) | S | yes |

### STX3 Agents skill and MCP rows, How status works, Feature flags, unlock key — verdict: complete as far as the source shows

- Skill rows per agent with state (not installed, covered, current, outdated, foreign with tooltip) and Install/Update/Uninstall [SettingsUI/Panels/SkillList.swift; ffi settings/skills.rs]
- `claude mcp add` line with Copy [SkillList.swift:85]
- How status works, two columns and the hooks note [Panels/HowStatusWorks.swift]
- Feature flags page, changed marks and reset, nav gating, ⌃⌥⌘F unlock toggling the entry and opening the page [Panels/FlagList.swift, SettingsView.swift:93; ffi settings/flags.rs]

### Windows sweep

| GPUI window / sheet / panel | Opened from | Swift counterpart |
|---|---|---|
| Main window (`window/window_lifecycle.rs:193`) | launch, New Window, profile | WindowUI (present) |
| Intro/onboarding window | first run, Help → Welcome, Play Intro | AuxWindows/Onboarding/OnboardingWindow.swift |
| Settings (`settings_window/open.rs:55`) | ⌘, | SettingsUI/SettingsWindowController.swift |
| Theme Studio (`studio/mod.rs:235`) | View → Theme Studio | StudioUI (SD rows) |
| Session Hosts (`hosts_window/mod.rs:95`) | Help → Session Hosts | AuxWindows/Hosts/HostsWindow.swift |
| Project commands / trust (`features/project/trust_window.rs:67`) | trust notice "View all", Project tab | AuxWindows/Trust/TrustWindow.swift |
| About, Licenses (`about/`) | app menu, Help | AboutPanel (system panel), Licenses.swift |
| Quick panel (`status_item/quick_panel/mod.rs:88`) | menu bar item | Wormhole/Ambient/AmbientController.swift (PL05) |
| Torn-off terminal window (`sidebar/tear_off.rs:160`) | drag a row out | SidebarUI/Drag/TearOffChipView.swift, AppController+Windows.swift (S07) |
| Agent access sheet (`agent_access/sheet.rs:297`) | control request in Ask mode | AuxWindows/Access/AccessSheet.swift (mode-change sheet never shown: A05) |
| Close/quit/group/host questions (`dialog::ask_before_closing*`, `ask_keeping_terminals`) | close, quit, End host | AuxWindows/Questions (W08), HostsView.swift |
| Restart question (`update/restart.rs`, `dialog::ask_with_cancel`) | update chip, Settings | UpdateFollower.requestRestart, UpdateView (Settings button skips it: A03) |
| Group folder question (`window/group_folder.rs`, `ask_with_box`) | group menu | SidebarUI/Group/GroupFolderQuestion.swift |
| Worktree branch dialog and errors (`features/git/worktree/ask.rs`) | Changes panel | PanelUI/Stores/WorktreeStore.swift |
| Diff review comment and "copied instead" alerts (`features/git/diff/review.rs`) | diff | PanelUI/Diff/CommentEditor.swift, Changes/ChangesView.swift |
| Open panels: import theme, choose app, profile picture, group folder, files tab, Open Folder, onboarding folder, look-step import | Settings, sidebar, menu bar, onboarding | ThemeFileActions, OpenInControl, ProfileControl, GroupMenu, FolderPicking, FolderStep, LookStep (present) |
| Save panel: Studio export (`studio/save.rs:298`) | Studio | StudioUI/Window/StudioOutputs.swift |
| New profile sheet, Probe approval sheet, settings import review | Settings | NewProfileSheet, ProbeSheetView, ImportCards (present) |

No GPUI window or sheet lacks a Swift counterpart; the gaps are inside them (A03, A05, A06 above).

### Uncovered (no tracker row)

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Custom/bundled font registration at launch and fonts-folder watching (an app-wide service, only touched by ST09's notes) | fonts.rs:48, :76 | app start + folder watch | M | yes |
| Agent-access mode-change protection (config.json widening held back) is a security feature with no row of its own (A05 is the sheet) | agent_access/change.rs:1-90 | ffi gate + AccessDriver | M | yes |
| The "no agents" sidebar tip and the dismissed-tips mechanism behind Reset tips | onboarding/mod.rs:500-600 | SidebarUI | M | yes |

## window rows (W01, W03, W04, W06, W09, W11) and overlays (O01, O03)

Read-only audit of the worktree `exp/swift` (GPUI reference = this worktree's `crates/`, which contains main).
Paths: GPUI relative to `crates/wormhole/src/` unless they start with `crates/`; Swift relative to `apple/swift/Sources/`.
Motion that GPUI plays only with the Motion feature flag (off in a release build: `config/features.rs:249`,
`window/plays_motion.rs:14`) is not counted as a gap: the split-opening slide (`window/panes/opening.rs`), the status
bar's rolling numbers (`status_bar.rs:359`) and the column-slide variants (`slide.rs:104`).

### W01 Splits and panes — verdict: gaps (small)

GPUI does:
- ⌘D / ⇧⌘D split right/down, refused without room (`can_split`) [WindowHub+Panes.swift:split, canSplit from core `layout.menu`]
- ⇧⌘↩ zoom / unzoom; zoomed pane fills, others hidden but alive [WindowHub+Panes.toggleZoom; PaneArea.frame]
- ⌥⌘arrows focus by direction, nothing at the edge [ActionRouter+Terminals.paneActions → hub.focusPane → core]
- View ▸ Layout presets (single, columns, rows, grid), ticked/greyed by room [paneActions layout*; MenuStateMaker presets]
- Move out of split; split dissolves at one member [PaneMenuItems leaveSplit → core]
- Pane header: tile, name (rename shows), folder cut from the front, Zoom, ⋯ menu, Close; click focuses; right-click menu [PaneHeader.swift]
- Pane menu with greyed items and keys [PaneMenuItems.swift]
- Unfocused pane dimmed 72 % [PaneFrame.swift]
- Header drag: to another pane's edge moves, onto sidebar leaves [PaneHeader .draggable; PaneDropDelegate; core movePane/joinSplit]
- Divider: drag (core clamps), double-click equalize, hover light after delay, resize cursor, tooltip [PaneDividers.swift, PaneDividerView.swift]
- Esc during a divider drag puts it back (`cancel_pane_drag`) — ABSENT (see table)
- `split_gaps` setting [SettingsApplier.swift:124 → look.splitGaps]
- Keymap rows splitRight, splitDown, zoomPane, leaveSplit, focusPane*, layout* all routed [ActionRouter+Terminals.swift]

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Esc while dragging a divider restores the layout from before the drag. `PaneDragSession.cancel()` exists but nothing calls it; the Esc ladder only cancels a column drag | window/panes/mod.rs:299-308 | WindowUI/Focus/WindowFocus.swift `escape()` (call `hub.drag.cancel()` beside `cancelColumnDrag`) | S | yes |
| Zoom and Close tooltips carry their key ("Zoom pane (⇧⌘↩)", "Close pane (⌘W)") | window/panes/render.rs:210, 213 | PanesUI/Panes/PaneHeader.swift `zoomButton`/`closeButton` (`.help`) | S | yes (minor) |

### W03 Status bar — verdict: gaps

GPUI does:
- Hidden by default, `status_bar` setting, View ▸ Status Bar [SettingsApplier.swift:120; ActionRouter+View toggleStatusBar]
- Attention button: the terminal waiting LONGEST, its tile with the agent's logo, name, "needs input · 2m" ("2m" when narrow), tooltip naming who else waits + the jump key; click goes there, in any window — PARTIAL
- Counts "2 working · 1 done · 1 failed" (glyph + number when narrow), each click to the first such terminal — PARTIAL (this window only)
- Counts span every visible window of the app — ABSENT
- State of the terminal on screen ("working · 4m", "done · 3m", "failed · exit 1", "needs input", "exited") — PARTIAL
- Hosting item ("Persistent" / "In app" / "Connecting…") while persistent terminals are on [StatusHosting.swift]
- Size item "program · cols×rows", redrawn when the grid changes — PARTIAL (wrong first word)
- Odd window's profile name and colour at the left — ABSENT
- Probe status items in their tone, Probe name in tooltip [StatusProbes.swift]
- Width classes Lean/Numbers/Words (720/1100) [StatusSummary.Detail]
- Keyboard: a toolbar the F6 ring reaches, items walked by arrows [StatusBar focusRing]

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Counts and the attention button cover every visible window's terminals (other windows read on the tick); a click on another window's terminal brings that window forward | window/status_bar.rs:204-235, 387-395 | WindowUI/WindowBody.swift `statusBar(window:)` passes `state.all` (this window only); needs the app's sessions (non-parked) and `AppController.show(terminal:)` | M | yes |
| Attention button picks the terminal that has waited longest (needs-you sorted by `since`); Swift takes the first in list order | crates/wormhole-core/src/view/layout/status_bar.rs:150-158 | WindowUI/StatusSummary.swift `init` | S | yes |
| Attention tooltip: "a, then b, c and 2 more · go to the first (⌃⌘↓)" | window/status_bar.rs:277, 478-481; core status_bar.rs:186-203 | WindowUI/StatusBar.swift attention `Button` (`.help`) | S | yes |
| State item: "working · 4m", "done · 3m", "failed · exit N", "needs input"; shown at every width. Swift shows a bare "working"/"failed", "needs you", and only at the Words width | window/title/breadcrumb.rs:459-472; window/status_bar.rs:268, 600-607 | WindowUI/StatusBar.swift (state item), WindowUI/ActivityLook.swift `stateWords` | S | yes |
| Size item names what runs in front (agent or command, as the title row does), not the terminal's title: "zsh · 90×40" | window/status_bar.rs:264-267; core status_bar.rs:220 | WindowUI/StatusBar.swift size item (`selected.title`) | S | yes |
| Odd window (another profile than the shown one): profile colour dot and name at the bar's left, with tooltip "This window is in X, not the profile Wormhole is in" (Words width only) | window/status_bar.rs:299-304, 545-564; core status_bar.rs:215 | WindowUI/StatusBar.swift (leading item) + a window-profile fact from WindowSession | S | yes |
| Attention tile shows the agent's logo/monogram (mini tile with needs-you glyph); counts use CircleAlert/CircleCheck/CircleX icons | window/status_bar.rs:487-498, 516-528 | WindowUI/StatusBar.swift (StatusDot) | S | yes (looks) |

### W04 Columns — verdict: gaps (small, looks)

GPUI does:
- Sidebar/panel toggle with slide (Classic in release), cut under Reduce Motion [ColumnSlide.swift]
- Fit rules (panel yields first, terminal keeps its least width, toggle waits with a dot) [ColumnFit.swift from core ChromeSnapshot; WindowControls `marked`]
- Edge drag: width follows pointer, sticks at min, snaps closed past slack, reopens dragging back; readout "430 · max"/"Close" [WindowBody+Drag.swift via core edgeDrag/dragEdge]
- Double-click edge resets to default [ColumnHandle reset → .resetEdge]
- Esc cancels a column drag [WindowBody+Drag.cancelDrag via WindowFocus.escape]
- Widths remembered per profile, through relaunch [core; evidence s2-window-b/W04.txt]
- `column_dividers` setting [SettingsApplier.swift:125]
- Handle line lights after a hover delay; active colour while dragging — ABSENT

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Column handle: after a short hover its line takes `resize.handle.hover`; while dragged `resize.handle.active` (Swift's handle is `Color.clear`, the hairline never changes) | window/resize.rs:101-158 | WindowUI/Columns/ColumnHandle.swift `body` (+ WindowBody `edges`) | S | yes |
| Readout sits beside the pointer (x + 8, y − 8); Swift pins it under the title row whatever the pointer's height | window/resize.rs:161-174 | WindowUI/Columns/ColumnHandle.swift `ResizeReadout` | S | yes (minor) |

### W06 Profiles — verdict: gaps

GPUI does:
- Switch (chip, menu, palette): leaving profile's windows hidden with terminals running, target's restored/shown [ProfileChipBinding.swift `switchProfile(to:)`]
- New profile sheet (name, start from) [SettingsUI/Panels/NewProfileSheet.swift; RustSettingsHost+Profiles.createProfile]
- Manage = Settings ▸ Profile: name, colour, start folder, folders that open in it, env, git identity, agent logins, delete [SettingsUI/Panels/ProfileControl.swift, PanelControl "profile-delete"]
- Open Window in <profile> (odd window) [ProfileChipBinding `openWindow(inProfile:)`]
- Move Window to <profile> [ProfileChipBinding `move(_:toProfile:)`]
- Data-folder failure says so [ProfileChipBinding notice `.dataFolder`]
- Odd window draws with ITS profile's theme, fonts and settings — ABSENT
- Odd window wears its profile's colour as an edge across the top — ABSENT
- Odd window's status bar names its profile — ABSENT (W03 table)

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| An odd window reads its own profile's settings, theme tokens and fonts (`settings_in`/`store_in(&self.profile)`, root font and rem size overridden); Swift has one `settingsModel`, one `ThemeStore.shared` and one `app.look` for every window, so an odd window looks and behaves like the shown profile | window/layout.rs:233-238; window/chrome.rs:66-76; window/profile_look_tests.rs (test of the rule) | Wormhole/Settings/SettingsApplier.swift, ProfileChipBinding.swift `followProfileSettings`, WindowUI/WindowLook.swift (per-window look + theme) | L | yes |
| Profile-colour edge across the top of an odd window | window/chrome.rs:78-83 (`style::profile_edge`) | WindowUI/WindowBody.swift overlay | S | yes |

### W09 Window chrome — verdict: gaps (one)

GPUI does:
- Traffic lights at 12 pt, centred in the 38 pt row [TrafficLights.place]
- Collapse button at a fixed spot after the lights, moves to the edge in full screen; panel toggle top right with waiting dot and Zen wording [WindowControls.swift; WindowReader collapseX]
- Toggle glyph strips follow the slide [PanelGlyph fill]
- Title rows drag the window [WindowDragArea]
- Minimise, Zoom, Full Screen, Bring All to Front [StandardActions.swift]
- Title-row double-click does what System Settings says (Zoom, Minimize, or nothing) — WRONG (always zoom)

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Double-click on a title row follows the system's "Double-click a window's title bar to" (`AppleActionOnDoubleClick`: Minimize / Fill / Do Nothing); Swift always calls `performZoom` | style/title_drag.rs:21-22 (→ gpui-pre-macos `titlebar_double_click`) | WindowUI/WindowDragArea.swift `mouseDown` | S | yes |

### W11 Probe hook effects — verdict: gaps (one, small)

GPUI does:
- Notify → banner [HookHost.perform `.probeNotice` → notices.post(probe:)]
- Status item set/cleared, tone token, Probe name in tooltip [FFI probes/run.rs:175 → HookEvent.probesChanged → look.probeItems → StatusProbeItems]
- Highlight a block's tone, Fold output [FFI probes/run.rs:178-188; frames]
- OpenPath: an offer chip, click resolves in the terminal's folder and opens in the chosen editor, "not found" chip [WindowHub+Probes.offerFile/openFile/noticeActed]
- Run a program from the approved copy [FFI probes/run.rs:194]
- Paused hook → warning chip [WindowHub+Probes.probePaused]
- Reload when the flag, "never hide lines" or approvals change [FFI hook_server/mod.rs:174-178, probes/run.rs:44]
- Backstop reload about once a minute while a Probe runs — ABSENT

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Slow backstop: the Probes are read again every ~60 s while any runs, so an approved Probe changed on disk behind the app's back is noticed (and refused) without a relaunch | app/probes.rs:39-40, 72-77 | apple/ffi/src/probes/run.rs (a timer thread calling `reload`) or a one-shot timer in Wormhole/Hooks/HookHost.swift | S | partly (a changed Probe keeps its old state until relaunch) |

### O01 Command palette — verdict: gaps

GPUI does:
- ⇧⌘P toggles over the window in front; keyboard returns where it was [ActionRouter+View commandPalette; OverlayChrome]
- Scopes @ ~ # , > by prefix or Tab/⇧Tab; Esc clears, then All, then closes; ⌫ in an empty narrowed field goes to All [PaletteView+Keys.handle, typed]
- Core fuzzy match with marks [PaletteSearch installed matcher]
- Empty query: Recent + suggested actions; Terminals by urgency; Actions by section with shells, Project ▸ actions, Profiles; Settings recent [PaletteRows, CommandCatalog]
- Rows: terminal (status, detail, ⌘N), folder (terminal count, recent projects), commit, setting (section, live value), action (key, "in ~/x", "N files changed"), project action, new terminal with shell, profile switch/open/move [CommandCatalog.swift, PaletteSourcing.swift `reveal`]
- ↩ runs; ⌘↩ opens a terminal/folder/commit in a new window; ⌘C copies a setting's key; ⌘⌫ closes a terminal [PaletteView+Keys]
- Refresh every second while open [TimelineView]
- ⌘⌫ asks before ending a running terminal — WRONG
- Terminals of every visible window, folded groups included — PARTIAL

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| ⌘⌫ on a terminal goes through `request_close_session` (asks while something runs, the worktree question, closes the window when it was the last). Swift sends `.close(id)` straight to the core: a running command or agent is killed without a question | command_palette/mod.rs:472-486 | Wormhole/Platform/PaletteSourcing.swift `closeTerminal:` (use `AppController.closeTerminal(_:in:)`) | S | yes |
| Terminals in folded groups are listed (GPUI walks every session); Swift lists `sections.flatMap(\.rows)`, which leaves folded ones out, so they cannot be found, opened or closed from the palette | window/ambient.rs:96-98; command_palette/mod.rs:157-172 | Wormhole/Platform/PaletteSourcing+Listed.swift `paletteTerminals` (iterate `state.all`) | S | yes |
| Hidden windows (another profile's, parked) are left out, and window numbers count only visible ones; Swift iterates every session, parked included | command_palette/mod.rs:157 (`filter(!w.hidden)`) | Wormhole/Platform/PaletteSourcing+Listed.swift `paletteTerminals` (skip `parked`) | S | yes |
| ⌘C on a setting posts the "Copied" notice | command_palette/mod.rs:377-383 | Wormhole/Platform/PaletteSourcing.swift `copySettingKey:` (post the notice) | S | yes |
| Commits scope lists the branch log of the palette's terminal, read when it opens; Swift reuses the History tab's model, which is the filtered log while History shows a file's history or a picked rev | command_palette/mod.rs:224-246 | Wormhole/Platform/PaletteSourcing.swift `commits:` | S | yes (edge) |

### O03 Folder picker ⌘J — verdict: gaps

GPUI does:
- ⌘J / Terminal ▸ Insert Folder Path only with the flag on, over the key window's focused terminal [FolderPicking.insertPath]
- Native panel; providers typed paths, multi-level fuzzy walk, recent by frecency, git branch badges, chevrons; per-provider answers merged; stale answers dropped; decorations only for visible rows [FolderPickerDriver; FFI folder_picker]
- Disk-job gate and hung-path skipping [apple/ffi/src/folder_picker/gate.rs]
- Keys: ↩ pick, ⇥/→ into, ← up, Esc, ⌘⌫ forget a recent; crumbs clickable; notes (empty, unreadable, cut at N, reading) [PickerKeys via driver.key; FolderPickerView notes]
- Pick is pasted (bracketed) at the cursor, never cd [FolderPicking.open onPick → source.paste]
- Closes on click elsewhere [FolderPickerController.windowDidResignKey]
- Placed at the terminal's cursor cell — WRONG (guessed)
- Only pastes into the terminal it opened over; closes when another comes to front — PARTIAL
- Opening records the cwd as a visit — ABSENT
- Recent folders per profile, in the profile's database, only while the flag is on — WRONG

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| Anchor is the terminal's cursor cell from its last frame (fallback: bottom-left line of the pane); Swift fakes a cursor 16 pt in at 40 % of the pane's height, so the panel is not at the prompt | folder_picker/mod.rs:130-160 | Wormhole/Platform/FolderPicking.swift `open()` (cursor rect from MetalTerminalView, as `InputClient.firstRect` computes) | S | yes |
| The pick goes only to the terminal the picker opened over, and the picker closes (or re-anchors) when another terminal comes in front; Swift pastes into whatever `selectedView` is at pick time | folder_picker/mod.rs:178-189, 207-222 | Wormhole/Platform/FolderPicking.swift `open()` (capture the view; observe selection) | S | yes (edge) |
| Opening the picker records the terminal's folder as a visit (a terminal started there never reported it) | folder_picker/mod.rs:104-105 | Wormhole/Platform/FolderPicking.swift `open()` (`recordVisit` + `save`) | S | yes |
| Recent folders: one list per profile, kept in the profile's store (`meta` key `recent_folders`), recorded only while the flag is on and cleared the first time a folder changes with it off; every window's terminals count. Swift keeps one `recent-folders.json` at the data root for all profiles, records with the flag off, and follows only the front window's listed terminals; the GPUI app's existing list is never read | app_state/recent_folders.rs:1-15, 89; window/summaries.rs:49 | Wormhole/Platform/FolderPicking.swift `follow()`, `save()`; apple/ffi/src/folder_picker/records.rs (read/write the store's meta row) | M | yes |

### Uncovered (no tracker row)

| Behaviour | GPUI file:line | Swift place | Size | Visible |
|---|---|---|---|---|
| xterm window operations (CSI t): with `allow_window_ops` a program may resize the window so its pane gets N rows/cols, move, iconify, raise, maximise, full-screen it; position and screen-size reports are always answered from the window's real frame. Swift has no `WindowOp` consumer and nothing reads `allow_window_ops` (its settings row exists). No features.csv row names it (T07/T16/T18 do not); the terminal auditors may report it too | window_ops.rs (250 lines); window/panes/mod.rs:104-111 (`TerminalEvent::WindowOp`) | apple/ffi/src (surface the engine's WindowOp events) + a WindowUI/Wormhole handler applying them to the NSWindow | M | yes (for programs that use it) |

Settings check for these rows: `status_bar`, `split_gaps`, `column_dividers`, `terminals_in`, `title_row`, `smooth_resize`, `jump_waiting_key` each have a Swift consumer (SettingsApplier.swift:118-126). Keymap actions of these areas (splitRight … layoutGrid, focusPane*, toggleSidebar, toggleRightPanel, toggleStatusBar, commandPalette, insertFolderPath, newProfile, manageProfiles, switch/open/move profile, minimize, zoom, fullScreen, bringAllToFront) are all routed (ActionRouter*.swift, StandardActions.swift, `viewed`).


---

# All gaps, ranked

Most user-visible and largest first. Gaps that several rows share are merged and name every row. "Visible" is
yes unless the line says otherwise.

| # | Gap | Rows | Size |
|---|---|---|---|
| 1 | No real terminal by default: without `--shell`/`WORMHOLE_SHELL` the app runs the sample workspace (`Wormhole/WorkspaceHost.swift:29-37`); with one, it runs as a non-login shell (no `-l`), so `.zprofile` (Homebrew's PATH) is not read | uncovered | S+S |
| 2 | Left Option is Meta for everyone: `MetalTerminalView.optionAsMeta = .left`, never set from `option_as_meta` (core default Off), so @ { [ cannot be typed on layouts that need Option | T07 | S |
| 3 | The engine never gets Settings or the theme: scrollback fixed at 10,000, `image_memory_mib` and the three `allow_*` switches ignored; no theme colours or light/dark, so OSC 10/11/12 queries answer xterm defaults, `CSI ? 996 n` has no answer and mode-2031 programs (Neovim, Helix, Claude Code) never hear of a switch | T01, T09, X02, uncovered | M |
| 4 | Closing without the question: the sidebar row's ×, the row menu's Close terminal and the palette's ⌘⌫ send `.close` straight to the core, so a running agent or command is killed (and no worktree question) | S01, O01 | S |
| 5 | Shell choice and environment ignored: `default_shell`, shell profiles, a terminal's saved shell (New Terminal with <shell> is kept in memory only), a trusted project's shell and env, the profile's environment, git identity, agent logins, `WORMHOLE_PROFILE`, start folder | uncovered | M+M |
| 6 | "Restore terminal text after restart" (on by default) does nothing for terminals in the app's process, which is every terminal in a release: nothing saved each minute or at quit, restored terminals start blank | T20 | M |
| 7 | A user theme picked in `config.json` (Settings card, hand edit, present at launch) is never shown: `ThemeSync.apply` only looks among bundled themes by name; editing the theme file in use is noticed but not re-applied | SD03 | S+S |
| 8 | A theme saved in the Studio is set in memory only, never written as the pick (lost at relaunch, Settings shows the old one), and `setTheme` moves the light/dark mode, which GPUI never does; the onboarding Look step has the same fault | SD01, SD03, X06 | S |
| 9 | Colours a program sets (OSC 4/10/11/12, resets) are never drawn: the FFI frame has no override table; OSC 11 not painted behind the padding; OSC 12 cursor colour ignored | T01, T03 | M |
| 10 | Default-terminal role is broken: no document types and no open-file handler in the Swift app, yet "Make Wormhole the default terminal" registers it, so a double-clicked `.command` opens Wormhole and runs nothing; `open -a Wormhole <folder>`, Open With and a folder dropped on the Dock icon do nothing | uncovered | M |
| 11 | An odd window (another profile than the shown one) uses the shown profile's settings, theme and fonts: one settings model and one `ThemeStore.shared` for all windows; no profile-colour edge, no profile tag in its status bar | W06, W03 | L |
| 12 | Theme file format version 1 missing: Swift skips any file with `version` (a GPUI-made theme opens only as a copy) and writes `extends` copies; Update from source needs it | SD01 | L (+M) |
| 13 | Notifications in the program's words never fire (OSC 9/99/777 text dropped), nor do command-finished and command-failed banners (`NoticeFeed` never fills `notice`, `failure`, `foreground`); OSC 7501 programs are worded as plain terminals; `wh notify` runs the bell rules and posts past the notification rules | T19 (PL03) | M+S+S+S |
| 14 | A release build has no updater: the feed exists only with `--update-feed`, the key is looked for in the repository, no team id or fallback URL | A03 | S |
| 15 | Agent access: a mode widened outside Settings (config.json Off→Open) is not held back and asked about (`AccessDisplay.modeChanged` has no caller); an unanswered question never times out; the sheet ignores the request's profile | A05, uncovered | M+S+S |
| 16 | Custom font files registered only when the font picker loads, and the fonts folder is not watched: a custom family as the terminal font draws in a fallback at launch | T02, ST09, X09, uncovered | S+M |
| 17 | Text zoom is the selected terminal's, not the window's: other terminals, new ones and moved-in ones keep their size | T17 | M |
| 18 | xterm window operations (CSI t move, resize to cells, iconify, raise, maximise, DECCOLM) and window-geometry reports: the FFI drops `WindowOp` and never calls `set_window`, so reports answer zeros | uncovered | M (L with reports) |
| 19 | No privacy usage strings in the Swift `Info.plist` (microphone, camera, contacts, Desktop/Documents, Apple Events, ...): macOS silently denies or kills a command-line tool that asks | uncovered | S |
| 20 | Status bar: counts and the attention button cover this window only (GPUI: every visible window, a click jumps there); attention takes list order, not the longest waiting; the state item is a bare word with no time or exit code and only at full width; the size item names the title, not the program | W03 | M+S×4 |
| 21 | Theme Studio menu and title: New Theme, New Theme From ▸, Add Light/Dark Appearance, Update from, This Project / Use in this project, Preview chip with Use, newer-format guard; Duplicate keeps editing the original; a taken name refuses instead of asking Replace | SD01 | M×4+S×5 |
| 22 | Project theme overlay (`.wormhole/theme.json` in a trusted project) never applied; icon themes and the `icon_theme` setting ignored (bundled icons always) | SD03, uncovered | M+M |
| 23 | Folder picker ⌘J: placed at a guessed point, not at the cursor; recent folders one file for all profiles, recorded with the flag off, GPUI's stored list never read; opening does not record a visit; the pick goes to whatever terminal is selected at pick time | O03 | M+S×3 |
| 24 | Terminals as tabs: switcher "New group" does nothing (`.newGroup([])` is refused); `terminals_in` toggles not saved; no tab context menu; sets open on the first member, not the last selected, and an empty group gets nothing; ⌃⌥[ ] 1-9 skip folded groups and Ungrouped; switcher cards lack popover, menu, +, wash, logos; no footer hint; no profile chip in sidebar mode | S09 | M+S×8 |
| 25 | Focus reports (DECSET 1004) are not fed to the status rules: a TUI or hookless agent that redraws when the window goes behind settles into a false Done and a false notification | T19 | S |
| 26 | A pane header dragged onto the sidebar has no drop target (move, join, take out of the split) | S07 | M |
| 27 | Groups: click on the name does not open the popover; no in-place rename (Rename opens the popover); popover lacks folder items, New terminal in group, Move group to new window, No-colour ring; keyboard not returned on close; ⌃↩ on a head opens nothing; colours a named submenu, not swatches | S03 | M×2+S×6 |
| 28 | Sidebar keyboard: Space gives the keyboard away instead of keeping it in the list; ↩ on the selected row keeps it in the list; after a rename the keyboard does not return to the terminal | S01, S02 | S+S |
| 29 | Cursor: still drawn after the program exits; an unfocused split pane keeps blinking; the theme's cursor-text colour ignored | T03 | S×3 |
| 30 | Sidebar row marks and menu: no lock mark (hidden from agents), no worktree mark, no tooltip on a cut-off name, no 600 ms flash on a moved row, no focus ring while its menu is open; groups a fence refuses not greyed; Show beside lists panes with no room; Copy path posts no notice; Add to new group does not open the name | S01, uncovered | M+S×8 |
| 31 | Session Hosts window: cards do not fold; link facts always empty (`link: None`); attachments read once; the badge reads the selected terminal, not the setting | A04 | M×2+S×2 |
| 32 | Update: Settings' Restart Now does not ask and has no Don't Restart or waiting status; channel-switch note dropped; no security line or advisory links; a bundle replaced while running is not noticed; the ready chip stays on one window | A03 | M×2+S×3 |
| 33 | ⌘C typed in the terminal shows no "Copied" chip (`onCopied` never assigned); Copy path and ⌘C on a palette setting post none either | T04, S01, O01 | S |
| 34 | Mouse: pointer shape stale when a program turns tracking on or off; back/forward buttons reported as a middle click; middle/right press does not forget the program's selection (not visible) | T05 | S×3 |
| 35 | Saved text not erased at once on ⌘K, on "Don't keep this terminal's text" (only logged), or when the setting goes off; checkpoints not stopped when their settings go off | T07, T20 | S×4 (privacy, not visible) |
| 36 | Settings shell: Changed page lacks Copy as JSON and the count; no "config.json can't be read" banner on every page; no Open on the Advanced page's error; no unreadable-profiles banner | ST01 | S×4 |
| 37 | Icon motion on buttons (+ turn, refresh spin, copy to check, split fill, chevron flip, panel tab glyph), gated by Animate icons and Reduce Motion | X01 | M |
| 38 | Studio gallery: whole specimens outlined instead of each element; click selects a token only in two swatches; no Settings specimen; panel specimen lacks Project rows, trust callout, file rows, icon tabs | SD02 | M×4 |
| 39 | Onboarding: the sidebar "no agents" tip with Don't show again (and the dismissed tips Reset tips undoes) missing; the welcome ship does not land on the real `>` cell | A01, uncovered | M+S |
| 40 | Small terminal tools: ⌘F does not seed the query with the selection; typing or scrolling does not forget the ⌘↑/⌘↓ block; minimap has no fold ticks and a click does not focus the terminal; clipboard chip does not name the program, shows for off-screen terminals, and goes to the wrong terminal with chips off | T10, T12, T13, T16 | S×7 |
| 41 | Small studio items: Base filter and origin, Esc closes, ⌥⌘, toggles, completion arrows, text size, export folder remembered, Edit menu step names, open failure told | SD01 | S×8 |
| 42 | Increase Contrast: no outlines on Swift's choice cards, segmented controls and tabs, cards and labels; the title row's project tint not drawn as a line | X05 | S×2 |
| 43 | Small window items: Esc during a divider drag does not restore (`PaneDragSession.cancel` has no caller); pane button tooltips lack keys; column handle never highlights, readout not beside the pointer; title double-click ignores the system preference; Probes not re-read about once a minute | W01, W04, W09, W11 | S×6 |
| 44 | Small trust/about/extensions items: trust window for a second project shows the first; trust lines and license texts in the system monospace, not the code font; Extensions page not rescanned when Settings comes forward | A06, A02, STX1 | S×4 |
| 45 | Small input items: ⌘ chords with extra modifiers do not fall back to the plain ⌘ action; ⌘-click in the padding opens the nearest edge cell's link; no drop highlight while files are dragged over a pane | T07, T08, T21 | S×3 |
| 46 | Sidebar numbering on tabs: holding ⌘ shows no keycaps on tabs (`hints` never passed); tab numbers are positions, not core's numbers | S05 | S×2 |
| 47 | A gauge is not forgotten when the shell has its terminal back; a hosted terminal that finished unseen loses its Done; the Persistent terminals flag is read once per window | T19, T20 | S×3 (minor) |
| 48 | SIGTERM ends the process without saving windows or letting go of hosts; a second launch's Activate opens no window when none is open | uncovered | S×2 (not visible) |
| 49 | Background test instances: Settings, Studio and About come to the front; the bell's sound is not silenced | X11, T14 | S×2 (test only) |
| 50 | Tracker: S06's 640 weight counts GPUI code removed in a9f61dd52; ⌘F with the list focused opens Swift's sidebar search where GPUI opens Find | S06 | tracker fix |

# Features no tracker row covers

| Feature | GPUI source | Size | Visible |
|---|---|---|---|
| The person's login shell as the default program (and running it as a login shell) | `crates/wormhole-core/src/terminal/process.rs:123-125`, `terminal/shells/mod.rs:40-46` | S+S | yes |
| Shell choice and terminal environment: default shell, shell profiles, saved per-terminal shell, project shell and env, profile environment, git identity, agent logins, `WORMHOLE_PROFILE`, start folder | `window/terminals.rs:184-201`, `features/project/detect.rs:130-152`, core `terminal/process.rs:54-66` | M+M | yes |
| Engine start options from Settings (scrollback, image memory, title report, window ops, screen checksum) | `terminal_view/mod.rs:284-288` | S | yes |
| Theme colours and light/dark told to programs (OSC 10/11/12 query answers, `CSI ? 996 n`, mode 2031); named under X02, no terminal row | `terminal_view/mod.rs:289-291, 653-665` | M | yes |
| xterm window operations and window-geometry reports | `window_ops.rs` (250 lines), `terminal_view/window_geometry.rs` | M-L | yes |
| Opening files and folders: document types, `on_open_urls`, the default-terminal role it backs | `packaging/Info.plist:44-74`, `app/default_terminal.rs` | M | yes |
| Privacy usage strings for programs run in the terminal | `packaging/Info.plist:76-108` | S | yes |
| Custom and bundled font registration at launch, fonts-folder watch | `fonts.rs:48-136`, `app/own_files.rs` | M | yes |
| Icon themes (user `icon-themes/`, Zed format, `icon_theme` setting incl. none) | `crates/wormhole-core/src/theme/icon_theme` | M | yes |
| Agent-access mode-change protection (a widened mode held back and asked about) | `agent_access/change.rs` | M | yes (security) |
| Sidebar "no agents" tip and the dismissed-tips store behind Reset tips | `onboarding/mod.rs:500-600`, `window/sidebar_host.rs:370` | M | yes |
| "New terminal with ▸" shells menu on a right-click of the + (sidebar title row and tab bar) | `sidebar/shells.rs` | S | yes |
| Worktree mark on sidebar rows ("worktree of <repo>") | `sidebar/row/face.rs:180-199` | M | yes |
| SIGTERM quits as ⌘Q does | `app/sigterm.rs` | S | no |
| Legacy upgrades at launch (andromeda hooks, old default-app bundle id, data folder move): moot while `Isolation.swift` only accepts `Wormhole-swift*` data folders, needed before the Swift app opens the real data folder | `app/launch.rs:55-56, 199-215` | S | no |

Checked and not gaps, because GPUI has none either: secure keyboard entry, Look Up, middle-click paste,
copy-on-select, key-release reporting, Option-click to move the cursor, cursor/bold/contrast/scroll-speed settings,
a large-paste or newline-paste question, URL schemes (ssh://), Services beyond the menu, shell-integration
injection. Swift-only additions (not parity): IME, dead keys, dictation, the emoji picker, Option-drag block
selection, text drops, sidebar search. Present on both: focus reporting (DECSET 1004), bracketed paste, OSC 7, 8,
9;4, 52, 133, 7501, the image protocols, the single-instance lock, `WORMHOLE_*` variables, every window and
sheet GPUI can open (the gaps are inside them).
