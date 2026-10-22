# Where Wormhole already does it

Paths are under `crates/wormhole/src/`. Open the file and copy the pattern;
line numbers drift, so search for the quoted symbol.

Contents: App, windows, app-wide state · Views and rendering · Lists and
scrolling · Input, focus, actions · Components · Async · Tests and checks

## App, windows, app-wide state

- App start, `gpui_kit::init`, app-level actions, `cx.bind_keys`, menus: `main.rs`.
- `AppState` (a `Global`) with `app(cx)` / `app_mut(cx)`, long-lived
  `cx.spawn` loops fed by channels, and `open_workspace_window`:
  `app_state/mod.rs`.
- Reaching another window's workspace safely: `AppState::with_workspace` in
  `app_state/mod.rs`. It returns `None` for the window you are in, because
  that one is already being updated.
- Opening extra windows (Settings, Studio) with `open_window`:
  `settings_window/mod.rs`, `studio/mod.rs`; their shared
  window options and surfaces: `workspace/aux_window.rs`.
- Settings and theme as a `Global` (`SettingsStore`), read with
  `settings(cx)` / `ui(cx)`: `settings/mod.rs`. Tokens become GPUI colors in
  `theme_bridge.rs` (`token`, `hsla`, `apply` for GPUI Kit's theme).

## Views and rendering

- The window's root view and its layout (sidebar, tabs, panels, the drop
  target on the title bar): `workspace/layout.rs` and `workspace/mod.rs`.
- Shared look: `style/`. The design rules and the radius table are in
  `mod.rs`'s header; `scale.rs` has control heights, type roles and
  borders. The builders are in:
  - `controls.rs`: `row`, `pill`, `icon_button`, `icon_tab`, `segmented`,
    `keycap`, `close_x`
  - `labels.rs`: `section_label`, `count_badge`, `card`, `panel_title`
  - `section.rs`: section headers and items
  - `setting.rs`: `setting_row`, `toggle`, `choice`, `stepper`
  - `reorder.rs`: drag ghosts

  Build with these before writing a styled `div`. `tests/design_rules.rs`
  fails on off-grid values.
- A `RenderOnce` component (`#[derive(IntoElement)]` + `impl RenderOnce`) and
  a `Popover`: `SelectTrigger` and the menu in `select.rs`.
- Custom painting with `canvas(prepaint, paint)`: the terminal grid in
  `terminal_view/render/`. An empty `canvas` that only measures its bounds:
  `workspace/features/git/diff/render.rs`.
- Animation (`with_animation`, `Animation::new(…).repeat()`,
  `pulsating_between`): the status dot in `workspace/sidebar.rs`.

## Lists and scrolling

- Variable-height virtual list (`list` + `ListState`, splicing only the rows
  that changed, keeping a focused row alive): `workspace/panel_list.rs`. Its
  header comment explains why a scrolling `div` isn't enough. The diff view
  uses `ListState` too: `workspace/features/git/diff/`.
- Fixed-height virtual list (`uniform_list` + `UniformListScrollHandle`,
  `scroll_to_item`): `settings_window/font_picker.rs`, `select.rs`.

## Input, focus, actions

- `FocusHandle` and `impl Focusable`: `terminal_view/mod.rs`. `track_focus` and
  the key and mouse handlers it wires up: `terminal_view/render/mod.rs`.
  `window.focus(&h, cx)` and raw key and mouse input: `terminal_view/input.rs`.
- A key context plus `on_action(cx.listener(…))` for window commands:
  `workspace/layout.rs` (`"Workspace"`), `settings_window/mod.rs`
  (`"SettingsWindow"`), `studio/mod.rs` (`"StudioWindow"`).
- Shortcuts handled from `on_key_down` (a project can rebind them, so they
  aren't static `KeyBinding`s): `workspace/shortcuts.rs`.
- Drag and drop: `on_drag` / `drag_over` / `on_drop` in `style/reorder.rs`
  (list reordering), `workspace/layout.rs` (files dropped on the title bar)
  and `terminal_view/render/`.

## Components (gpui-component through `gpui_kit::component`)

- `InputState` + `Input`, with `cx.subscribe_in(&input, window, …)` on
  `InputEvent` (`Change`, `PressEnter`, `Focus`, `Blur`), and `Checkbox`:
  `workspace/features/notes/tasks.rs`. Multi-line `TextareaState`: `workspace/features/notes/`.
- `Switch` in a settings row: `style/setting.rs`; `ColorPicker`:
  `settings_window/sections/appearance.rs`, `studio/token_editor.rs`.
- `DropdownMenu` and `Tooltip`: `workspace/sidebar.rs`; `Tooltip` on icon
  buttons: `style/controls.rs`.
- Syntax highlighting (`SyntaxHighlighter`, `HighlightTheme`):
  `workspace/features/git/diff/highlight.rs`.
- Every component in one place, with each theme applied: the Studio gallery
  in `studio/gallery/` (one specimen per file). A new shared
  component goes there too; `tests/design_rules.rs` checks it.

## Async

- A debounced follow-up kept in an `Option<Task<()>>` field (replacing the
  field cancels the previous one): `cwd_check` in `terminal_view/mod.rs`.
- Work off the main thread (`cx.background_executor().spawn`), then
  `this.update(cx, …)` with a ticket so a result that no longer matches the
  request is dropped: `workspace/features/git/changes.rs`
  (`finish_git_refresh`), `workspace/features/git/diff/load.rs`,
  `workspace/features/git/history.rs`.
- A window-bound loop that stops when its view is gone (`spawn_in` +
  `update_in` returning `Err`): `settings_window/mod.rs`.
- A native file dialog (`cx.prompt_for_paths`) awaited in a task:
  `settings_window/sections/mod.rs`.

## Tests and checks

- Unit tests in a GPUI crate import items explicitly, never `use super::*`:
  `workspace/panel_list.rs`, `select.rs`.
- Headless GPUI tests (`#[gpui_kit::test]` with `TestAppContext`;
  `gpui_kit::init` and `SettingsStore::for_test()` first, then
  `gpui_kit::open_window`): `select/window_tests.rs`,
  `settings/app_tests.rs`, `studio/window_tests.rs`. The app
  crate's dev-dependency turns on `gpui-kit/test-support`, so plain
  `cargo test` builds them.
- A whole `Workspace` window in a test, isolated in a child process with a
  scratch data dir and `/bin/cat` as the shell (`isolate`), driven with
  `gpui_kit::test::TestWindowExt`: `workspace/view_tests.rs`.
- Logic that needs no UI goes into `wormhole-core` and is tested there.
- Scripted checks of the real app (screenshots, clicks, typing): the
  `devctl` skill and `devctl/`. The inspector (⇧⌥⌘I in debug builds):
  `inspector/`.
