//! The program's brain, without its hands: one `tick` takes what happened since the last (the time, the
//! size, the bytes typed, the app's answers) and says what to do about it (the bytes to draw, the
//! requests to send, whether it is over). `run` does the reading, writing and asking; this decides, so
//! the whole welcome runs in tests from a script of ticks.
//!
//! The clock: the program draws nothing until the start animation says the text may begin (`wait_ms` in
//! the app's first answer, counted from when it arrived). A key while it waits ends the wait; the app can
//! also shorten it in a later answer (the animation was skipped).

use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;

use super::ansi::Grid;
use super::facts;
use super::keys::{ESC_WAIT, Key, Parser};
use super::model::{Effect, End, Welcome};
use super::render::{self, Hit, Size, Target};
use crate::ipc::control::{WelcomeOp, WelcomeState};

/// How often the app is asked for its state: often enough that an Install or a permission answer shows
/// within a blink, and the end of the animation's wait is noticed.
const POLL: Duration = Duration::from_millis(250);

/// The app's answer to something we sent.
#[derive(Debug)]
pub(super) struct Answer {
    /// The op's name (`WelcomeOp::name`).
    pub op: &'static str,
    /// The reply's result, or what went wrong (the words of the refusal, or why no reply came).
    pub result: Result<Option<Value>, String>,
}

/// What to do after a tick.
#[derive(Debug, Default)]
pub(super) struct Tick {
    /// Bytes for the terminal.
    pub draw: Vec<u8>,
    /// Requests to send, in order.
    pub ask: Vec<WelcomeOp>,
    /// The welcome is over: send the `Finish` that is in `ask`, then hand the terminal over.
    pub end: Option<End>,
}

pub(super) struct Driver {
    /// There is a way to reach the app (a token, a tab and a socket).
    linked: bool,
    home: PathBuf,
    model: Option<Welcome>,
    parser: Parser,
    /// When the text starts, on the driver's clock (the time given to `tick`).
    text_start: Option<Duration>,
    polled_at: Option<Duration>,
    state_in_flight: bool,
    /// A choice was sent while a state was on its way: that state was asked for before the choice, so
    /// it may not show it yet.
    stale_state: bool,
    /// What the app was last told about the example band (it starts without one); `None` once a
    /// request about it failed, which is asked again, and when that was.
    demo_sent: Option<bool>,
    demo_at: Option<Duration>,
    held_since: Option<Duration>,
    previous: Option<Grid>,
    hits: Vec<Hit>,
    ended: bool,
}

impl Driver {
    pub(super) fn new(linked: bool, home: PathBuf) -> Self {
        Self {
            linked,
            home,
            model: None,
            parser: Parser::default(),
            text_start: None,
            polled_at: None,
            state_in_flight: false,
            stale_state: false,
            demo_sent: Some(false),
            demo_at: None,
            held_since: None,
            previous: None,
            hits: Vec::new(),
            ended: false,
        }
    }

    /// The model, once there is one.
    #[cfg(test)]
    pub(super) fn model(&self) -> Option<&Welcome> {
        self.model.as_ref()
    }

    /// Whether the text has started (the animation's wait is over).
    pub(super) fn started(&self, now: Duration) -> bool {
        self.model.is_some() && self.text_start.is_some_and(|start| now >= start)
    }

    /// One step of the program at `now`.
    pub(super) fn tick(&mut self, now: Duration, size: Size, input: &[u8], answers: Vec<Answer>) -> Tick {
        let mut tick = Tick::default();
        if self.ended {
            return tick;
        }
        for answer in answers {
            self.answer(now, answer);
        }
        // Nobody to ask: the tour begins at once.
        if self.model.is_none() && !self.linked {
            self.model = Some(Welcome::new(facts::tour_state(&self.home), false));
            self.text_start = Some(now);
        }
        self.ask_state(now, &mut tick);

        let keys = self.read_keys(now, input);
        let waiting = self.model.is_some() && !self.started(now);
        if waiting && !keys.is_empty() {
            // A key ends the wait; it isn't acted on, it only says "now".
            self.text_start = Some(now);
        } else if !waiting {
            let clock = self.clock(now);
            for key in keys {
                if let Some(effects) = self.handle(key, clock) {
                    self.run_effects(effects, &mut tick);
                }
            }
        }
        self.sync_demo(now, &mut tick);

        if self.started(now) && !self.ended {
            self.draw(now, size, &mut tick);
        }
        tick
    }

    fn clock(&self, now: Duration) -> Duration {
        self.text_start.map_or(Duration::ZERO, |start| now.saturating_sub(start))
    }

    fn read_keys(&mut self, now: Duration, input: &[u8]) -> Vec<Key> {
        let mut keys = self.parser.feed(input);
        if !input.is_empty() || !self.parser.holding() {
            self.held_since = self.parser.holding().then_some(now);
        } else if self.held_since.is_some_and(|since| now.saturating_sub(since) >= ESC_WAIT) {
            keys.extend(self.parser.flush());
            self.held_since = None;
        }
        keys
    }

    fn handle(&mut self, key: Key, clock: Duration) -> Option<Vec<Effect>> {
        let model = self.model.as_mut()?;
        Some(match key {
            Key::Click { row, col } => {
                let hit = self.hits.iter().find(|hit| hit.contains(row, col)).copied();
                match hit.map(|hit| hit.target) {
                    Some(Target::Row(at)) => model.click_row(at, clock),
                    Some(Target::Key(press)) => model.key(press, clock),
                    None => Vec::new(),
                }
            }
            key => model.key(key, clock),
        })
    }

    fn run_effects(&mut self, effects: Vec<Effect>, tick: &mut Tick) {
        for effect in effects {
            match effect {
                Effect::Ask(op) => {
                    if self.state_in_flight
                        && matches!(op, WelcomeOp::SetLook { .. } | WelcomeOp::SetNotify { .. })
                    {
                        self.stale_state = true;
                    }
                    tick.ask.push(op);
                }
                Effect::End(end) => {
                    if self.model.as_ref().is_some_and(Welcome::live) {
                        tick.ask.push(WelcomeOp::Finish { skipped: end.skipped });
                    }
                    self.ended = true;
                    tick.end = Some(end);
                }
            }
        }
    }

    /// Ask the app for its state: at the start, and then every [`POLL`].
    fn ask_state(&mut self, now: Duration, tick: &mut Tick) {
        if !self.linked || self.state_in_flight {
            return;
        }
        let due = self.polled_at.is_none_or(|at| now.saturating_sub(at) >= POLL);
        if due {
            tick.ask.push(WelcomeOp::State);
            self.polled_at = Some(now);
            self.state_in_flight = true;
        }
    }

    /// Tell the app when the example band should go up or come down: the moment the sidebar step is
    /// entered or left. (The end needs no word: its `Finish` takes the band away.) A request that
    /// failed is made again, not every frame but every so often, so the band is never left behind.
    fn sync_demo(&mut self, now: Duration, tick: &mut Tick) {
        if !self.linked || self.ended {
            return;
        }
        let wanted = self.model.as_ref().is_some_and(Welcome::wants_demo);
        if self.demo_sent == Some(wanted) {
            return;
        }
        if self.demo_sent.is_none() && self.demo_at.is_some_and(|at| now.saturating_sub(at) < POLL) {
            return;
        }
        tick.ask.push(WelcomeOp::Demo { show: wanted });
        self.demo_sent = Some(wanted);
        self.demo_at = Some(now);
    }

    fn answer(&mut self, now: Duration, answer: Answer) {
        if answer.op == "state" {
            self.state_in_flight = false;
        }
        match (answer.op, answer.result) {
            ("state", Ok(Some(value))) => match serde_json::from_value::<WelcomeState>(value) {
                Ok(state) => self.take_state(now, state),
                Err(err) => self.state_failed(&format!("unreadable state: {err}")),
            },
            ("state", Ok(None)) => self.state_failed("no state"),
            ("state", Err(why)) => self.state_failed(&why),
            (op, result) => {
                if op == "demo" && result.is_err() {
                    self.demo_sent = None;
                }
                if let Some(model) = &mut self.model {
                    model.note_failure(result.err());
                }
            }
        }
    }

    fn take_state(&mut self, now: Duration, state: WelcomeState) {
        let start = now + Duration::from_millis(state.wait_ms);
        let stale = std::mem::take(&mut self.stale_state);
        match &mut self.model {
            Some(model) => {
                model.update(state, stale);
                // The app may shorten the wait (the animation was skipped), never lengthen it.
                self.text_start = Some(self.text_start.map_or(start, |was| was.min(start)));
            }
            None => {
                self.model = Some(Welcome::new(state, true));
                self.text_start = Some(start);
            }
        }
    }

    /// The app can't be reached: before there is a model, the welcome becomes the read-only tour; after,
    /// it goes on with what it has.
    fn state_failed(&mut self, why: &str) {
        tracing::debug!("welcome: no state from the app: {why}");
        if self.model.is_none() {
            self.linked = false;
        }
    }

    fn draw(&mut self, now: Duration, size: Size, tick: &mut Tick) {
        let Some(model) = &self.model else { return };
        let frame = render::render(model, size, self.clock(now));
        tick.draw = frame.grid.encode(self.previous.as_ref());
        self.previous = Some(frame.grid);
        self.hits = frame.hits;
    }
}
