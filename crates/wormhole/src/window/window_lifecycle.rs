//! Opening and closing the windows that show a [`Workspace`]: where a new one is placed, running
//! something on the front window or opening one when there is none, saving every window on the way
//! out, bringing a terminal's window forward, and a window moving to another profile.
//!
//! Part of the window: it reads and updates the shared state (`app`, `app_state::look`) and must not use
//! the surfaces above it (the command palette, the shortcut sheet, the Settings and Studio windows,
//! the status item, the app root). The shared state never calls it back: whoever needs a window
//! opened comes here.

use gpui_kit::*;
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::storage::store::WindowRecord;
use wormhole_core::view::layout::{WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH};

use crate::app_state::look::sync_system_look;
use crate::app_state::{AppState, OpenWindow, app, app_mut};
use crate::settings::settings_in;
use crate::style;
use crate::window::{Layout, Workspace, window_bounds};

const DEFAULT_SIZE: (f32, f32) = (1320.0, 820.0);

/// Open a window showing `record` (a new, empty one without it), placed where
/// it was saved, or stepped away from `source`, or centered.
pub(crate) fn open_workspace_window(
    record: Option<WindowRecord>,
    source: Option<Bounds<Pixels>>,
    cx: &mut App,
) {
    let profile = app(cx).active_profile().clone();
    open_workspace_window_in(&profile, record, source, cx);
}

/// [`open_workspace_window`] for a window of `profile`.
pub(crate) fn open_workspace_window_in(
    profile: &ProfileId,
    record: Option<WindowRecord>,
    source: Option<Bounds<Pixels>>,
    cx: &mut App,
) {
    let id = match &record {
        Some(record) => record.id,
        None => app_mut(cx).alloc_window_id(),
    };
    // No record: a window the person asked for with none open, so its columns are as they last
    // left them, not `Panes::default()` (which the saved-layout path and the tests start from).
    let record = record.unwrap_or_else(|| WindowRecord {
        id,
        layout: Layout::remembered(&app(cx).last_panes),
        ..Default::default()
    });
    let displays: Vec<Bounds<Pixels>> = cx.displays().iter().map(|d| d.bounds()).collect();
    let saved = record.bounds.as_deref().and_then(window_bounds::decode).filter(|saved| {
        // A monitor may be gone since: never restore a window nobody can see.
        displays.iter().any(|d| d.intersects(&saved.get_bounds()))
    });
    let bounds = match (saved, source) {
        (Some(saved), _) => saved,
        (None, Some(source)) => WindowBounds::Windowed(window_bounds::cascade(source, &displays)),
        (None, None) => {
            WindowBounds::Windowed(Bounds::centered(None, size(px(DEFAULT_SIZE.0), px(DEFAULT_SIZE.1)), cx))
        }
    };
    let options = WindowOptions {
        window_bounds: Some(bounds),
        titlebar: Some(TitlebarOptions {
            title: Some("Wormhole".into()),
            appears_transparent: true,
            traffic_light_position: Some(style::traffic_light_position(style::TITLE_ROW_H)),
        }),
        window_background: WindowBackgroundAppearance::Transparent,
        window_min_size: Some(size(px(WINDOW_MIN_WIDTH), px(WINDOW_MIN_HEIGHT))),
        // The title rows drag the window themselves (`style::TitleDrag`).
        app_owns_titlebar_drag: true,
        ..Default::default()
    };
    let owner = profile.clone();
    match crate::window_open::open(options, cx, move |window, cx| {
        cx.new(|cx| Workspace::new(record, owner, window, cx))
    }) {
        Ok((handle, workspace)) => {
            // The system's appearance is every window's: any of them hears it change. GPUI also
            // reports it once as the window opens. Deferred, as it arrives inside this window's
            // update and may redraw the others.
            let _ = handle.update(cx, |_, window, _| {
                window.observe_window_appearance(|_, cx| cx.defer(sync_system_look)).detach();
            });
            let state = app_mut(cx);
            state.windows.push(OpenWindow {
                id,
                handle,
                workspace: workspace.downgrade(),
                profile: profile.clone(),
                hidden: false,
            });
            state.front = id;
            // Its own observer starts after this update, so it would not hear that it was listed.
            workspace.update(cx, |workspace, cx| workspace.app_state_changed(cx));
            crate::background::activate(cx);
            let _ = handle.update(cx, |_, window, cx| crate::background::activate_window(window, cx));
        }
        Err(err) => tracing::error!("opening a window failed: {err:#}"),
    }
}

/// Run `f` on the front window's workspace, opening a window if there is none.
pub(crate) fn with_front_workspace<R>(
    cx: &mut App,
    f: impl FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>) -> R,
) -> Option<R> {
    match AppState::front_window(cx) {
        Some(window) => AppState::with_workspace(&window, cx, f),
        None => {
            open_workspace_window(None, None, cx);
            None
        }
    }
}

/// ⌘N (or the menu): a window like the one in front.
pub(crate) fn new_window_from_front(cx: &mut App) {
    if with_front_workspace(cx, |ws, window, cx| ws.new_window(window, cx)).is_none()
        && app(cx).windows.is_empty()
    {
        open_workspace_window(None, None, cx);
    }
}

pub(crate) fn window_closed(cx: &mut App, id: WindowId) {
    let state = app_mut(cx);
    state.windows.retain(|w| w.handle.window_id() != id);
    // Sessions end with their window, so with none left there is nothing to show. (The close
    // button asked before the last visible window went, while another profile still had windows.)
    if state.windows.is_empty() && !state.quitting {
        cx.quit();
    }
}

/// Everything that is worth keeping, for every window (scrollback included).
pub(crate) fn save_all(cx: &mut App) {
    app_mut(cx).quitting = true;
    let windows = app(cx).windows.clone();
    for window in &windows {
        AppState::with_workspace(window, cx, |ws, win, cx| ws.persist(win, cx));
    }
    app(cx).checkpoint_stores();
}

/// A notification about the terminal `tab` (its id) was clicked (this runs from the platform's
/// callback, outside any window's update): bring its window and the terminal forward.
pub(crate) fn open_tab(tab: i64, cx: &mut App) {
    let Some(owner) = AppState::find_window(cx, |ws, _| ws.find_session(tab).is_some()) else { return };
    AppState::with_workspace(&owner, cx, |ws, window, cx| {
        crate::background::activate_window(window, cx);
        ws.reveal_terminal(tab, window, cx);
    });
}

impl Workspace {
    /// This window and its terminals go to profile `target`: its record and its terminals'
    /// scrollback are saved into that profile's database and removed from the old one. The
    /// terminals themselves are untouched, so they keep the environment they started with.
    pub(crate) fn rehome(&mut self, target: &ProfileId, cx: &mut Context<Self>) -> Result<(), String> {
        let from = self.profile.clone();
        if &from == target {
            return Ok(());
        }
        let record = self.window_record(cx);
        // Only what the target profile keeps: nothing when its Restore is off, never a terminal whose
        // text the person asked not to keep.
        let keeps_text = settings_in(target, cx).restore_terminal_text;
        let scrollback: Vec<(i64, String)> = self
            .sessions
            .iter()
            .filter(|session| keeps_text && !session.forget_text)
            .map(|session| &session.pane)
            .map(|pane| {
                (pane.id, pane.terminal.read(cx).scrollback_text(crate::window::save::SCROLLBACK_LINES))
            })
            .collect();
        let state = app_mut(cx);
        let Some(store) = state.store_of_mut(target) else {
            return Err(format!("the database of profile {target} is not open"));
        };
        store.save_window(&record).map_err(|err| format!("saving the window in {target}: {err:#}"))?;
        for (id, text) in &scrollback {
            if let Err(err) = store.save_scrollback(*id, text) {
                tracing::warn!("scrollback of {id} did not move to {target}: {err:#}");
            }
        }
        state.forget_window(&from, self.window_id);
        self.profile = target.clone();
        // The features that keep something for the profile (an open worktree dialog) follow.
        self.rehomed_features(cx);
        // What it draws with follows: its terminals read their window's profile (an open diff reads it
        // from the window's facts at the next frame).
        for session in &self.sessions {
            session.pane.terminal.update(cx, |terminal, cx| terminal.set_profile(target.clone(), cx));
        }
        self.dirty = false;
        cx.notify();
        Ok(())
    }
}
