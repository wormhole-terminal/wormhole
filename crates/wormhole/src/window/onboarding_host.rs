//! The window's side of the first run (`onboarding::FirstRun`, an entity of its own): what it is told
//! before it is drawn, what the window does when it asks for something (`FirstRunEvent`), and the few
//! calls the rest of the window makes into it (Help → Welcome to Wormhole, the sidebar's note, devctl).
//! The first run never reads the `Workspace`; this is where the two meet.

use gpui_kit::*;
use wormhole_core::config::FeatureId;
#[cfg(any(test, feature = "devctl"))]
use wormhole_core::onboarding::Step;
use wormhole_core::onboarding::intro::{Handover, Start};
use wormhole_core::onboarding::{Onboarding, Tip};

use crate::app_state::app_mut;
use crate::onboarding::{Facts, FirstRunEvent, Opening, welcome};
use crate::window::Workspace;

impl Workspace {
    /// The first launch's window, never shown: the intro window opens over it and it arrives out of the
    /// animation.
    pub(crate) fn open_first_intro(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let opening = self.opening(Onboarding::new(true), true, Some(Start::Hidden), cx);
        self.first_run.update(cx, |first_run, cx| first_run.open_intro(opening, window, cx));
    }

    /// Open the intro window at `step`, with nothing played (devctl and the tests).
    #[cfg(any(test, feature = "devctl"))]
    pub(crate) fn open_onboarding(&mut self, step: Step, window: &mut Window, cx: &mut Context<Self>) {
        let model = if step == Step::Welcome { Onboarding::new(true) } else { Onboarding::at(step) };
        let opening = self.opening(model, true, None, cx);
        self.first_run.update(cx, |first_run, cx| first_run.open_intro(opening, window, cx));
    }

    /// How an intro window opens in this window: over its first terminal, with the projects opened
    /// recently to offer.
    fn opening(&self, model: Onboarding, first_run: bool, play: Option<Start>, cx: &App) -> Opening {
        Opening {
            model,
            first_run,
            first: self.sessions.first().map(|session| session.id),
            play,
            recent: crate::app_state::app(cx).recent_projects.roots().to_vec(),
        }
    }

    /// Whether the first run's Look step is up (devctl).
    #[cfg(feature = "devctl")]
    pub(crate) fn on_look_step(&self, cx: &App) -> bool {
        self.first_run.read(cx).on_look_step()
    }

    /// Do `act` in the Look step's walk through how much to show, as a key or a click would (devctl and
    /// the tests). Returns whether it did anything.
    #[cfg(any(test, feature = "devctl"))]
    pub(crate) fn tailor_act(&mut self, act: crate::onboarding::TailorAct, cx: &mut Context<Self>) -> bool {
        self.first_run.update(cx, |first_run, cx| first_run.tailor_act(act, cx))
    }

    /// Welcome to Wormhole (Help menu, palette): the intro again, its welcome page alone.
    pub(crate) fn show_intro(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // With the terminal welcome on, it is the welcome: a new terminal runs it, with the cursor
        // hand-over.
        if welcome::enabled(cx) {
            self.replay_welcome(Handover::Cursor, window, cx);
            return;
        }
        let opening = self.opening(Onboarding::replay(), false, Some(Start::Shown), cx);
        self.first_run.update(cx, |first_run, cx| {
            if first_run.is_open() {
                // It is up already: replay it.
                first_run.replay_intro(window, cx);
            } else {
                // The window is showing: it leaves, the animation plays, and it arrives.
                first_run.open_intro(opening, window, cx);
            }
        });
    }

    /// Close the intro window (devctl `onboarding close`), as Skip does but without remembering it,
    /// and skip a terminal welcome that is running (Esc, as the person would), so a fresh data dir's
    /// first launch is out of the way whichever of the two it opens.
    #[cfg(feature = "devctl")]
    pub(crate) fn close_onboarding(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.skip_welcomes(cx);
        self.first_run.update(cx, |first_run, cx| first_run.close(window, cx));
        window.focus(&self.active_terminal().focus_handle(cx), cx);
        cx.notify();
    }

    /// What the intro window is doing, for devctl's `state`.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_intro_state(&self, cx: &App) -> Option<String> {
        self.first_run.read(cx).devctl_intro_state(cx)
    }

    /// Tell the first run how the window stands, while its page is up.
    pub(super) fn push_first_run(&self, cx: &mut Context<Self>) {
        if !self.first_run.read(cx).is_open() {
            return;
        }
        let trust = self.features.iter().enumerate().find_map(|(at, def)| {
            let (hooks, entity) = (def.trust.as_ref()?, self.feature_entity(at)?);
            let active_root = (hooks.active_root)(entity, &self.facts(cx), cx);
            let root = self.first_run.read(cx).trust_root(active_root)?;
            (hooks.card)(&self.profile, &root, cx)
        });
        let panel_open = self.notes_visible();
        let notes_on = crate::settings::features_in(&self.profile, cx).has(FeatureId::Notes);
        self.first_run
            .update(cx, |first_run, cx| first_run.set_facts(Facts { trust, notes_on, panel_open }, cx));
    }

    /// The sidebar's note after a first run without agents, while it is up.
    pub(super) fn render_no_agents_note(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let dismissed = crate::app_state::app(cx).tips.contains(Tip::NoAgents);
        if !self.first_run.read(cx).note_wanted(dismissed) {
            return None;
        }
        let agent_seen = self.sessions.iter().any(|session| session.pane.summary.agent.is_some());
        self.first_run.update(cx, |first_run, cx| first_run.render_no_agents_note(agent_seen, cx))
    }

    /// The first run asks for something. Handled when the update that raised it ends.
    pub(super) fn on_first_run_event(
        &mut self,
        _: &Entity<crate::onboarding::FirstRun>,
        event: &FirstRunEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            FirstRunEvent::DismissNoAgentsTip => app_mut(cx).dismiss_tip(Tip::NoAgents),
            FirstRunEvent::Ended { replace, onboarded } => {
                if *onboarded {
                    crate::window::remember_first_run(cx);
                }
                if let Some((first, folder)) = replace
                    && let Some(ix) = self.sessions.iter().position(|session| session.id == *first)
                {
                    let there =
                        self.sessions[ix].terminal().read(cx).cwd.as_deref() == Some(folder.as_path());
                    if !there && self.sessions.len() == 1 {
                        self.spawn_session(None, Some(folder.clone()), window, cx);
                        self.close_session(ix, window, cx);
                    }
                }
                window.focus(&self.active_terminal().focus_handle(cx), cx);
                self.mark_dirty(cx);
            }
            FirstRunEvent::OpenWelcome { plan, play } => self.open_welcome_session(*plan, *play, window, cx),
            FirstRunEvent::PanelOpen(open) => {
                if self.notes_visible() != *open {
                    self.set_panel_open(*open, cx);
                }
            }
            FirstRunEvent::Trust(root) => {
                if let Some(approve) =
                    self.features.iter().find_map(|def| def.trust.as_ref()).map(|t| t.approve)
                {
                    approve(&self.profile, root, cx);
                }
            }
        }
    }
}
