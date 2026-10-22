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
use wormhole_core::agents::integrations;
use wormhole_core::ipc::control::AgentsFacts;
use wormhole_core::onboarding::intro::{Handover, Start};
use wormhole_core::onboarding::welcome::{self, DockCell, Registry};
use wormhole_core::onboarding::{ONBOARDED_KEY, should_show};
use wormhole_core::storage::store::SessionRecord;

use super::Workspace;
use super::app::{AppState, app, app_mut};
#[cfg(feature = "devctl")]
use super::notifications;
use crate::settings::settings;

pub(super) use answers::handle;

/// What a welcome is to be.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Plan {
    pub handover: Handover,
    /// The first run, which finishing is remembered of, or a replay.
    pub first_run: bool,
}

/// Whether the start animation plays around this welcome, and since when.
#[derive(Clone, Copy, Debug)]
enum Play {
    /// None plays (a test instance's first launch, no Metal): the program has nothing to wait for.
    Never,
    /// About to: the first launch opens its window and then plays; a request that comes in between
    /// waits for the whole run.
    Pending,
    Since(Instant),
}

/// One welcome terminal's side in the app.
struct Run {
    tab: i64,
    handover: Handover,
    first_run: bool,
    play: Play,
    /// The agents found; `None` until the search (off the UI thread) is done.
    agents: Option<AgentsFacts>,
    /// What went wrong with the last Install, until the next one that works.
    problem: Option<String>,
    /// devctl's `state` names it, so `welcome-probe` can send a request as the program would (and the
    /// tests do).
    #[cfg(any(test, feature = "devctl"))]
    token: String,
    _scan: Option<Task<()>>,
}

/// Every welcome the app has running, and the doors to them.
#[derive(Default)]
pub(crate) struct Welcomes {
    gates: Registry,
    runs: Vec<Run>,
}

impl Welcomes {
    fn run(&self, tab: i64) -> Option<&Run> {
        self.runs.iter().find(|run| run.tab == tab)
    }

    fn run_mut(&mut self, tab: i64) -> Option<&mut Run> {
        self.runs.iter_mut().find(|run| run.tab == tab)
    }
}

/// The plan for the first launch, when the terminal welcome is on and this is one (nothing saved, the
/// first run never finished or skipped): the window's first terminal is then the welcome.
pub(super) fn first_launch_plan(cx: &App) -> Option<Plan> {
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
pub(super) fn play_first(plan: Plan, cx: &mut App) {
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
        AppState::with_front_workspace(cx, |ws, window, cx| {
            crate::background::activate_window(window, cx);
            ws.replay_welcome(handover, window, cx);
        });
    });
}

/// Whether the welcome in `tab` is still running (its token still opens the door).
pub(super) fn running(tab: i64, cx: &App) -> bool {
    app(cx).welcomes.gates.running(tab)
}

/// A welcome terminal is gone (closed, or its window): its token opens nothing, and a first run that
/// was closed unfinished counts as skipped, so the next launch doesn't start it all over.
pub(super) fn forget(tab: i64, cx: &mut App) {
    let state = app_mut(cx);
    let first_run =
        state.welcomes.run(tab).is_some_and(|run| run.first_run && state.welcomes.gates.running(tab));
    state.welcomes.gates.forget(tab);
    state.welcomes.runs.retain(|run| run.tab != tab);
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

#[cfg(feature = "devctl")]
impl Workspace {
    /// devctl's `onboarding close`: press Esc in every terminal of this window whose welcome runs.
    pub(crate) fn skip_welcomes(&mut self, cx: &mut Context<Self>) {
        let running: Vec<_> = self
            .sessions
            .iter()
            .filter(|session| app(cx).welcomes.gates.running(session.id))
            .map(|session| session.terminal().clone())
            .collect();
        for terminal in running {
            terminal.update(cx, |terminal, _| terminal.write(b"\x1b".to_vec()));
        }
    }

    /// devctl's `state`: a line for each terminal that runs a welcome.
    pub(crate) fn devctl_welcome_state(&self, cx: &App) -> String {
        let welcomes = &app(cx).welcomes;
        let mut out = String::new();
        for (ix, session) in self.sessions.iter().enumerate() {
            let Some(run) = welcomes.run(session.id) else { continue };
            let agents = match &run.agents {
                None => "scanning".to_string(),
                Some(agents) => format!("{} found", agents.found.len()),
            };
            let demo = match self.welcome_demo_rows(cx) {
                None => "off".to_string(),
                Some(rows) => rows
                    .iter()
                    .map(|row| {
                        format!(
                            "{}={}",
                            row.name,
                            wormhole_core::view::sidebar::spoken::state_word(row.state)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(","),
            };
            out += &format!(
                "welcome terminal={} tab={} token={} handover={} first_run={} running={} play={:?} agents={agents} demo={demo} permission_asks={}\n",
                ix + 1,
                session.id,
                run.token,
                run.handover.key(),
                run.first_run,
                welcomes.gates.running(session.id),
                run.play,
                notifications::permission_asks(cx),
            );
        }
        out
    }
}

impl Workspace {
    /// The first terminal of a window opened for the first launch: the welcome, not a plain shell.
    pub(super) fn open_first_welcome(&mut self, plan: Plan, window: &mut Window, cx: &mut Context<Self>) {
        let play = if crate::background::on() { Play::Never } else { Play::Pending };
        self.open_welcome_session(plan, play, window, cx);
    }

    /// Play the wormhole run around this window with the hand-over `plan` asks for, from a window that
    /// is `start`ing shown or hidden (`None`: nothing plays). The welcome terminal is already there.
    fn start_welcome_animation(
        &mut self,
        plan: Plan,
        start: Option<Start>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let tab = self.sessions.iter().rev().map(|s| s.id).find(|id| app(cx).welcomes.run(*id).is_some());
        let played = start.is_some_and(|start| self.play_run(plan.handover, start, window, cx));
        if let Some(run) = tab.and_then(|id| app_mut(cx).welcomes.run_mut(id)) {
            run.play = if played { Play::Since(Instant::now()) } else { Play::Never };
        }
    }

    /// Start the run; whether it is playing.
    fn play_run(
        &mut self,
        handover: Handover,
        start: Start,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        // The window keeps its chrome: the welcome is in the terminal, the traffic lights stay.
        crate::intro::play_with(
            window,
            handover.style(),
            crate::intro::Options { start, lights: false, handover: Some(handover) },
            cx,
        );
        crate::intro::progress(cx).is_some()
    }

    /// Replay: the window leaves into the wormhole and comes back with a new terminal in it that
    /// runs the welcome with `handover`, as a replay of the first run (the welcome page alone).
    pub(crate) fn replay_welcome(&mut self, handover: Handover, window: &mut Window, cx: &mut Context<Self>) {
        self.replay_welcome_from(handover, Start::Shown, window, cx);
    }

    /// [`replay_welcome`](Self::replay_welcome) from a window that is `start`ing shown (it leaves
    /// first) or hidden (a first launch's, which devctl's `welcome <hand-over> hidden` imitates).
    pub(crate) fn replay_welcome_from(
        &mut self,
        handover: Handover,
        start: Start,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let plan = Plan { handover, first_run: false };
        let playing = self.play_run(handover, start, window, cx);
        let play = if playing { Play::Since(Instant::now()) } else { Play::Never };
        if !playing || start == Start::Hidden {
            self.open_welcome_session(plan, play, window, cx);
            return;
        }
        // The terminal opens once the window is out of sight, so it never shows its first moments
        // (the cursor before the program hides it); the program waits out the rest of the run.
        self.after_fade(window, cx, move |this, window, cx| {
            this.open_welcome_session(plan, play, window, cx);
        });
    }

    /// A new terminal that runs the welcome: its token, its place among the welcomes, and the cell
    /// the ship is to dock on. Falls back to a plain terminal when there is no random token to be had.
    fn open_welcome_session(&mut self, plan: Plan, play: Play, window: &mut Window, cx: &mut Context<Self>) {
        let Some(token) = welcome::new_token() else {
            tracing::warn!("no random token for the welcome: starting a plain terminal instead");
            self.new_session(window, cx);
            return;
        };
        let id = self.alloc_id(cx);
        let scan = cx.spawn(async move |this, cx| {
            let agents = cx
                .background_executor()
                .spawn(async move { welcome::facts::scan_agents(&integrations::home()) })
                .await;
            this.update(cx, |_, cx| {
                if let Some(run) = app_mut(cx).welcomes.run_mut(id) {
                    run.agents = Some(agents);
                }
                cx.notify();
            })
            .ok();
        });
        let state = app_mut(cx);
        state.welcomes.gates.open(id, token.clone());
        state.welcomes.runs.retain(|run| run.tab != id);
        state.welcomes.runs.push(Run {
            tab: id,
            handover: plan.handover,
            first_run: plan.first_run,
            play,
            agents: None,
            problem: None,
            #[cfg(any(test, feature = "devctl"))]
            token: token.clone(),
            _scan: Some(scan),
        });
        self.pending_welcome = Some((id, token));
        let cwd = self.default_cwd(cx);
        let record = SessionRecord::new(id, None, cwd);
        let ix = self.add_session(Some(record), window, cx);
        self.activate(ix, window, cx);
        self.dirty = true;
        let dock: Option<DockCell> = welcome::dock(plan.handover);
        self.sessions[ix].terminal().update(cx, |terminal, cx| terminal.set_dock(dock, cx));
    }

    /// The terminal the welcome ended in no longer reports a dock, the sidebar's example band goes
    /// with it, and the sidebar's note for a first run that ended on the primer (no agent on PATH)
    /// comes up.
    fn welcome_ended(&mut self, tab: i64, primer: bool, cx: &mut Context<Self>) {
        if let Some(session) = self.sessions.iter().find(|s| s.id == tab) {
            session.terminal().update(cx, |terminal, cx| terminal.set_dock(None, cx));
        }
        self.end_welcome_demo(tab);
        if primer {
            self.first_run.show_no_agents_note();
        }
        cx.notify();
    }
}
