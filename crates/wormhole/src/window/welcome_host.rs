//! The window's side of the terminal welcome (`onboarding::welcome` keeps its state and answers its
//! requests): the first terminal of a first launch being the welcome, the run played around the window,
//! a replay opening a new terminal that runs it, and what the window does when the welcome ends. The
//! welcome's state is its own global; this is where it meets the terminals.

use std::time::Instant;

use gpui_kit::*;
use wormhole_core::agents::integrations;
use wormhole_core::onboarding::ONBOARDED_KEY;
use wormhole_core::onboarding::intro::{Handover, Start};
use wormhole_core::onboarding::welcome::{self as core_welcome, DockCell};
use wormhole_core::storage::store::SessionRecord;

use crate::onboarding::FirstRunEvent;
use crate::onboarding::welcome::{self, Plan, Play, Run};
use crate::window::Workspace;

#[cfg(feature = "devctl")]
impl Workspace {
    /// devctl's `onboarding close`: press Esc in every terminal of this window whose welcome runs.
    pub(crate) fn skip_welcomes(&mut self, cx: &mut Context<Self>) {
        let running: Vec<_> = self
            .sessions
            .iter()
            .filter(|session| welcome::running(session.id, cx))
            .map(|session| session.terminal().clone())
            .collect();
        for terminal in running {
            terminal.update(cx, |terminal, _| terminal.write(b"\x1b".to_vec()));
        }
    }

    /// devctl's `state`: a line for each terminal that runs a welcome.
    pub(crate) fn devctl_welcome_state(&self, permission_asks: u32, cx: &App) -> String {
        let mut out = String::new();
        for (ix, session) in self.sessions.iter().enumerate() {
            let Some(run) = welcome::run_of(session.id, cx) else { continue };
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
                welcome::running(session.id, cx),
                run.play,
                permission_asks,
            );
        }
        out
    }
}

/// The first run is over: not shown again.
pub(crate) fn remember_first_run(cx: &mut App) {
    if let Some(store) = &crate::app_state::app(cx).store
        && let Err(err) = store.set_meta(ONBOARDED_KEY, "1")
    {
        tracing::warn!("remembering the first run: {err:#}");
    }
}

impl Workspace {
    /// The first terminal of a window opened for the first launch: the welcome, not a plain shell.
    pub(in crate::window) fn open_first_welcome(
        &mut self,
        plan: Plan,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let play = if crate::background::on() { Play::Never } else { Play::Pending };
        self.open_welcome_session(plan, play, window, cx);
    }

    /// Play the wormhole run around this window with the hand-over `plan` asks for, from a window that
    /// is `start`ing shown or hidden (`None`: nothing plays). The welcome terminal is already there.
    pub(crate) fn start_welcome_animation(
        &mut self,
        plan: Plan,
        start: Option<Start>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let tab = self.sessions.iter().rev().map(|s| s.id).find(|id| welcome::run_of(*id, cx).is_some());
        let played = start.is_some_and(|start| self.play_run(plan.handover, start, window, cx));
        if let Some(run) = tab.and_then(|id| welcome::run_of_mut(id, cx)) {
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
        self.first_run.update(cx, |first_run, cx| {
            first_run
                .after_fade(window, cx, move |_, _, cx| cx.emit(FirstRunEvent::OpenWelcome { plan, play }));
        });
    }

    /// A new terminal that runs the welcome: its token, its place among the welcomes, and the cell
    /// the ship is to dock on. Falls back to a plain terminal when there is no random token to be had.
    pub(in crate::window) fn open_welcome_session(
        &mut self,
        plan: Plan,
        play: Play,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(token) = core_welcome::new_token() else {
            tracing::warn!("no random token for the welcome: starting a plain terminal instead");
            self.new_session(window, cx);
            return;
        };
        let id = self.alloc_id(cx);
        let scan = cx.spawn(async move |this, cx| {
            let agents = cx
                .background_executor()
                .spawn(async move { core_welcome::facts::scan_agents(&integrations::home()) })
                .await;
            this.update(cx, |_, cx| {
                if let Some(run) = welcome::run_of_mut(id, cx) {
                    run.agents = Some(agents);
                }
                cx.notify();
            })
            .ok();
        });
        welcome::begin(
            id,
            token.clone(),
            Run {
                tab: id,
                handover: plan.handover,
                first_run: plan.first_run,
                play,
                agents: None,
                problem: None,
                imported_theme: None,
                #[cfg(any(test, feature = "devctl"))]
                token,
                _scan: Some(scan),
            },
            cx,
        );
        let cwd = self.default_cwd(cx);
        let record = SessionRecord::new(id, None, cwd);
        let ix = self.add_session(Some(record), window, cx);
        self.activate(ix, window, cx);
        self.dirty = true;
        let dock: Option<DockCell> = core_welcome::dock(plan.handover);
        self.sessions[ix].terminal().update(cx, |terminal, cx| terminal.set_dock(dock, cx));
    }

    /// The terminal the welcome ended in no longer reports a dock, the sidebar's example band goes
    /// with it, and the sidebar's note for a first run that ended on the primer (no agent on PATH)
    /// comes up.
    pub(crate) fn welcome_ended(&mut self, tab: i64, primer: bool, cx: &mut Context<Self>) {
        if let Some(session) = self.sessions.iter().find(|s| s.id == tab) {
            session.terminal().update(cx, |terminal, cx| terminal.set_dock(None, cx));
        }
        self.end_welcome_demo(tab, cx);
        if primer {
            self.first_run.update(cx, |first_run, cx| first_run.show_no_agents_note(cx));
        }
        cx.notify();
    }
}
