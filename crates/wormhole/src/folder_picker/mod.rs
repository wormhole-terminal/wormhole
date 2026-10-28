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
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::*;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::platform::paths;
use wormhole_core::storage::files::{FOLDERS_CAP, list_folders_where};
use wormhole_core::view::folder_picker::{Picker, Placed, keeps};
use wormhole_text::{keys, text};

use crate::app_state::AppState;
use crate::settings::settings_in;
use crate::terminal_view::TerminalView;
use crate::window::Workspace;
use crate::window::overlay::{OverlayContent, OverlayKind, OverlayView};

#[cfg(feature = "devctl")]
pub(crate) use devctl::devctl_folder_picker;

pub(crate) struct FolderPicker {
    workspace: WeakEntity<Workspace>,
    /// The terminal the path goes to: the one in front when the picker opened. The picker closes when another
    /// becomes the one in front, so it is also the one in front when the path arrives.
    terminal: WeakEntity<TerminalView>,
    state: Picker,
    field: Entity<InputState>,
    scroll: UniformListScrollHandle,
    /// The read of the folder being listed, from the pause before it to the answer. One at a time: the text may
    /// change many times while it runs, and when it is done the picker asks for what it wants by then.
    reading: Option<Task<()>>,
    /// Where the card was last drawn, for devctl and the tests.
    placed: Cell<Option<Placed>>,
    _subscriptions: Vec<Subscription>,
}

impl OverlayContent for FolderPicker {
    const KIND: OverlayKind = OverlayKind::FolderPicker;
}

/// Whether the Folder picker's feature flag is on for `ws`'s profile.
pub(crate) fn flag_on(ws: &Workspace, cx: &App) -> bool {
    settings_in(ws.profile(), cx).feature_enabled(FeatureFlag::FolderPicker)
}

/// Whether the picker may open over `ws` now: its flag is on, nothing covers the terminal or holds the keyboard
/// (the first-run sheet, a diff), the window is the one in use and its terminal has the keyboard. The key only
/// works there (it is a terminal-scope key); the menu and the palette run it from wherever, which must not
/// paste into a terminal the person isn't in.
fn can_open(ws: &Workspace, window: &Window, cx: &App) -> bool {
    flag_on(ws, cx)
        && ws.terminal_overlay_allowed(cx)
        && window.is_window_active()
        && ws.active_terminal().focus_handle(cx).is_focused(window)
}

/// ⌘J (and Terminal → Insert Folder Path…): open the picker over the window in front, or close it there.
/// Runs outside any window's update (the action handler defers it). Nothing happens with the flag off.
pub(crate) fn toggle(cx: &mut App) {
    let Some(front) = AppState::front_window(cx) else { return };
    AppState::with_workspace(&front, cx, |ws, window, cx| {
        if ws.folder_picker_open() {
            ws.close_overlay(window, cx);
        } else if can_open(ws, window, cx) {
            open(ws, window, cx);
        }
    });
}

/// The picker over `ws`, for its terminal in front.
fn open(ws: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
    crate::background::activate_window(window, cx);
    let terminal = ws.active_terminal();
    let home = paths::home();
    // Where the terminal is; a terminal that hasn't said (no OSC 7 yet, no process to ask) is at home.
    let cwd = terminal.read(cx).cwd.clone().or_else(|| home.clone()).unwrap_or_else(|| PathBuf::from("/"));
    let home = home.unwrap_or_else(|| cwd.clone());
    let return_focus = ws.end_overlay(window, cx);
    let workspace = cx.entity();
    let picker = cx.new(|cx| FolderPicker::new(workspace, terminal.downgrade(), cwd, home, window, cx));
    ws.show_overlay(OverlayView::new(picker), return_focus, cx);
}

/// A read of a folder is a blocking call on a pool thread that nothing can cancel, and on a share that has
/// stopped answering it never comes back. So there is one at a time for the whole app: while one runs, the
/// next waits for it, and a picker opened again doesn't add another thread to the stuck one.
static READING: AtomicBool = AtomicBool::new(false);

/// The right to read: held while a read runs, wherever it runs.
struct Reading;

impl Reading {
    fn take() -> Option<Self> {
        (!READING.swap(true, Ordering::AcqRel)).then_some(Reading)
    }
}

impl Drop for Reading {
    fn drop(&mut self) {
        READING.store(false, Ordering::Release);
    }
}

/// How long the text must rest before a folder is read again for it (a read for a filter, which each key changes).
const PAUSE: Duration = Duration::from_millis(120);
/// How often a read that waits for another to end looks again.
const WAIT: Duration = Duration::from_millis(50);

impl FolderPicker {
    fn new(
        workspace: Entity<Workspace>,
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
        // The picker is for one terminal: when another is in front, or this one is gone, there is nothing to
        // put a path into, and the card's place (the old terminal's cursor) is wrong.
        let moved = cx.observe_in(&workspace, window, |this, workspace, window, cx| {
            let in_front = workspace.read(cx).active_terminal();
            if this.terminal.upgrade().is_none_or(|terminal| terminal != in_front) {
                this.close(window, cx);
            }
        });
        let mut picker = Self {
            workspace: workspace.downgrade(),
            terminal,
            state: Picker::new(cwd, home),
            field,
            scroll: UniformListScrollHandle::new(),
            reading: None,
            placed: Cell::new(None),
            _subscriptions: vec![key_events, changed, moved],
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

    /// Read what the picker is waiting for, if anything and if no read is running: off the UI thread, since the
    /// folder may be on a slow share or hold thousands of entries. The first read of a folder starts at once; one
    /// for a new filter (a folder cut by the caps is read again for each) waits for the text to rest, then reads
    /// for what is typed by then. When a read ends the picker asks again, so the text that moved on meanwhile
    /// is answered by one more read and never by a crowd of them.
    fn read_wanted(&mut self, cx: &mut Context<Self>) {
        if self.reading.is_some() || self.state.wants().is_none() {
            return;
        }
        let pause = if self.state.loaded() { PAUSE } else { Duration::ZERO };
        self.reading = Some(cx.spawn(async move |this, cx| {
            if !pause.is_zero() {
                cx.background_executor().timer(pause).await;
            }
            let right = loop {
                if let Some(right) = Reading::take() {
                    break right;
                }
                cx.background_executor().timer(WAIT).await;
            };
            let want = this
                .update(cx, |this, _| {
                    this.state.wants().map(|(folder, filter)| (folder.to_path_buf(), filter.to_string()))
                })
                .ok()
                .flatten();
            let Some((folder, filter)) = want else {
                drop(right);
                this.update(cx, |this, _| this.reading = None).ok();
                return;
            };
            let (asked, matching) = (folder.clone(), filter.clone());
            let read = cx
                .background_spawn(async move {
                    let _right = right;
                    list_folders_where(&asked, FOLDERS_CAP, |name| keeps(name, &matching))
                })
                .await;
            this.update(cx, |this, cx| {
                this.reading = None;
                this.state.set_listing(&folder, &filter, read);
                this.scroll_to_selected();
                cx.notify();
                this.read_wanted(cx);
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
        // Only into the terminal it opened over, and only while that is the one in front: the picker closes
        // when it isn't, but the key that picks may come first.
        let there = self.in_front(cx);
        self.close(window, cx);
        if !there {
            return None;
        }
        let _ = self.terminal.update(cx, |terminal, cx| {
            terminal.paste(&inserted);
            cx.notify();
        });
        Some(inserted)
    }

    /// Whether the terminal the picker opened over is still the window's front terminal.
    fn in_front(&self, cx: &App) -> bool {
        let (Some(terminal), Ok(front)) =
            (self.terminal.upgrade(), self.workspace.read_with(cx, |ws, _| ws.active_terminal()))
        else {
            return false;
        };
        terminal == front
    }

    /// Take the picker down, if it is what is over the window.
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.workspace.update(cx, |ws, cx| {
            if ws.folder_picker_open() {
                ws.close_overlay(window, cx);
            }
        });
    }

    /// An input method is composing text in the field (a marked range): its keys, ↩ among them, are its own.
    fn composing(&self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self.field.update(cx, |field, cx| field.marked_text_range(window, cx).is_some())
    }

    fn on_key(&mut self, keystroke: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let m = &keystroke.modifiers;
        let plain = !m.modified();
        // While an input method composes, ↩ settles the composition and Esc cancels it: not the picker's.
        if self.composing(window, cx) {
            return false;
        }
        match keystroke.key.as_str() {
            "escape" if plain => self.close(window, cx),
            "up" if plain => self.step(-1, cx),
            "down" if plain => self.step(1, cx),
            "tab" if plain => self.complete(window, cx),
            // The key that opened it closes it: it is bound to the terminal only, which the field isn't.
            "j" if m.platform && !m.shift && !m.alt && !m.control => self.close(window, cx),
            "enter" if plain => {
                self.pick(window, cx);
            }
            _ => return false,
        }
        true
    }
}
