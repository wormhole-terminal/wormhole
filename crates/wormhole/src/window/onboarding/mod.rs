//! The intro window: what someone new sees instead of the terminal. A welcome page, then the
//! three steps (where you work, how it looks, and the coding agents with the folder's trust, or a
//! primer when no agent is on PATH). The steps are core's `onboarding` model; this draws them and
//! does what they ask.
//!
//! It is an entity of its own, [`FirstRun`], that the window hosts as a child view. It never reads the
//! `Workspace`: the window tells it what it needs before it is drawn ([`Facts`], the terminal that was
//! there when it began, whether an agent runs) and hears what it asks as a [`FirstRunEvent`]
//! (`window::onboarding_host`).
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
use wormhole_core::onboarding::{self, Action, Event, Onboarding, Place, Step};

use crate::actions::OpenSettingsAt;

use self::welcome::{Plan, Play};
use crate::settings::{UiColors, ui};
use crate::style::{self, Edges, TextRole};

pub(crate) use keys::init;
pub(crate) use look_step::{offered_primaries, offered_themes};

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
struct Intro {
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

/// The first run as a window keeps it, an entity of its own that the window hosts: the intro window, and
/// the sidebar's note after a primer. The window tells it what it needs of the window (`set_facts`, the
/// terminal that was there when it began, whether an agent runs) and hears from it by [`FirstRunEvent`].
#[derive(Default)]
pub(crate) struct FirstRun {
    intro: Option<Intro>,
    /// A change of page waiting for the window to be out of sight (a replay's start).
    swap: Option<Task<()>>,
    /// The intro ended on the primer (no agent on PATH): the sidebar says so until an agent shows.
    agents_note: AgentsNote,
    /// What the window last said of itself.
    facts: Facts,
}

/// How an intro window opens: what it is of, and what the window knows that it needs from the start.
pub(crate) struct Opening {
    pub(crate) model: Onboarding,
    /// Whether finishing it is the first run (remembered, the first terminal moving to the folder
    /// picked) or only a replay.
    pub(crate) first_run: bool,
    /// The window's terminal that is there now.
    pub(crate) first: Option<i64>,
    /// Whether the animation plays around it, and from where.
    pub(crate) play: Option<Start>,
    /// The projects opened recently, which the first step offers.
    pub(crate) recent: Vec<PathBuf>,
}

/// What the intro needs to know of the window it is in, said before it is drawn.
#[derive(Default)]
pub(crate) struct Facts {
    /// The trust card of the project the intro is about, if it has one to show.
    pub(crate) trust: Option<TrustCard>,
    /// Whether the window's profile has Notes on, for the words and keys that name it.
    pub(crate) notes_on: bool,
}

/// The project folder's trust card, as the window's profile sees the project (the window asks the project
/// feature, so the intro names no feature).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TrustCard {
    /// The project's root: what the button trusts.
    pub(crate) root: PathBuf,
    /// The project folder's name, and the name of its `.wormhole` directory on disk.
    pub(crate) name: String,
    pub(crate) dir: String,
    /// How many commands trusting it would let run.
    pub(crate) commands: usize,
    /// The approval dialog is up.
    pub(crate) approving: bool,
}

/// What the intro asks of the window. Delivered when the update that raised it ends.
#[derive(Debug)]
pub(crate) enum FirstRunEvent {
    /// The intro window is over and the keyboard goes back to the terminal. On a first run that picked a
    /// folder, the terminal that was there when it began (`first`) is replaced by one in the folder, if it
    /// is still the window's only one and not there already.
    Ended {
        replace: Option<(i64, PathBuf)>,
        /// It was the first run: not shown again.
        onboarded: bool,
    },
    /// "Don't show again" on the sidebar's note after a primer: the tip is dismissed for good.
    DismissNoAgentsTip,
    /// A replay's window has left: the terminal that runs the welcome opens now.
    OpenWelcome { plan: Plan, play: Play },
    /// "Trust with Touch ID…" was pressed on the project at this root: the window asks for the approval,
    /// as its profile.
    Trust(PathBuf),
}

impl EventEmitter<FirstRunEvent> for FirstRun {}

impl FirstRun {
    /// The project whose trust card the agents step shows: the folder picked in the first step, else the
    /// project of the terminal in front (`active_root`). `None` when the step shows no card (another
    /// step, or "Later" was pressed). The window builds the card from it ([`Facts::trust`]).
    pub(crate) fn trust_root(&self, active_root: Option<PathBuf>) -> Option<PathBuf> {
        let intro = self.intro.as_ref().filter(|intro| !intro.trust_later)?;
        if intro.model.step() != Step::Agents {
            return None;
        }
        match intro.model.folder() {
            Some(folder) => wormhole_core::project::find_project(folder).map(|found| found.root),
            None => active_root,
        }
    }

    /// The window's facts, before the intro is drawn.
    pub(crate) fn set_facts(&mut self, facts: Facts) {
        self.facts = facts;
    }

    /// The intro window is over the window (its keys go to it, the window's actions wait).
    pub(crate) fn is_open(&self) -> bool {
        self.intro.is_some()
    }

    /// Whether the sidebar's note is up or offered (the window then says whether an agent runs).
    pub(crate) fn note_wanted(&self, tip_dismissed: bool) -> bool {
        self.agents_note != AgentsNote::Off && !tip_dismissed
    }

    /// The terminal welcome ended on the primer (no agent on PATH): the sidebar says so until an
    /// agent shows, as after the intro window's primer.
    pub(crate) fn show_no_agents_note(&mut self, cx: &mut Context<Self>) {
        self.agents_note = AgentsNote::Shown;
        cx.notify();
    }
}

/// How the window leaves as a replay begins, the page changing once it is gone (core's
/// `wormhole::window_motion` takes 0.3 s).
const FADE_OUT: Duration = Duration::from_millis(320);

impl FirstRun {
    /// Open the intro window, with the animation playing around it from a window that is `play`ing
    /// as shown or never shown, or with nothing played when that is `None`. `first_run` is whether
    /// finishing it is the first run (remembered, the first terminal moving to the folder picked) or
    /// only a replay; `first` is the window's terminal that is there now.
    pub(crate) fn open_intro(&mut self, opening: Opening, window: &mut Window, cx: &mut Context<Self>) {
        let Opening { model, first_run, first, play, recent } = opening;
        let intro = self.new_intro(model, first_run, first, &recent, cx);
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
        self.intro = Some(intro);
        cx.notify();
    }

    /// Run `then` once the window has left, as a replay begins.
    pub(crate) fn after_fade(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        then: impl FnOnce(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) {
        self.swap = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(FADE_OUT).await;
            this.update_in(cx, |this, window, cx| then(this, window, cx)).ok();
        }));
    }

    /// The replay button (and R): the window leaves, the animation plays again and the intro comes
    /// back to its welcome page.
    pub(crate) fn replay_intro(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.intro.is_none() {
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
            if let Some(intro) = &mut this.intro {
                intro.model.to_welcome();
                window.focus(&intro.focus, cx);
            }
            cx.notify();
        });
        cx.notify();
    }

    /// The state of a new intro window; the agents are looked for off the UI thread. `first` is the
    /// window's terminal that is there now.
    fn new_intro(
        &mut self,
        model: Onboarding,
        first_run: bool,
        first: Option<i64>,
        recent: &[PathBuf],
        cx: &mut Context<Self>,
    ) -> Intro {
        let focus = cx.focus_handle();
        // A handful of `stat`s, once: the places don't change while the intro is up.
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
        let places = onboarding::places(&home, recent, |path| path.is_dir());
        let scan = cx.spawn(async move |this, cx| {
            let home = integrations::home();
            let agents = cx.background_executor().spawn(async move { Agents::scan(&home) }).await;
            this.update(cx, |this, cx| {
                if let Some(intro) = &mut this.intro {
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
        let on_folder_step = self.intro.as_ref().is_some_and(|s| s.model.step() == Step::Folder);
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
        let intro = self.intro.as_ref()?;
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

    /// Close the intro window without remembering it (devctl `onboarding close`, as Skip but unmarked).
    #[cfg(feature = "devctl")]
    pub(crate) fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.intro = None;
        self.swap = None;
        crate::platform::window::set_shown(window, true);
        crate::intro::skip(cx);
        cx.notify();
    }

    fn onboarding_event(&mut self, event: Event, window: &mut Window, cx: &mut Context<Self>) {
        let Some(intro) = &mut self.intro else { return };
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
            Action::Trust(root) => cx.emit(FirstRunEvent::Trust(root)),
        }
        cx.notify();
    }

    /// The intro is done: on the first run remember it and have the window put the first terminal in the
    /// folder picked, and give the keyboard back to the terminal. The animation, if it is still playing,
    /// ends with it.
    fn end_onboarding(
        &mut self,
        folder: Option<PathBuf>,
        primer: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.swap = None;
        let Some(intro) = self.intro.take() else { return };
        crate::platform::window::set_shown(window, true);
        crate::intro::skip(cx);
        let mut replace = None;
        if intro.first_run {
            self.agents_note = if primer { AgentsNote::Shown } else { AgentsNote::Off };
            // The first terminal never had the keyboard while the intro was open, so nothing was
            // typed into it: a new one in the folder takes its place.
            replace = intro.first.zip(folder);
        }
        cx.emit(FirstRunEvent::Ended { replace, onboarded: intro.first_run });
        cx.notify();
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
                if let Some(intro) = &mut this.intro {
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
    /// `agent_seen`: a terminal of the window runs an agent, which the window says.
    pub(crate) fn render_no_agents_note(
        &self,
        agent_seen: bool,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let note = self.agents_note;
        // The window has asked `note_wanted` (the tip not dismissed) before calling.
        if note == AgentsNote::Off || agent_seen {
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
                            this.agents_note = AgentsNote::Offer;
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
                    cx.emit(FirstRunEvent::DismissNoAgentsTip);
                    this.agents_note = AgentsNote::Off;
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
                            this.agents_note = AgentsNote::Off;
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
