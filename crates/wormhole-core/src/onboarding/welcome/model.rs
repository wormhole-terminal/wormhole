//! The welcome's state: which step is up, where the cursor is in its list, what the app last said,
//! and what each key does. It wraps the terminal welcome's own steps ([`Flow`]: Back, Continue, Skip),
//! so the intro window and the terminal walk different sequences. Pure: keys in, effects out; the
//! program in `run` does the asking and the drawing.
//!
//! Nothing here installs or changes what reaches you behind the person's back. An Install goes out
//! only from Return (or a click) on that agent's own row, a notification choice only from a press on
//! its own row, and the cursor starts on the step's button: the tests walk every key from every step
//! to show that moving through the steps asks for neither.

use std::time::Duration;

use super::flow::{Action, Event, Flow, Step};
use super::keys::Key;
use super::offer::ImportOffer;
use crate::config::NotifySound;
use crate::ipc::control::{
    AgentsFacts, ImportDone, LookFacts, NotifyFacts, PermissionFact, WelcomeOp, WelcomeState,
};
use crate::onboarding::intro::Handover;

/// What the program does about a key.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Effect {
    /// Ask the app (never sent in a read-only tour).
    Ask(WelcomeOp),
    /// The welcome is over.
    End(End),
}

/// How the welcome ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct End {
    /// Esc (or Ctrl-C) ended it.
    pub skipped: bool,
}

/// A row of the notifications step the cursor can be on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NotifyRow {
    /// Ask macOS to allow banners, offered only while it has not been asked.
    Allow,
    NeedsYou,
    AgentDone,
    Sound,
}

/// A line of a step's list that the cursor can be on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Item {
    Theme(usize),
    Notify(NotifyRow),
    Agent(usize),
    /// The step's own button: Get started, Continue, Start.
    Start,
}

/// Where the offer to bring another terminal's settings over stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Import {
    /// Found, not asked for yet.
    Offered(ImportOffer),
    /// Asked for; the app has not answered.
    Asked(ImportOffer),
    /// Brought over: what the app says it did.
    Done(ImportOffer, ImportDone),
}

/// One welcome, from its first state to its end.
#[derive(Clone, Debug)]
pub(super) struct Welcome {
    flow: Flow,
    /// Talking to the app that started it: the choices work. Without, a read-only tour.
    live: bool,
    facts: WelcomeState,
    /// Where the cursor is in the theme list; `None` is the button, the last line, for the notification
    /// rows and the agents list.
    theme_at: usize,
    notify_at: Option<NotifyRow>,
    agents_at: Option<usize>,
    /// When the current step began, on the program's clock, and whether its entrance was cut short
    /// by a key. Steps seen before come in whole.
    step_since: Duration,
    settled: bool,
    seen: Vec<Step>,
    ended: Option<End>,
    /// The terminal found with settings to bring over, `None` when there is none or it is not (yet)
    /// known.
    import: Option<Import>,
    /// The theme list changed under the cursor by an import: the next state from the app puts the
    /// cursor on the theme now in use.
    resync_theme: bool,
    /// A request of ours the app refused or could not answer, until the next one that works.
    note: Option<String>,
}

impl Welcome {
    /// The welcome for the first state the app reported, `live`, or for a tour (`facts` made locally).
    pub(super) fn new(facts: WelcomeState, live: bool) -> Self {
        let flow = if facts.first_run { Flow::new() } else { Flow::replay() };
        let theme_at = facts.look.themes.iter().position(|theme| theme.name == facts.look.theme).unwrap_or(0);
        Self {
            seen: vec![flow.step()],
            flow,
            live,
            facts,
            theme_at,
            notify_at: None,
            agents_at: None,
            step_since: Duration::ZERO,
            settled: false,
            ended: None,
            import: None,
            resync_theme: false,
            note: None,
        }
    }

    // ---------------------------------------------------------------- the import offer

    /// The terminal the machine has settings of, found by the program (not the app): offered on the look
    /// page. Ignored once the person has begun to use an earlier offer.
    pub(super) fn offer(&mut self, offer: Option<ImportOffer>) {
        if self.import.is_none() {
            self.import = offer.map(Import::Offered);
        }
    }

    pub(super) fn import(&self) -> Option<Import> {
        self.import
    }

    /// The app answered the import: what it brought over. The theme list may have a new entry, so the
    /// cursor follows the theme in use as soon as the app next says which it is.
    pub(super) fn imported(&mut self, done: ImportDone) {
        if let Some(Import::Asked(offer)) = self.import {
            self.import = Some(Import::Done(offer, done));
            self.resync_theme = true;
            self.note = None;
        }
    }

    /// The app refused or failed the import: the offer stands to be tried again, and the page says why.
    pub(super) fn import_failed(&mut self, why: String) {
        if let Some(Import::Asked(offer)) = self.import {
            self.import = Some(Import::Offered(offer));
        }
        self.note = Some(why);
    }

    // ---------------------------------------------------------------- what the pages read

    pub(super) fn step(&self) -> Step {
        self.flow.step()
    }

    #[cfg(test)]
    pub(super) fn facts(&self) -> &WelcomeState {
        &self.facts
    }

    pub(super) fn live(&self) -> bool {
        self.live
    }

    /// The hand-over the app named; one this program doesn't know plays as the default.
    pub(super) fn handover(&self) -> Handover {
        Handover::parse(&self.facts.handover).unwrap_or_default()
    }

    /// A replay of the welcome: the welcome page alone.
    pub(super) fn is_replay(&self) -> bool {
        self.flow.is_replay()
    }

    #[cfg(test)]
    pub(super) fn ended(&self) -> Option<&End> {
        self.ended.as_ref()
    }

    /// Whether the app is to show its example band in the sidebar: while the sidebar step is up, in a
    /// welcome that talks to the app and has not ended.
    pub(super) fn wants_demo(&self) -> bool {
        self.live && self.ended.is_none() && self.flow.step() == Step::Sidebar
    }

    pub(super) fn look(&self) -> &LookFacts {
        &self.facts.look
    }

    /// What reaches the person now, when there is an app to ask.
    pub(super) fn notify(&self) -> Option<&NotifyFacts> {
        self.facts.notify.as_ref().filter(|_| self.live)
    }

    pub(super) fn agents(&self) -> Option<&AgentsFacts> {
        self.facts.agents.as_ref()
    }

    /// What went wrong: the app's word about the last Install, else a request it refused.
    pub(super) fn problem(&self) -> Option<&str> {
        self.facts.problem.as_deref().or(self.note.as_deref())
    }

    /// A request the app refused or could not answer (`None` once one works again).
    pub(super) fn note_failure(&mut self, message: Option<String>) {
        self.note = message;
    }

    /// The lines of the current step's list the cursor can be on, in order.
    pub(super) fn items(&self) -> Vec<Item> {
        match self.flow.step() {
            Step::Welcome | Step::Extras => vec![Item::Start],
            // With nothing to list (a tour has no themes) the step still has its way on.
            Step::Look if self.facts.look.themes.is_empty() => vec![Item::Start],
            Step::Look => (0..self.facts.look.themes.len()).map(Item::Theme).collect(),
            Step::Notify => {
                let mut items = Vec::new();
                if let Some(notify) = self.notify() {
                    items.extend(
                        [NotifyRow::NeedsYou, NotifyRow::AgentDone, NotifyRow::Sound].map(Item::Notify),
                    );
                    // Under the choices, while macOS has not been asked: the one press that asks.
                    if notify.permission == PermissionFact::NotAsked {
                        items.push(Item::Notify(NotifyRow::Allow));
                    }
                }
                items.push(Item::Start);
                items
            }
            Step::Sidebar => {
                let found = self.facts.agents.as_ref().map_or(0, |agents| agents.found.len());
                let mut items: Vec<Item> = (0..found).map(Item::Agent).collect();
                items.push(Item::Start);
                items
            }
        }
    }

    /// Where the cursor is among [`items`](Self::items).
    pub(super) fn cursor(&self) -> usize {
        let items = self.items();
        let last = items.len().saturating_sub(1);
        match self.flow.step() {
            Step::Look => self.theme_at.min(last),
            Step::Notify => self
                .notify_at
                .and_then(|row| items.iter().position(|item| *item == Item::Notify(row)))
                .unwrap_or(last),
            Step::Sidebar => self.agents_at.map_or(last, |at| at.min(last)),
            Step::Welcome | Step::Extras => 0,
        }
    }

    /// The clock the current page draws by, given the program's clock: how long this step has been
    /// up, or for ever when its entrance should not play (Reduce Motion, a key pressed, a step seen).
    pub(super) fn page_clock(&self, now: Duration) -> Duration {
        if self.settled || self.facts.reduce_motion {
            Duration::MAX
        } else {
            now.saturating_sub(self.step_since)
        }
    }

    // ---------------------------------------------------------------- the app's answers

    /// The app's latest state replaces the facts; what the person is in the middle of (the cursor, the
    /// step) stays. `stale` says the state was asked for before the last choice was sent, so it may
    /// still show what was there before: the choices on the page are kept as the person made them
    /// (the next state corrects them if the app didn't take them).
    pub(super) fn update(&mut self, mut facts: WelcomeState, stale: bool) {
        // The handover and first-run flag are the app's word for the whole run: fixed at the start.
        facts.handover = self.facts.handover.clone();
        facts.first_run = self.facts.first_run;
        if stale {
            let LookFacts { themes, primaries, .. } =
                std::mem::replace(&mut facts.look, self.facts.look.clone());
            facts.look.themes = themes;
            facts.look.primaries = primaries;
            facts.notify = match (self.facts.notify, facts.notify) {
                (Some(mine), Some(theirs)) => Some(NotifyFacts { permission: theirs.permission, ..mine }),
                (_, theirs) => theirs,
            };
        }
        self.facts = facts;
        if self.resync_theme && !stale {
            self.resync_theme = false;
            self.theme_at = self
                .facts
                .look
                .themes
                .iter()
                .position(|theme| theme.name == self.facts.look.theme)
                .unwrap_or(self.theme_at);
        }
    }

    // ---------------------------------------------------------------- keys

    /// What the key at `now` (the program's clock) does.
    pub(super) fn key(&mut self, key: Key, now: Duration) -> Vec<Effect> {
        if self.ended.is_some() {
            return Vec::new();
        }
        // Any key cuts the entrance of the page short.
        self.settled = true;
        let step = self.flow.step();
        match key {
            Key::Esc | Key::Interrupt => self.flow_event(Event::Skip, now),
            Key::Left | Key::Backspace => self.flow_event(Event::Back, now),
            // → goes on where Return might press a row (an Install, a choice) and so is no way to
            // go on; it never presses one. The last step has nothing after it but the end.
            Key::Right if step != Step::Extras => self.flow_event(Event::Continue, now),
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
    pub(super) fn click_row(&mut self, at: usize, now: Duration) -> Vec<Effect> {
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
        let items = self.items();
        let last = items.len().saturating_sub(1);
        match (self.flow.step(), items.get(at)) {
            (Step::Look, _) => self.theme_at = at,
            (Step::Notify, Some(Item::Notify(row))) => self.notify_at = Some(*row),
            (Step::Notify, _) => self.notify_at = None,
            (Step::Sidebar, _) => self.agents_at = (at != last).then_some(at),
            (Step::Welcome | Step::Extras, _) => {}
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
            'i' => self.ask_import(),
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

    /// `i`: bring over the settings of the terminal found, once. Only the look page offers it, and only
    /// to a welcome that talks to the app.
    fn ask_import(&mut self) -> Vec<Effect> {
        let Some(Import::Offered(offer)) = self.import else { return Vec::new() };
        self.import = Some(Import::Asked(offer));
        vec![Effect::Ask(WelcomeOp::Import { source: offer.source.key().to_string() })]
    }

    /// Return on the line the cursor is on.
    fn activate(&mut self, now: Duration) -> Vec<Effect> {
        let items = self.items();
        let Some(item) = items.get(self.cursor()).cloned() else { return Vec::new() };
        match item {
            Item::Theme(_) | Item::Start => self.flow_event(Event::Continue, now),
            Item::Notify(row) => self.press_notify(row),
            Item::Agent(at) => {
                let agent = self.facts.agents.as_ref().and_then(|agents| agents.found.get(at));
                match agent {
                    Some(agent) if agent.hooks.installable() => {
                        self.ask(WelcomeOp::InstallHooks { agent: agent.id })
                    }
                    _ => Vec::new(),
                }
            }
        }
    }

    /// A press on a notification row: the choice changes here at once, and the app is told. A switch
    /// flips, the sound goes round, Allow asks macOS; the app asks macOS itself when the press turns a
    /// banner on and it has not been asked.
    fn press_notify(&mut self, row: NotifyRow) -> Vec<Effect> {
        let Some(notify) = self.facts.notify.as_mut().filter(|_| self.live) else { return Vec::new() };
        let (mut needs_you, mut agent_done, mut sound, mut allow) = (None, None, None, false);
        match row {
            NotifyRow::Allow => allow = true,
            NotifyRow::NeedsYou => {
                notify.needs_you = !notify.needs_you;
                needs_you = Some(notify.needs_you);
            }
            NotifyRow::AgentDone => {
                notify.agent_done = !notify.agent_done;
                agent_done = Some(notify.agent_done);
            }
            NotifyRow::Sound => {
                notify.sound = next_sound(notify.sound);
                sound = Some(notify.sound);
            }
        }
        vec![Effect::Ask(WelcomeOp::SetNotify { needs_you, agent_done, sound, allow })]
    }

    /// Run `event` through the flow and turn what it asks into effects.
    fn flow_event(&mut self, event: Event, now: Duration) -> Vec<Effect> {
        let before = self.flow.step();
        let action = self.flow.apply(event);
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
            Action::End { skipped } => self.end(skipped),
        }
    }

    fn ask(&self, op: WelcomeOp) -> Vec<Effect> {
        if self.live { vec![Effect::Ask(op)] } else { Vec::new() }
    }

    fn end(&mut self, skipped: bool) -> Vec<Effect> {
        let end = End { skipped };
        self.ended = Some(end);
        vec![Effect::End(end)]
    }
}

/// The sound choices in the order Settings lists them, round again after the last.
fn next_sound(sound: NotifySound) -> NotifySound {
    match sound {
        NotifySound::NeedsYou => NotifySound::All,
        NotifySound::All => NotifySound::None,
        NotifySound::None => NotifySound::NeedsYou,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sound_goes_round_in_settings_order() {
        let mut sound = NotifySound::NeedsYou;
        let mut seen = Vec::new();
        for _ in 0..4 {
            seen.push(sound);
            sound = next_sound(sound);
        }
        assert_eq!(seen, [NotifySound::NeedsYou, NotifySound::All, NotifySound::None, NotifySound::NeedsYou]);
    }
}
