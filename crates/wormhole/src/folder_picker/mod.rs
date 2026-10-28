//! The folder picker at the prompt: ⌘J opens a card at the terminal's cursor that lists the folders below
//! the terminal's own; typing a path walks (`src/`, `../`, `~/`, `/usr/`), ↩ puts the picked folder's path at
//! the cursor as a paste and closes. It never changes directory. Behind the Folder picker feature flag.
//!
//! It is an overlay of the window (`window::overlay`), not a popover of the platform, for what it needs: a
//! search field that keeps the keyboard, a list that scrolls and follows the selection, and a card placed
//! against the cursor, which is GPUI's to draw. An overlay brings the window's rules with it (one at a time,
//! the keyboard returns where it was) and, unlike the palette's, has no scrim: a click outside closes it and
//! lands on what is under it. It is not a feature in the `FeatureId` table sense: no tab, no setting, no state
//! per terminal beyond the card while it is open, so a flag and this module are all there is.
//!
//! What it means and shows is core's `view::folder_picker` (the typed text, the rows, the shell-ready path, the
//! card's place); a folder is read off the UI thread with `storage::files::list_folders`.

#[cfg(feature = "devctl")]
mod devctl;
#[cfg(all(test, feature = "devctl"))]
mod tests;
mod view;

use std::cell::Cell;
use std::path::{Path, PathBuf};

use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::*;
use wormhole_core::platform::paths;
use wormhole_core::storage::files::{FOLDERS_CAP, list_folders};
use wormhole_core::view::folder_picker::{Picker, Placed};
use wormhole_text::{keys, text};

use crate::app_state::AppState;
use crate::terminal_view::TerminalView;
use crate::window::Workspace;
use crate::window::overlay::{OverlayContent, OverlayKind, OverlayView};

#[cfg(feature = "devctl")]
pub(crate) use devctl::devctl_folder_picker;

pub(crate) struct FolderPicker {
    workspace: WeakEntity<Workspace>,
    /// The terminal the path goes to: the one in front when the picker opened.
    terminal: WeakEntity<TerminalView>,
    state: Picker,
    field: Entity<InputState>,
    scroll: UniformListScrollHandle,
    /// The read of the folder being listed; replacing it drops the one before.
    reading: Option<Task<()>>,
    /// Where the card was last drawn, for devctl and the tests.
    placed: Cell<Option<Placed>>,
    _subscriptions: Vec<Subscription>,
}

impl OverlayContent for FolderPicker {
    const KIND: OverlayKind = OverlayKind::FolderPicker;
}

/// ⌘J (and Terminal → Insert Folder Path…): open the picker over the window in front, or close it there.
/// Runs outside any window's update (the action handler defers it). Nothing happens with the flag off.
pub(crate) fn toggle(cx: &mut App) {
    let Some(front) = AppState::front_window(cx) else { return };
    AppState::with_workspace(&front, cx, |ws, window, cx| {
        if ws.folder_picker_open() {
            ws.close_overlay(window, cx);
        } else if ws.can_pick_folder(cx) {
            open(ws, window, cx);
        }
    });
}

/// The picker over `ws`, for its terminal in front.
fn open(ws: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
    crate::background::activate_window(window, cx);
    let terminal = ws.active_terminal().clone();
    let home = paths::home();
    // Where the terminal is; a terminal that hasn't said (no OSC 7 yet, no process to ask) is at home.
    let cwd = terminal.read(cx).cwd.clone().or_else(|| home.clone()).unwrap_or_else(|| PathBuf::from("/"));
    let home = home.unwrap_or_else(|| cwd.clone());
    let return_focus = ws.end_overlay(window, cx);
    let workspace = cx.weak_entity();
    let picker = cx.new(|cx| FolderPicker::new(workspace, terminal.downgrade(), cwd, home, window, cx));
    ws.show_overlay(OverlayView::new(picker), return_focus, cx);
}

impl FolderPicker {
    fn new(
        workspace: WeakEntity<Workspace>,
        terminal: WeakEntity<TerminalView>,
        cwd: PathBuf,
        home: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let field = cx.new(|cx| InputState::new(window, cx).placeholder(text(keys::FOLDER_PICKER_SEARCH)));
        window.focus(&field.focus_handle(cx), cx);
        let handle = window.window_handle().window_id();
        let this = cx.weak_entity();
        // Ahead of the field, which would take ↑ ↓ ⏎ Tab and Esc for itself.
        let key_events = cx.intercept_keystrokes(move |event, window, cx| {
            if window.window_handle().window_id() != handle {
                return;
            }
            let handled =
                this.update(cx, |this, cx| this.on_key(&event.keystroke, window, cx)).unwrap_or(false);
            if handled {
                cx.stop_propagation();
            }
        });
        let changed = cx.subscribe_in(&field, window, |this, field, event: &InputEvent, _, cx| {
            if let InputEvent::Change = event {
                let typed = field.read(cx).value().to_string();
                this.typed(&typed, cx);
            }
        });
        let mut picker = Self {
            workspace,
            terminal,
            state: Picker::new(cwd, home),
            field,
            scroll: UniformListScrollHandle::new(),
            reading: None,
            placed: Cell::new(None),
            _subscriptions: vec![key_events, changed],
        };
        picker.read_wanted(cx);
        picker
    }

    /// The text in the field is now `typed`.
    fn typed(&mut self, typed: &str, cx: &mut Context<Self>) {
        self.state.set_typed(typed);
        self.read_wanted(cx);
        self.scroll_to_selected();
        cx.notify();
    }

    /// Read the folder the text points at, if its listing isn't in: off the UI thread, since the folder may be
    /// on a slow share or hold thousands of entries.
    fn read_wanted(&mut self, cx: &mut Context<Self>) {
        let Some(folder) = self.state.wants().map(Path::to_path_buf) else { return };
        let reading = folder.clone();
        let read = cx.background_spawn(async move { list_folders(&reading, FOLDERS_CAP) });
        self.reading = Some(cx.spawn(async move |this, cx| {
            let read = read.await;
            this.update(cx, |this, cx| {
                this.state.set_listing(&folder, read);
                this.scroll_to_selected();
                cx.notify();
            })
            .ok();
        }));
    }

    fn scroll_to_selected(&self) {
        let at = self.state.selected().unwrap_or(0);
        self.scroll.scroll_to_item(at, ScrollStrategy::Nearest);
    }

    fn step(&mut self, by: isize, cx: &mut Context<Self>) {
        self.state.step(by);
        self.scroll_to_selected();
        cx.notify();
    }

    /// ⇥: the selected folder written into the field, so its folders are the list.
    fn complete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(completed) = self.state.completion() else { return };
        self.field.update(cx, |field, cx| field.set_value(completed.clone(), window, cx));
        self.typed(&completed, cx);
    }

    /// ↩ or a click: put the selected folder's path at the terminal's cursor and close. The picker closes
    /// first, so the keyboard is back in the terminal when the text arrives. It goes in as a paste, which a
    /// program that asked for bracketed paste (a coding agent) receives as one.
    fn pick(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<String> {
        let inserted = self.state.choice()?;
        self.close(window, cx);
        let _ = self.terminal.update(cx, |terminal, cx| {
            terminal.paste(&inserted);
            cx.notify();
        });
        Some(inserted)
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.workspace.update(cx, |ws, cx| ws.close_overlay(window, cx));
    }

    fn on_key(&mut self, keystroke: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let m = &keystroke.modifiers;
        let plain = !m.modified();
        match keystroke.key.as_str() {
            "escape" if plain => self.close(window, cx),
            "up" if plain => self.step(-1, cx),
            "down" if plain => self.step(1, cx),
            "tab" if plain => self.complete(window, cx),
            "enter" if plain => {
                self.pick(window, cx);
            }
            _ => return false,
        }
        true
    }
}
