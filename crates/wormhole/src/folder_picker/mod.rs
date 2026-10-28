//! The folder picker at the prompt: ⌘J opens the generic picker (`crate::picker`) over the terminal's cursor
//! with the folder providers plugged in (core's `folder_picker`: the subfolders, forgivingly through several
//! levels, the folders lately worked in, and the badges). Type a path or part of a name to narrow it, ↩ puts the
//! picked folder's path at the cursor as a paste and closes, ⇥ or → goes into the selected folder, ← goes up.
//! It never changes directory. Behind the Folder picker feature flag.
//!
//! This is the one module that knows the terminal: where the cursor is (the anchor), where the path goes, and
//! when the picker is no longer for it (another terminal comes to the front). The panel, the keys and the
//! queries are the generic picker's; the overlay it holds in the window (`window::overlay`) draws nothing of its
//! own, only keeps the window's rule that one thing is over it at a time and gives the keyboard back where
//! it was.

#[cfg(feature = "devctl")]
mod devctl;
#[cfg(all(test, feature = "devctl"))]
mod tests;

use std::path::PathBuf;
use std::rc::Rc;

use gpui_kit::*;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::folder_picker::parts;
use wormhole_core::platform::paths;
use wormhole_core::storage::files::{FOLDERS_CAP, SCAN_CAP};
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::view::limits::count_grouped;
use wormhole_core::view::panes::Rect;
use wormhole_core::view::picker::{Context as Where, Note};
use wormhole_text::{keys, text};

use crate::app_state::recent_folders;
use crate::picker::{Picker, PickerEvent, Sources, Words};
use crate::settings::settings_in;
use crate::style;
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
    picker: Entity<Picker>,
    profile: ProfileId,
    _subscriptions: Vec<Subscription>,
}

impl OverlayContent for FolderPicker {
    const KIND: OverlayKind = OverlayKind::FolderPicker;
}

/// The overlay has nothing to draw: the panel is the system's. (The window keeps it as the open overlay.)
impl Render for FolderPicker {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
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
/// Runs outside any window's update (the action handler defers it). Nothing happens with the flag off. While the
/// picker is up the panel has the keyboard and the window is not the active one, so only the closing half of
/// this can run from the key.
pub(crate) fn toggle(cx: &mut App) {
    let Some(front) = crate::app_state::AppState::front_window(cx) else { return };
    crate::app_state::AppState::with_workspace(&front, cx, |ws, window, cx| {
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
    let profile = ws.profile().clone();
    // The folder the picker opened in counts as a visit: a terminal that was started there never reported it.
    recent_folders::record(cx, &profile, &cwd);
    let return_focus = ws.end_overlay(window, cx);
    let workspace = cx.entity();
    let host =
        cx.new(|cx| FolderPicker::new(workspace, terminal.downgrade(), cwd, home, profile, window, cx));
    ws.show_overlay(OverlayView::new(host), return_focus, cx);
}

/// What a note under the rows says.
fn note(note: &Note) -> String {
    match note {
        Note::Empty => text(keys::FOLDER_PICKER_EMPTY).to_string(),
        Note::Failed => text(keys::FOLDER_PICKER_UNREADABLE).to_string(),
        Note::Limited(count) => keys::folder_picker_cut_folders(&count_grouped((*count).min(FOLDERS_CAP))),
        Note::Searched(count) => keys::folder_picker_cut_scanned(&count_grouped((*count).min(SCAN_CAP))),
        Note::Reading => text(keys::FOLDER_PICKER_READING).to_string(),
        // The folder providers have nothing of their own to say; a provider that does says it in its words.
        Note::Custom(words) => words.clone(),
    }
}

/// What the cursor is taken to be where the terminal has none to show (a full-screen program hid it): a line
/// at the pane's bottom left.
const FALLBACK_CELL: (f32, f32) = (8.0, 18.0);

/// The pane and the cursor's cell, in the window's space: the terminal's own last frame, else a guess.
fn anchor_of(terminal: &WeakEntity<TerminalView>, window: &Window, cx: &App) -> (Rect, Rect) {
    let viewport = window.viewport_size();
    let window_rect = Rect::new(0.0, 0.0, f32::from(viewport.width), f32::from(viewport.height));
    let Some(terminal) = terminal.upgrade() else { return (window_rect, Rect::default()) };
    let terminal = terminal.read(cx);
    let rect = |b: Bounds<Pixels>| {
        Rect::new(
            f32::from(b.origin.x),
            f32::from(b.origin.y),
            f32::from(b.size.width),
            f32::from(b.size.height),
        )
    };
    // A pane not drawn yet is zero wide: the window stands in.
    let pane = Some(rect(terminal.pane_bounds())).filter(|r| r.w > 0.0 && r.h > 0.0).unwrap_or(window_rect);
    let cursor = terminal.cursor_cell().map(rect).unwrap_or_else(|| {
        Rect::new(
            pane.x + style::EDGE_INSET,
            pane.bottom() - 2.0 * FALLBACK_CELL.1,
            FALLBACK_CELL.0,
            FALLBACK_CELL.1,
        )
    });
    (pane, cursor)
}

impl FolderPicker {
    fn new(
        workspace: Entity<Workspace>,
        terminal: WeakEntity<TerminalView>,
        cwd: PathBuf,
        home: PathBuf,
        profile: ProfileId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let folders = parts(recent_folders::shared(cx, &profile), text(keys::FOLDER_PICKER_RECENT));
        let sources = Sources { providers: folders.providers, decorators: folders.decorators };
        let words = Words {
            placeholder: text(keys::FOLDER_PICKER_SEARCH).to_string(),
            name: text(keys::FOLDER_PICKER_NAME).to_string(),
            hint: text(keys::FOLDER_PICKER_HINT).to_string(),
            note,
        };
        let context = Where { cwd, home, profile: profile.as_str().to_string() };
        let anchor_terminal = terminal.clone();
        let anchor: crate::picker::Anchor =
            Rc::new(move |window, cx| anchor_of(&anchor_terminal, window, cx));
        let picker_profile = profile.clone();
        let picker = cx.new(|cx| Picker::new(context, sources, words, picker_profile, anchor, window, cx));
        let heard = cx.subscribe_in(&picker, window, |this, _, event: &PickerEvent, window, cx| {
            this.on_picker(event, window, cx);
        });
        // The picker is for one terminal: when another is in front, or this one is gone, there is nothing to
        // put a path into, and the panel's place (the old terminal's cursor) is wrong.
        let moved = cx.observe_in(&workspace, window, |this, workspace, window, cx| {
            let in_front = workspace.read(cx).active_terminal();
            if this.terminal.upgrade().is_none_or(|terminal| terminal != in_front) {
                this.close(window, cx);
            } else {
                this.picker.update(cx, |picker, cx| picker.reanchor(window, cx));
            }
        });
        Self {
            workspace: workspace.downgrade(),
            terminal,
            picker,
            profile,
            _subscriptions: vec![heard, moved],
        }
    }

    fn on_picker(&mut self, event: &PickerEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            PickerEvent::Insert(path) => {
                self.pick(path, window, cx);
            }
            PickerEvent::Forget(key) => self.forget(key, cx),
            PickerEvent::Close => self.close(window, cx),
        }
    }

    /// A pick: put `path` at the terminal's cursor and close. The picker closes first, so the keyboard is back in
    /// the terminal when the text arrives. It goes in as a paste, which a program that asked for bracketed paste
    /// (a coding agent) receives as one.
    fn pick(&mut self, path: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        // Only into the terminal it opened over, and only while that is the one in front: the picker closes
        // when it isn't, but the key that picks may come first.
        let there = self.in_front(cx);
        self.close(window, cx);
        if !there {
            return false;
        }
        let _ = self.terminal.update(cx, |terminal, cx| {
            terminal.paste(path);
            cx.notify();
        });
        true
    }

    /// Forget a recent folder: gone from the list the picker and the store share, and from this picker's rows.
    fn forget(&mut self, key: &str, cx: &mut Context<Self>) {
        recent_folders::forget(cx, &self.profile, key);
        self.picker.update(cx, |picker, cx| picker.refresh(cx));
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
        // The panel goes first, from a task of its own: the overlay's release would take it down inside GPUI's.
        self.picker.update(cx, |picker, cx| picker.close(cx));
        let _ = self.workspace.update(cx, |ws, cx| {
            if ws.folder_picker_open() {
                ws.close_overlay(window, cx);
            }
        });
    }
}
