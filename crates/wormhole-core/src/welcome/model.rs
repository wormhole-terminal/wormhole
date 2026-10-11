//! The welcome's state: which step is up, where the cursor is in its list, what the app last said,
//! and what each key does. It wraps core's first-run model ([`Onboarding`]: the steps, Back, Continue,
//! Skip, and what the app is asked to do), so the terminal's welcome and the intro window walk the same
//! steps. Pure: keys in, effects out; the program in `run` does the asking and the drawing.
//!
//! Nothing here installs or trusts behind the person's back. An Install goes out only from Return (or a
//! click) on that agent's own row, Trust only from the Trust row, and the cursor starts on Start: the
//! tests walk every key from every step to show that moving through the steps asks for neither.

use std::path::PathBuf;
use std::time::Duration;

use super::keys::Key;
use crate::intro::Handover;
use crate::ipc::control::{AgentsFacts, LookFacts, PlaceFact, TrustFacts, WelcomeOp, WelcomeState};
use crate::onboarding::{Action, Event, Onboarding, Step};

/// What the program does about a key.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// Ask the app (never sent in a read-only tour).
    Ask(WelcomeOp),
    /// The welcome is over.
    End(End),
}

/// How the welcome ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct End {
    /// The folder the shell is to start in, if one was picked.
    pub folder: Option<PathBuf>,
    /// Esc (or Ctrl-C) ended it.
    pub skipped: bool,
}

/// A line of a step's list that the cursor can be on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    Place(usize),
    Theme(usize),
    Agent(usize),
    Trust,
    /// The step's own button: Get started, Continue on a replay, Start.
    Start,
}

/// One welcome, from its first state to its end.
#[derive(Clone, Debug)]
pub struct Welcome {
    flow: Onboarding,
    /// Talking to the app that started it: the choices work. Without, a read-only tour.
    live: bool,
    /// There is a shell to hand the terminal to, so the folder step has a point.
    has_shell: bool,
    facts: WelcomeState,
    /// Where the cursor is in the folder list, and the theme list; `None` for the agents list is
    /// Start, its last line.
    folder_at: usize,
    theme_at: usize,
    agents_at: Option<usize>,
    /// When the current step began, on the program's clock, and whether its entrance was cut short
    /// by a key. Steps seen before come in whole.
    step_since: Duration,
    settled: bool,
    seen: Vec<Step>,
    ended: Option<End>,
    /// A request of ours the app refused or could not answer, until the next one that works.
    note: Option<String>,
}

impl Welcome {
    /// The welcome for the first state the app reported, `live`, or for a tour (`facts` made locally).
    pub fn new(facts: WelcomeState, live: bool, has_shell: bool) -> Self {
        let flow = if facts.first_run {
            Onboarding::new(facts.agents.as_ref().is_none_or(|agents| !agents.found.is_empty()))
        } else {
            Onboarding::replay()
        };
        let theme_at = facts.look.themes.iter().position(|theme| theme.name == facts.look.theme).unwrap_or(0);
        Self {
            seen: vec![flow.step()],
            flow,
            live,
            has_shell,
            facts,
            folder_at: 0,
            theme_at,
            agents_at: None,
            step_since: Duration::ZERO,
            settled: false,
            ended: None,
            note: None,
        }
    }

    // ---------------------------------------------------------------- what the pages read

    pub fn step(&self) -> Step {
        self.flow.step()
    }

    #[cfg(test)]
    pub fn facts(&self) -> &WelcomeState {
        &self.facts
    }

    pub fn live(&self) -> bool {
        self.live
    }

    pub fn handover(&self) -> Handover {
        self.facts.handover
    }

    /// A replay of the welcome: the welcome page alone.
    pub fn is_replay(&self) -> bool {
        !self.facts.first_run
    }

    #[cfg(test)]
    pub fn ended(&self) -> Option<&End> {
        self.ended.as_ref()
    }

    /// The folder picked so far, which the agents page's trust is about.
    pub fn folder(&self) -> Option<&PathBuf> {
        self.flow.folder()
    }

    pub fn places(&self) -> &[PlaceFact] {
        &self.facts.places
    }

    pub fn look(&self) -> &LookFacts {
        &self.facts.look
    }

    pub fn agents(&self) -> Option<&AgentsFacts> {
        self.facts.agents.as_ref()
    }

    pub fn trust(&self) -> Option<&TrustFacts> {
        self.facts.trust.as_ref().filter(|_| self.live)
    }

    /// What went wrong: the app's word about the last Install or Trust, else a request it refused.
    pub fn problem(&self) -> Option<&str> {
        self.facts.problem.as_deref().or(self.note.as_deref())
    }

    /// A request the app refused or could not answer (`None` once one works again).
    pub fn note_failure(&mut self, message: Option<String>) {
        self.note = message;
    }

    /// The lines of the current step's list the cursor can be on, in order.
    pub fn items(&self) -> Vec<Item> {
        match self.flow.step() {
            Step::Welcome => vec![Item::Start],
            // With nothing to list (a tour has no themes) the step still has its way on.
            Step::Folder if self.facts.places.is_empty() => vec![Item::Start],
            Step::Look if self.facts.look.themes.is_empty() => vec![Item::Start],
            Step::Folder => (0..self.facts.places.len()).map(Item::Place).collect(),
            Step::Look => (0..self.facts.look.themes.len()).map(Item::Theme).collect(),
            Step::Agents => {
                let found = self.facts.agents.as_ref().map_or(0, |agents| agents.found.len());
                let mut items: Vec<Item> = (0..found).map(Item::Agent).collect();
                if self.trust().is_some() {
                    items.push(Item::Trust);
                }
                items.push(Item::Start);
                items
            }
            Step::Primer => vec![Item::Start],
        }
    }

    /// Where the cursor is among [`items`](Self::items).
    pub fn cursor(&self) -> usize {
        let items = self.items();
        let last = items.len().saturating_sub(1);
        match self.flow.step() {
            Step::Folder => self.folder_at.min(last),
            Step::Look => self.theme_at.min(last),
            Step::Agents => self.agents_at.map_or(last, |at| at.min(last)),
            Step::Welcome | Step::Primer => 0,
        }
    }

    /// The clock the current page draws by, given the program's clock: how long this step has been
    /// up, or for ever when its entrance should not play (Reduce Motion, a key pressed, a step seen).
    pub fn page_clock(&self, now: Duration) -> Duration {
        if self.settled || self.facts.reduce_motion {
            Duration::MAX
        } else {
            now.saturating_sub(self.step_since)
        }
    }

    // ---------------------------------------------------------------- the app's answers

    /// The app's latest state replaces the facts; what the person is in the middle of (the cursor, the
    /// step) stays. The agents turning up, or not, moves the flow's third step.
    pub fn update(&mut self, mut facts: WelcomeState) {
        // The handover and first-run flag are the app's word for the whole run: fixed at the start.
        facts.handover = self.facts.handover;
        facts.first_run = self.facts.first_run;
        if let Some(agents) = &facts.agents
            && self.flow.step() != Step::Primer
        {
            self.flow.set_agents_found(!agents.found.is_empty());
        }
        self.facts = facts;
    }

    // ---------------------------------------------------------------- keys

    /// What the key at `now` (the program's clock) does.
    pub fn key(&mut self, key: Key, now: Duration) -> Vec<Effect> {
        if self.ended.is_some() {
            return Vec::new();
        }
        // Any key cuts the entrance of the page short.
        self.settled = true;
        let step = self.flow.step();
        match key {
            Key::Esc | Key::Interrupt => self.flow_event(Event::Skip, now),
            Key::Left | Key::Backspace => self.flow_event(Event::Back, now),
            // → goes on where Return has nothing else to mean: not on the agents, where it would
            // press an Install, and not on Start, which ends it.
            Key::Right if matches!(step, Step::Welcome | Step::Folder | Step::Look) => self.activate(now),
            Key::Enter | Key::Char(' ') => self.activate(now),
            Key::Up | Key::Char('k') | Key::BackTab => self.step_cursor(-1),
            Key::Down | Key::Char('j') | Key::Tab => self.step_cursor(1),
            Key::Char(c) if step == Step::Look => self.look_key(c),
            // A click is resolved to a row or a key by the program, which has the hits.
            _ => Vec::new(),
        }
    }

    /// A click on list line `at`: the cursor goes there and it is activated, as Return does (on the
    /// look step it only chooses the theme).
    pub fn click_row(&mut self, at: usize, now: Duration) -> Vec<Effect> {
        let len = self.items().len();
        if at >= len {
            return Vec::new();
        }
        self.settled = true;
        self.set_cursor(at);
        let mut effects = self.look_effect_for_cursor();
        // A click on a theme chooses it, as moving to it does; Return is what goes on.
        if self.flow.step() != Step::Look {
            effects.extend(self.activate(now));
        }
        effects
    }

    fn set_cursor(&mut self, at: usize) {
        match self.flow.step() {
            Step::Folder => self.folder_at = at,
            Step::Look => self.theme_at = at,
            Step::Agents => {
                let last = self.items().len().saturating_sub(1);
                self.agents_at = (at != last).then_some(at);
            }
            Step::Welcome | Step::Primer => {}
        }
    }

    fn step_cursor(&mut self, by: isize) -> Vec<Effect> {
        let len = self.items().len();
        if len == 0 {
            return Vec::new();
        }
        let to = self.cursor().saturating_add_signed(by).min(len - 1);
        let moved = to != self.cursor();
        self.set_cursor(to);
        if moved { self.look_effect_for_cursor() } else { Vec::new() }
    }

    /// Moving the cursor over a theme applies it, so the person sees the whole terminal take it.
    fn look_effect_for_cursor(&mut self) -> Vec<Effect> {
        if self.flow.step() != Step::Look || !self.live {
            return Vec::new();
        }
        let Some(theme) = self.facts.look.themes.get(self.cursor()) else { return Vec::new() };
        if theme.name == self.facts.look.theme {
            return Vec::new();
        }
        let name = theme.name.clone();
        self.facts.look.theme.clone_from(&name);
        vec![Effect::Ask(WelcomeOp::SetLook {
            theme: Some(name),
            primary: None,
            ligatures: None,
            font_step: None,
        })]
    }

    /// `p` the next primary colour, `l` the ligatures, `+` and `-` the font's size.
    fn look_key(&mut self, c: char) -> Vec<Effect> {
        if !self.live {
            return Vec::new();
        }
        let look = &mut self.facts.look;
        let ask = |op: WelcomeOp| vec![Effect::Ask(op)];
        match c {
            'p' => {
                let at = look.primaries.iter().position(|p| p.hex == look.primary).unwrap_or(0);
                let Some(next) = look.primaries.get((at + 1) % look.primaries.len().max(1)) else {
                    return Vec::new();
                };
                look.primary.clone_from(&next.hex);
                ask(WelcomeOp::SetLook {
                    theme: None,
                    primary: Some(next.hex.clone()),
                    ligatures: None,
                    font_step: None,
                })
            }
            'l' => {
                look.ligatures = !look.ligatures;
                ask(WelcomeOp::SetLook {
                    theme: None,
                    primary: None,
                    ligatures: Some(look.ligatures),
                    font_step: None,
                })
            }
            '+' | '=' => {
                look.font_size += 1.0;
                ask(WelcomeOp::SetLook { theme: None, primary: None, ligatures: None, font_step: Some(1) })
            }
            '-' | '_' => {
                look.font_size -= 1.0;
                ask(WelcomeOp::SetLook { theme: None, primary: None, ligatures: None, font_step: Some(-1) })
            }
            _ => Vec::new(),
        }
    }

    /// Return on the line the cursor is on.
    fn activate(&mut self, now: Duration) -> Vec<Effect> {
        let items = self.items();
        let Some(item) = items.get(self.cursor()).cloned() else { return Vec::new() };
        match item {
            Item::Place(at) => {
                let Some(place) = self.facts.places.get(at) else { return Vec::new() };
                let folder = PathBuf::from(&place.path);
                self.flow.apply(Event::PickFolder(folder));
                self.flow_event(Event::Continue, now)
            }
            Item::Theme(_) => self.flow_event(Event::Continue, now),
            Item::Agent(at) => {
                let agent = self.facts.agents.as_ref().and_then(|agents| agents.found.get(at));
                match agent {
                    Some(agent) if agent.hooks.installable() => {
                        self.flow_event(Event::Install(agent.id), now)
                    }
                    _ => Vec::new(),
                }
            }
            Item::Trust => match self.trust().map(|trust| PathBuf::from(&trust.folder)) {
                Some(folder) => self.flow_event(Event::Trust(folder), now),
                None => Vec::new(),
            },
            Item::Start => self.flow_event(Event::Start, now),
        }
    }

    /// Run `event` through the first-run model and turn what it asks into effects, skipping the folder
    /// step when there is no shell to start in it.
    fn flow_event(&mut self, event: Event, now: Duration) -> Vec<Effect> {
        let before = self.flow.step();
        let action = self.flow.apply(event.clone());
        if self.flow.step() == Step::Folder && !self.has_shell {
            // Nothing to hand over: the folder step is skipped, in whichever direction it was entered.
            let onward = if before == Step::Welcome { Event::Continue } else { Event::Back };
            self.flow.apply(onward);
        }
        let step = self.flow.step();
        if step != before {
            self.step_since = now;
            self.settled = self.seen.contains(&step);
            if !self.seen.contains(&step) {
                self.seen.push(step);
            }
        }
        match action {
            Action::None => Vec::new(),
            Action::Skip => self.end(None, true),
            Action::Finish { folder } => self.end(folder, false),
            Action::InstallHooks(agent) => self.ask(WelcomeOp::InstallHooks { agent }),
            Action::Trust(folder) => {
                self.ask(WelcomeOp::Trust { folder: folder.to_string_lossy().into_owned() })
            }
        }
    }

    fn ask(&self, op: WelcomeOp) -> Vec<Effect> {
        if self.live { vec![Effect::Ask(op)] } else { Vec::new() }
    }

    fn end(&mut self, folder: Option<PathBuf>, skipped: bool) -> Vec<Effect> {
        let end = End { folder: folder.filter(|_| self.has_shell), skipped };
        self.ended = Some(end.clone());
        vec![Effect::End(end)]
    }
}
