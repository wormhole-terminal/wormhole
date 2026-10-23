//! The terminal welcome: the first run as a program in the first terminal (`Wormhole welcome`, core's
//! `welcome`), with the window's sidebar and chrome showing, instead of the intro window that stands in
//! for the terminal. Behind the `terminal_welcome` feature flag (on in a dev build, off in a release):
//! with it off, the first launch and Help → Welcome to Wormhole open the intro window as before.
//!
//! This module starts it and answers it. Starting: a terminal whose shell is `Wormhole welcome --
//! <the shell it would have run>`, with a random token in its environment alone; the wormhole run
//! plays around the window with the hand-over on trial (`Handover`: the ship docks on the mark's cell,
//! no ship, or the ship docks on a block-art logo), the terminal reporting where its cells are so the
//! ship lands on the real one. Answering (`answers`): the program's requests come over the control
//! socket, are let through only with that token from that terminal while its welcome runs, and are the
//! app's to do: the look, what reaches the person (and macOS' permission for banners, asked only from a
//! press that turns one on), an agent's hooks, the example band in the sidebar, and remembering the
//! first run.
//!
//! Nothing is installed or switched on behind the person's back: only the program's explicit Install or
//! notification requests do it, and the program sends those only from the press on that row. The
//! example band (`sidebar::demo`) is the one thing the window shows for the program without a press:
//! it lives in the window's sidebar state, not in any list of terminals, and goes with the step.

mod answers;

use std::time::Instant;

use gpui_kit::*;
use wormhole_core::ipc::control::AgentsFacts;
use wormhole_core::onboarding::intro::{Handover, Start};
use wormhole_core::onboarding::welcome::Registry;
use wormhole_core::onboarding::{ONBOARDED_KEY, should_show};

use crate::app_state::{AppState, app};
use crate::settings::settings;
use crate::window::window_lifecycle::with_front_workspace;

pub(crate) use answers::{Services, handle};

/// What a welcome is to be.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Plan {
    pub handover: Handover,
    /// The first run, which finishing is remembered of, or a replay.
    pub first_run: bool,
}

/// Whether the start animation plays around this welcome, and since when.
#[derive(Clone, Copy, Debug)]
pub(in crate::window) enum Play {
    /// None plays (a test instance's first launch, no Metal): the program has nothing to wait for.
    Never,
    /// About to: the first launch opens its window and then plays; a request that comes in between
    /// waits for the whole run.
    Pending,
    Since(Instant),
}

/// One welcome terminal's side in the app.
pub(in crate::window) struct Run {
    pub(in crate::window) tab: i64,
    pub(in crate::window) handover: Handover,
    pub(in crate::window) first_run: bool,
    pub(in crate::window) play: Play,
    /// The agents found; `None` until the search (off the UI thread) is done.
    pub(in crate::window) agents: Option<AgentsFacts>,
    /// What went wrong with the last Install, until the next one that works.
    pub(in crate::window) problem: Option<String>,
    /// devctl's `state` names it, so `welcome-probe` can send a request as the program would (and the
    /// tests do).
    #[cfg(any(test, feature = "devctl"))]
    pub(in crate::window) token: String,
    pub(in crate::window) _scan: Option<Task<()>>,
}

/// Every welcome the app has running, and the doors to them: the first-run welcome's own global, made
/// the first time a welcome asks for it.
#[derive(Default)]
pub(crate) struct Welcomes {
    gates: Registry,
    runs: Vec<Run>,
    /// The first launch's window starts with a welcome terminal, not a plain one: set for as long as that
    /// window is being opened.
    first: Option<Plan>,
    /// The token of the welcome terminal being made, until its session takes it into its environment.
    pending: Option<(i64, String)>,
}

impl Global for Welcomes {}

/// The welcomes, if any ever started.
fn welcomes(cx: &App) -> Option<&Welcomes> {
    cx.try_global::<Welcomes>()
}

fn welcomes_mut(cx: &mut App) -> &mut Welcomes {
    cx.default_global::<Welcomes>()
}

impl Welcomes {
    fn run(&self, tab: i64) -> Option<&Run> {
        self.runs.iter().find(|run| run.tab == tab)
    }

    fn run_mut(&mut self, tab: i64) -> Option<&mut Run> {
        self.runs.iter_mut().find(|run| run.tab == tab)
    }
}

/// The run of the welcome in terminal `tab`, if it has one.
pub(in crate::window) fn run_of(tab: i64, cx: &App) -> Option<&Run> {
    welcomes(cx)?.run(tab)
}

/// The run of the welcome in terminal `tab`, to change.
pub(in crate::window) fn run_of_mut(tab: i64, cx: &mut App) -> Option<&mut Run> {
    welcomes_mut(cx).run_mut(tab)
}

/// The first launch's window is about to open: its first terminal is the welcome that `plan` says
/// (`None`: a plain one). Set for as long as that window is being opened.
pub(crate) fn set_first_launch(plan: Option<Plan>, cx: &mut App) {
    welcomes_mut(cx).first = plan;
}

/// The plan for the window being opened for the first launch, taken: only its first terminal is the
/// welcome.
pub(in crate::window) fn take_first_launch(cx: &mut App) -> Option<Plan> {
    welcomes_mut(cx).first.take()
}

/// The session `record` is made for takes the token of the welcome it is, if it is the welcome's terminal.
pub(in crate::window) fn take_token(id: i64, cx: &mut App) -> Option<String> {
    welcomes_mut(cx).pending.take_if(|(pending, _)| *pending == id).map(|(_, token)| token)
}

/// A welcome terminal `id` starts, with `token`: its door, its run and its token for the session about to
/// be made.
pub(in crate::window) fn begin(id: i64, token: String, run: Run, cx: &mut App) {
    let welcomes = welcomes_mut(cx);
    welcomes.gates.open(id, token.clone());
    welcomes.runs.retain(|run| run.tab != id);
    welcomes.runs.push(run);
    welcomes.pending = Some((id, token));
}

/// The plan for the first launch, when the terminal welcome is on and this is one (nothing saved, the
/// first run never finished or skipped): the window's first terminal is then the welcome.
pub(crate) fn first_launch_plan(cx: &App) -> Option<Plan> {
    let store = app(cx).store.as_ref()?;
    let flag = store.meta(ONBOARDED_KEY).ok().flatten();
    (should_show(flag.as_deref(), 0) && enabled(cx))
        .then_some(Plan { handover: Handover::Cursor, first_run: true })
}

/// Whether the terminal welcome is on (the feature flag).
pub(crate) fn enabled(cx: &App) -> bool {
    settings(cx).feature_enabled(wormhole_core::config::features::FeatureFlag::TerminalWelcome)
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

/// Whether the welcome in `tab` is still running (its token still opens the door).
pub(in crate::window) fn running(tab: i64, cx: &App) -> bool {
    welcomes(cx).is_some_and(|welcomes| welcomes.gates.running(tab))
}

/// A welcome terminal is gone (closed, or its window): its token opens nothing, and a first run that
/// was closed unfinished counts as skipped, so the next launch doesn't start it all over.
pub(in crate::window) fn forget(tab: i64, cx: &mut App) {
    let welcomes = welcomes_mut(cx);
    let first_run = welcomes.run(tab).is_some_and(|run| run.first_run && welcomes.gates.running(tab));
    welcomes.gates.forget(tab);
    welcomes.runs.retain(|run| run.tab != tab);
    if first_run {
        remember_first_run(cx);
    }
}

/// The first run is over: not shown again.
fn remember_first_run(cx: &mut App) {
    if let Some(store) = &app(cx).store
        && let Err(err) = store.set_meta(ONBOARDED_KEY, "1")
    {
        tracing::warn!("remembering the first run: {err:#}");
    }
}

#[cfg(test)]
mod tests;
