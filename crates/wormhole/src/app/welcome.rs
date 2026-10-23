//! The app root's way into the first run and the terminal welcome: on a first launch, which of the two
//! opens over the window; Help → Welcome to Wormhole and the palette's "Play Welcome", which find the
//! front window; and what the welcome program's requests (answered in `onboarding::welcome`) ask of the
//! windows. They find the window, which the first run and the welcome, below it, do not.

use gpui_kit::*;
use wormhole_core::onboarding::intro::{Handover, Start};
use wormhole_core::onboarding::{ONBOARDED_KEY, should_show};

use crate::app_state::{AppState, app};
use crate::window::onboarding::welcome::{self, Plan};
use crate::window::window_lifecycle::with_front_workspace;

/// On a first launch (nothing saved, the intro never finished or skipped), open the intro window
/// on the window that was just opened.
pub(crate) fn start_if_first_launch(cx: &mut App) {
    let Some(store) = &app(cx).store else { return };
    let flag = store.meta(ONBOARDED_KEY).ok().flatten();
    if !should_show(flag.as_deref(), 0) {
        return;
    }
    let Some(window) = app(cx).windows.first().cloned() else { return };
    // Deferred: the window is still being set up.
    cx.defer(move |cx| {
        // The window was never shown: it arrives out of the animation.
        AppState::with_workspace(&window, cx, |ws, window, cx| ws.open_first_intro(window, cx));
    });
}

/// The plan for the first launch, when the terminal welcome is on and this is one (nothing saved, the
/// first run never finished or skipped): the window's first terminal is then the welcome.
pub(crate) fn first_launch_plan(cx: &App) -> Option<Plan> {
    let store = app(cx).store.as_ref()?;
    let flag = store.meta(ONBOARDED_KEY).ok().flatten();
    (should_show(flag.as_deref(), 0) && welcome::enabled(cx))
        .then_some(Plan { handover: Handover::Cursor, first_run: true })
}

/// The first launch's welcome has its terminal; play the animation around the window that was just
/// opened (never shown, so it arrives out of the run). Test instances stay behind other windows and
/// skip the show, as the intro window does.
pub(crate) fn play_first(plan: Plan, cx: &mut App) {
    let Some(window) = app(cx).windows.first().cloned() else { return };
    // Deferred: the window is still being set up.
    cx.defer(move |cx| {
        AppState::with_workspace(&window, cx, |ws, window, cx| {
            let play = !crate::background::on();
            ws.start_welcome_animation(plan, play.then_some(Start::Hidden), window, cx);
        });
    });
}

/// Help → Welcome to Wormhole and the palette's "Play Welcome" entries: the front window plays the
/// run and gets a new terminal that runs the welcome with `handover`.
pub(crate) fn replay_in_front(handover: Handover, cx: &mut App) {
    cx.defer(move |cx| {
        with_front_workspace(cx, |ws, window, cx| {
            crate::background::activate_window(window, cx);
            ws.replay_welcome(handover, window, cx);
        });
    });
}

/// The window the welcome's terminal `tab` is in.
fn in_window(tab: i64, cx: &App) -> Option<crate::app_state::OpenWindow> {
    AppState::find_window(cx, |ws, _| ws.find_session(tab).is_some())
}

/// The welcome program's `Demo`: the example band in the sidebar of the window its terminal is in
/// (`Workspace::set_welcome_demo`); whether there is such a terminal.
pub(crate) fn demo(tab: i64, show: bool, cx: &mut App) -> bool {
    let Some(window) = in_window(tab, cx) else { return false };
    AppState::with_workspace(&window, cx, |ws, _, cx| ws.set_welcome_demo(tab, show, cx));
    true
}

/// The welcome program's `Finish`, for the window of its terminal: the example band goes and, with
/// `note`, the sidebar's note for a run that ended on the primer comes up.
pub(crate) fn ended(tab: i64, note: bool, cx: &mut App) {
    if let Some(window) = in_window(tab, cx) {
        AppState::with_workspace(&window, cx, |ws, _, cx| ws.welcome_ended(tab, note, cx));
    }
}
