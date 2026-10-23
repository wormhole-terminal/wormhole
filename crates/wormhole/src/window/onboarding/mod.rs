//! The intro window: what someone new sees instead of the terminal. A welcome page, then the
//! three steps (where you work, how it looks, and the coding agents with the folder's trust, or a
//! primer when no agent is on PATH). The steps are core's `onboarding` model; this draws them and
//! does what they ask.
//!
//! The window plays into the wormhole animation: it arrives out of it and the page comes in part by
//! part. The traffic lights, which are the window's chrome, are away for as long as the intro window
//! is up and come back when it closes (the run only fades them out when it begins over a showing
//! window, `intro`). Any key or click while the page is still coming in cuts the run short. It is the whole window, hiding the sidebar and the terminal, and it holds the keyboard:
//! ←, →, ⏎, Esc and R, ⇥ between its buttons, and the mouse for all of it. A replay button in the
//! corner plays it again.
//!
//! Nothing runs or is written behind the person's back: an agent's hooks are installed only by
//! its own Install button, the folder is trusted only from "Trust with Touch ID…", and Start
//! itself installs nothing.
//!
//! After Start on the primer, the window's sidebar keeps a quiet note at its bottom while none
//! of its terminals has an agent (`render_no_agents_note`); its × closes it, and then one line
//! offers "Don't show again", which Settings → Advanced → Reset tips undoes.

mod agents_step;
mod keys;
mod look_step;
mod page;
mod steps;
#[cfg(test)]
mod tests;
pub(crate) mod welcome;
mod welcome_page;

use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::*;
use wormhole_core::agents::integrations::{self, Spec, State};
use wormhole_core::agents::{self, AGENTS, Agent};
use wormhole_core::config::settings_index::SectionId;
use wormhole_core::onboarding::intro::Start;
use wormhole_core::onboarding::{
    self, Action, Event, ONBOARDED_KEY, Onboarding, Place, Step, Tip, should_show,
};

use crate::actions::OpenSettingsAt;
use crate::app_state::{AppState, app, app_mut};
use crate::settings::{UiColors, ui};
use crate::style::{self, Edges, TextRole};
use crate::window::Workspace;
use crate::window::features::project;

pub(crate) use keys::init;
pub(in crate::window) use look_step::{offered_primaries, offered_themes};

/// An agent on PATH, as the third step lists it.
#[derive(Clone)]
struct FoundAgent {
    agent: &'static Agent,
    spec: Option<&'static Spec>,
    /// The hooks' state; `None` without hook support or when the file didn't read.
    state: Option<State>,
    /// The file the hooks go in, for the row (`~/.claude/settings.json`).
    path: Option<String>,
}

/// What was found on PATH: the agents there, and the first of the others with their count.
#[derive(Clone, Default)]
struct Agents {
    found: Vec<FoundAgent>,
    others: Vec<&'static Agent>,
}

impl Agents {
    /// Every agent's command looked up on PATH and its hooks read under `home`: a few hundred
    /// `stat`s, so off the UI thread.
    fn scan(home: &Path) -> Self {
        let mut agents = Agents::default();
        for agent in AGENTS {
            if !agents::installed(agent) {
                agents.others.push(agent);
                continue;
            }
            let spec = integrations::spec(agent.id);
            agents.found.push(FoundAgent {
                agent,
                spec,
                state: spec.and_then(|spec| spec.state(home).ok()),
                path: spec.map(|spec| spec.display_path(home)),
            });
        }
        agents
    }
}

/// The intro window while it is open.
pub(super) struct Intro {
    model: Onboarding,
    focus: FocusHandle,
    /// `None` until the agents were looked for.
    agents: Option<Agents>,
    _scan: Option<Task<()>>,
    /// The terminal that was there when the first run began: Start moves it to the folder picked.
    first: Option<i64>,
    /// The folders the first step offers, read once when the intro opens.
    places: Vec<Place>,
    /// "Later" was pressed on the folder's trust.
    trust_later: bool,
    /// What went wrong writing an agent's hooks.
    problem: Option<SharedString>,
    /// The first run, which finishing is remembered of and starts the first terminal in the folder
    /// picked, or a replay of the intro, which changes nothing.
    first_run: bool,
}

/// The sidebar's "No coding agents yet" note after a primer, as the window keeps it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum AgentsNote {
    #[default]
    Off,
    /// Up until a terminal of the window runs an agent, or its × is pressed.
    Shown,
    /// Its × was pressed: in its place, one line offers "Don't show again".
    Offer,
}

/// The first run as a window keeps it: the intro window, and the sidebar's note after a primer.
#[derive(Default)]
pub(super) struct FirstRun {
    intro: Option<Intro>,
    /// A change of page waiting for the window to be out of sight (a replay's start).
    swap: Option<Task<()>>,
    /// The intro ended on the primer (no agent on PATH): the sidebar says so until an agent shows.
    agents_note: AgentsNote,
}

impl FirstRun {
    /// The intro window is over the window (its keys go to it, the window's actions wait).
    pub(super) fn is_open(&self) -> bool {
        self.intro.is_some()
    }

    /// The terminal welcome ended on the primer (no agent on PATH): the sidebar says so until an
    /// agent shows, as after the intro window's primer.
    pub(super) fn show_no_agents_note(&mut self) {
        self.agents_note = AgentsNote::Shown;
    }
}

/// How the window leaves as a replay begins, the page changing once it is gone (core's
/// `wormhole::window_motion` takes 0.3 s).
const FADE_OUT: Duration = Duration::from_millis(320);

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
        AppState::with_workspace(&window, cx, |ws, window, cx| {
            // The window was never shown: it arrives out of the animation.
            ws.open_intro(Onboarding::new(true), true, Some(Start::Hidden), window, cx)
        });
    });
}

impl Workspace {
    /// Open the intro window at `step`, with nothing played (devctl and the tests).
    #[cfg(any(test, feature = "devctl"))]
    pub(crate) fn open_onboarding(&mut self, step: Step, window: &mut Window, cx: &mut Context<Self>) {
        let model = if step == Step::Welcome { Onboarding::new(true) } else { Onboarding::at(step) };
        self.open_intro(model, true, None, window, cx);
    }

    /// Welcome to Wormhole (Help menu, palette): the intro again, its welcome page alone.
    pub(crate) fn show_intro(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // With the terminal welcome on, it is the welcome: a new terminal runs it, with the cursor
        // hand-over.
        if crate::window::onboarding::welcome::enabled(cx) {
            self.replay_welcome(wormhole_core::onboarding::intro::Handover::Cursor, window, cx);
            return;
        }
        if self.first_run.is_open() {
            // It is up already: replay it.
            self.replay_intro(window, cx);
        } else {
            // The window is showing: it leaves, the animation plays, and it arrives.
            self.open_intro(Onboarding::replay(), false, Some(Start::Shown), window, cx);
        }
    }

    /// Open the intro window, with the animation playing around it from a window that is `play`ing
    /// as shown or never shown, or with nothing played when that is `None`. `first_run` is whether
    /// finishing it is the first run (remembered, the first terminal moving to the folder picked) or
    /// only a replay.
    fn open_intro(
        &mut self,
        model: Onboarding,
        first_run: bool,
        play: Option<Start>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let intro = self.new_intro(model, first_run, cx);
        // Test instances stay behind other windows, and their screenshots don't need a show.
        let start = play.filter(|_| !crate::background::on());
        let Some(start) = start else {
            self.show_page(intro, window, cx);
            return;
        };
        // Over a window that is showing, its lights fade out as the run begins; the page's own.
        crate::intro::play_with(
            window,
            crate::intro::FIRST_LAUNCH,
            crate::intro::Options { start, lights: start == Start::Shown, handover: None },
            cx,
        );
        if start == Start::Hidden {
            self.show_page(intro, window, cx);
        } else {
            // The window is showing and leaves first; the page changes once it is out of sight.
            self.after_fade(window, cx, move |this, window, cx| this.show_page(intro, window, cx));
        }
    }

    /// The page is the intro window's: put it up, with the keyboard.
    fn show_page(&mut self, intro: Intro, window: &mut Window, cx: &mut Context<Self>) {
        crate::platform::window::set_shown(window, false);
        window.focus(&intro.focus, cx);
        self.first_run.intro = Some(intro);
        cx.notify();
    }

    /// Run `then` once the window has left, as a replay begins.
    pub(in crate::window) fn after_fade(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        then: impl FnOnce(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) {
        self.first_run.swap = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(FADE_OUT).await;
            this.update_in(cx, |this, window, cx| then(this, window, cx)).ok();
        }));
    }

    /// The replay button (and R): the window leaves, the animation plays again and the intro comes
    /// back to its welcome page.
    pub(crate) fn replay_intro(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.first_run.intro.is_none() {
            return;
        }
        // The page is up, so the lights are away already.
        crate::intro::play_with(
            window,
            crate::intro::FIRST_LAUNCH,
            crate::intro::Options { start: Start::Shown, lights: false, handover: None },
            cx,
        );
        self.after_fade(window, cx, |this, window, cx| {
            if let Some(intro) = &mut this.first_run.intro {
                intro.model.to_welcome();
                window.focus(&intro.focus, cx);
            }
            cx.notify();
        });
        cx.notify();
    }

    /// The state of a new intro window; the agents are looked for off the UI thread.
    fn new_intro(&mut self, model: Onboarding, first_run: bool, cx: &mut Context<Self>) -> Intro {
        let focus = cx.focus_handle();
        let first = self.sessions.first().map(|s| s.id);
        // A handful of `stat`s, once: the places don't change while the intro is up.
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
        let places = onboarding::places(&home, app(cx).recent_projects.roots(), |path| path.is_dir());
        let scan = cx.spawn(async move |this, cx| {
            let home = integrations::home();
            let agents = cx.background_executor().spawn(async move { Agents::scan(&home) }).await;
            this.update(cx, |this, cx| {
                if let Some(intro) = &mut this.first_run.intro {
                    // devctl's `onboarding primer` shows the primer whatever is on PATH.
                    if intro.model.step() != Step::Primer {
                        intro.model.set_agents_found(!agents.found.is_empty());
                    }
                    intro.agents = Some(agents);
                    cx.notify();
                }
            })
            .ok();
        });
        Intro {
            model,
            focus,
            agents: None,
            _scan: Some(scan),
            first,
            places,
            trust_later: false,
            problem: None,
            first_run,
        }
    }

    /// A folder dropped on the first step becomes the intro's (a file: the folder it is in).
    fn drop_folder(&mut self, files: &ExternalPaths, window: &mut Window, cx: &mut Context<Self>) {
        let on_folder_step = self.first_run.intro.as_ref().is_some_and(|s| s.model.step() == Step::Folder);
        if !on_folder_step {
            return;
        }
        let folder = files.paths().iter().find_map(|path| {
            if path.is_dir() { Some(path.clone()) } else { path.parent().map(Path::to_path_buf) }
        });
        if let Some(folder) = folder {
            self.onboarding_event(Event::PickFolder(folder), window, cx);
        }
    }

    /// What the intro window is doing, for devctl's `state`: the page, the animation and how far the
    /// page has entered; `None` when it isn't open.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_intro_state(&self, cx: &App) -> Option<String> {
        let intro = self.first_run.intro.as_ref()?;
        let playing = match crate::intro::progress(cx) {
            Some(p) => format!("{:.2}s{}", p.seconds, if p.skipped { " skipped" } else { "" }),
            None => "no".to_string(),
        };
        Some(format!(
            "intro page={:?} first_run={} playing={playing} entered={:.2}",
            intro.model.step(),
            intro.first_run,
            self.intro_entrance(cx).least()
        ))
    }

    /// Close the intro window (devctl `onboarding close`), as Skip does but without remembering it,
    /// and skip a terminal welcome that is running (Esc, as the person would), so a fresh data dir's
    /// first launch is out of the way whichever of the two it opens.
    #[cfg(feature = "devctl")]
    pub(crate) fn close_onboarding(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.skip_welcomes(cx);
        self.first_run.intro = None;
        self.first_run.swap = None;
        crate::platform::window::set_shown(window, true);
        crate::intro::skip(cx);
        window.focus(&self.active_terminal().focus_handle(cx), cx);
        cx.notify();
    }

    fn onboarding_event(&mut self, event: Event, window: &mut Window, cx: &mut Context<Self>) {
        let profile = self.profile().clone();
        let Some(intro) = &mut self.first_run.intro else { return };
        let primer = intro.model.step() == Step::Primer;
        let moves = matches!(event, Event::Continue | Event::Back | Event::Start);
        match intro.model.apply(event) {
            Action::None => {
                // A click on a button leaves the keyboard on it; the arrows are the page's.
                if moves {
                    window.focus(&intro.focus, cx);
                }
            }
            Action::Skip => self.end_onboarding(None, false, window, cx),
            Action::Finish { folder } => self.end_onboarding(folder, primer, window, cx),
            Action::InstallHooks(id) => {
                let Some(spec) = integrations::spec(id) else { return };
                let home = integrations::home();
                let result = spec.install(&home);
                if let Err(err) = &result {
                    tracing::warn!("{id:?} hooks: {err}");
                }
                intro.problem = result.err().map(|err| format!("Couldn't write the hooks: {err}").into());
                // Read the hooks back, so the row says what is there now.
                if let Some(agents) = &mut intro.agents
                    && let Some(found) = agents.found.iter_mut().find(|f| f.agent.id == id)
                {
                    found.state = spec.state(&home).ok();
                }
            }
            Action::Trust(root) => project::api::approve(&profile, &root, cx),
        }
        cx.notify();
    }

    /// The intro is done: on the first run remember it and put the first terminal in the folder
    /// picked, and give the keyboard back to the terminal. The animation, if it is still playing,
    /// ends with it.
    fn end_onboarding(
        &mut self,
        folder: Option<PathBuf>,
        primer: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.first_run.swap = None;
        let Some(intro) = self.first_run.intro.take() else { return };
        crate::platform::window::set_shown(window, true);
        crate::intro::skip(cx);
        if intro.first_run {
            if let Some(store) = &app(cx).store
                && let Err(err) = store.set_meta(ONBOARDED_KEY, "1")
            {
                tracing::warn!("remembering the first run: {err:#}");
            }
            self.first_run.agents_note = if primer { AgentsNote::Shown } else { AgentsNote::Off };
            // The first terminal never had the keyboard while the intro was open, so nothing was
            // typed into it: a new one in the folder takes its place.
            let first_ix = intro.first.and_then(|id| self.sessions.iter().position(|s| s.id == id));
            if let (Some(folder), Some(ix)) = (folder, first_ix) {
                let there = self.sessions[ix].terminal().read(cx).cwd.as_deref() == Some(folder.as_path());
                if !there && self.sessions.len() == 1 {
                    self.spawn_session(None, Some(folder), window, cx);
                    self.close_session(ix, window, cx);
                }
            }
        }
        window.focus(&self.active_terminal().focus_handle(cx), cx);
        self.mark_dirty(cx);
    }

    /// Open folder…: the system's folder picker; the folder picked becomes the intro's.
    fn pick_folder(&mut self, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Start here".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = paths.await else { return };
            let Some(folder) = paths.into_iter().next() else { return };
            this.update(cx, |this, cx| {
                if let Some(intro) = &mut this.first_run.intro {
                    intro.model.apply(Event::PickFolder(folder));
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    /// The sidebar's note after a first run without agents: what rows show instead, and
    /// the way to the agents' settings. Gone once a terminal of the window runs an agent, or
    /// with its ×; closed for good with "Don't show again".
    pub(super) fn render_no_agents_note(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let note = self.first_run.agents_note;
        if note == AgentsNote::Off || app(cx).tips.contains(Tip::NoAgents) {
            return None;
        }
        let agent_seen = self.sessions.iter().any(|session| session.terminal().read(cx).agent.is_some());
        if agent_seen {
            return None;
        }
        let ui = ui(cx);
        if note == AgentsNote::Offer {
            return Some(self.render_note_offer(ui, cx));
        }
        Some(
            div()
                .flex()
                .gap_2()
                .pl_4()
                .pr_2()
                .py_3()
                .rule_top(ui)
                .child(style::lead(style::icon(IconName::Sparkles).text_color(ui.muted)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(div().text_label().text_color(ui.text).child("No coding agents yet"))
                        .child(div().text_caption().text_color(ui.muted).child(
                            "Rows show the running command instead. Status glyphs still mark a busy terminal.",
                        ))
                        .child(
                            style::link_button("no-agents-settings", ui, "Coding agents settings", false)
                                .ml(px(-NOTE_LINK_INSET))
                                .on_click(|_, window, cx| {
                                    window.dispatch_action(Box::new(OpenSettingsAt(SectionId::Agents)), cx)
                                }),
                        ),
                )
                // 22 drawn, clicked across 28 (HIT_MIN), as a tab's ×.
                .child(style::hit_area(
                    "no-agents-close-hit",
                    style::CONTROL_H_SM,
                    style::close_x("no-agents-close", ui, style::SelectedFill::Plain),
                    |hit| {
                        hit.role(Role::Button).aria_label("Close tip").on_click(cx.listener(|this, _, _, cx| {
                            this.first_run.agents_note = AgentsNote::Offer;
                            cx.notify();
                        }))
                    },
                ))
                .into_any_element(),
        )
    }

    /// Where the note was, once its × was pressed: a way to keep it closed for good, or just
    /// closed for now.
    fn render_note_offer(&self, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .pl_4()
            .pr_2()
            .py_1()
            .rule_top(ui)
            .child(style::lead(style::icon(IconName::Sparkles).text_color(ui.faint)))
            .child(
                div().flex_1().min_w_0().truncate().text_caption().text_color(ui.muted).child("Tip closed"),
            )
            .child(style::link_button("no-agents-never", ui, "Don't show again", false).on_click(
                cx.listener(|this, _, _, cx| {
                    app_mut(cx).dismiss_tip(Tip::NoAgents);
                    this.first_run.agents_note = AgentsNote::Off;
                    cx.notify();
                }),
            ))
            .child(style::hit_area(
                "no-agents-offer-close-hit",
                style::CONTROL_H_SM,
                style::close_x("no-agents-offer-close", ui, style::SelectedFill::Plain),
                |hit| {
                    hit.role(Role::Button).aria_label("Keep the tip for next time").on_click(cx.listener(
                        |this, _, _, cx| {
                            this.first_run.agents_note = AgentsNote::Off;
                            cx.notify();
                        },
                    ))
                },
            ))
            .into_any_element()
    }
}

/// How far the note's link reaches left past its text column, so its words line up with the
/// lines above (a link button has a small inset of its own).
const NOTE_LINK_INSET: f32 = 4.0;

#[cfg(test)]
mod agent_scan_tests {
    use wormhole_core::agents::{AGENTS, AgentId};

    use super::Agents;

    /// A scratch home directory, removed at the end of the test.
    struct Home(std::path::PathBuf);

    impl Home {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("wormhole-onboarding-{name}-{}", std::process::id()));
            std::fs::create_dir_all(&dir).expect("scratch home");
            Self(dir)
        }
    }

    impl Drop for Home {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn the_scan_lists_every_agent_once_and_writes_nothing() {
        let home = Home::new("scan");
        let agents = Agents::scan(&home.0);
        assert_eq!(agents.found.len() + agents.others.len(), AGENTS.len());
        // Looking writes no hooks: the scratch home is as empty as before.
        let entries = std::fs::read_dir(&home.0).expect("readable").count();
        assert_eq!(entries, 0, "the scan wrote into the home folder");
        assert!(AGENTS.iter().any(|a| a.id == AgentId::Claude));
    }
}
