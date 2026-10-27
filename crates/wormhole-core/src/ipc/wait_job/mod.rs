//! A `wait` or a `run` the stream thread carries out: it blocks until the condition holds, the time is up,
//! the terminal or the app goes, or the caller may no longer read, and the reply is its one line.
//!
//! The app resolves the target and the policy on its thread in a moment and hands over a [`WaitJob`]:
//! the subscription it made (with the filter of the events the conditions need, in the same step as the
//! terminal's state was read, so no event between the two is missed), a weak handle on the terminal's
//! screen, and the plan. Nothing here runs on the UI thread, the PTY's reader or the socket thread that
//! serves the hooks. The conditions are evaluated on events only: a state or a command arriving, a
//! wake-up that the terminal printed (at most every [`LOOK_EVERY`](super::bus::LOOK_EVERY), however much it prints), or a timer
//! the plan itself asks for (`--stable`, the timeout). Between them the thread sleeps.

mod run;
mod screen;
#[cfg(test)]
mod tests;

use std::time::{Duration, Instant};

use serde::Serialize;
use wormhole_term::WeakScreenReader;

use super::bus::{Next, Subscription};
use super::control::event::{BlockPhase, Body, CloseReason, Event};
use super::control::wait::{Met, Waited};
use super::control::{Error, ErrorCode, Reason, Reply, TerminalState, WaitPlan, matched_text, reads_off};
use screen::{Hit, Look};

/// A wait or a run to carry out.
pub struct WaitJob {
    core: Core,
    what: What,
}

impl std::fmt::Debug for WaitJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WaitJob").field("terminal", &self.core.terminal).finish_non_exhaustive()
    }
}

/// What both kinds have: where they listen and what they look at.
struct Core {
    subscription: Subscription,
    reader: WeakScreenReader,
    terminal: i64,
}

impl Core {
    /// The terminal's screen, while the terminal is there.
    fn screen(&self) -> Result<wormhole_term::ScreenReader, Error> {
        self.reader.upgrade().ok_or_else(|| gone(self.terminal, "was closed"))
    }
}

enum What {
    Wait(WaitPlan),
    Run(run::RunWait),
}

impl WaitJob {
    /// A `wait` on `terminal` for `plan`. `subscription` must have been made for it (`Bus::wait`).
    pub fn wait(subscription: Subscription, reader: WeakScreenReader, terminal: i64, plan: WaitPlan) -> Self {
        Self { core: Core { subscription, reader, terminal }, what: What::Wait(plan) }
    }

    /// A `run` in `terminal`, whose command was just typed at a prompt after the block `baseline`
    /// (none: the first); gives up after `timeout`.
    pub fn run(
        subscription: Subscription,
        reader: WeakScreenReader,
        terminal: i64,
        baseline: Option<u64>,
        timeout: Duration,
    ) -> Self {
        let plan = run::RunWait { baseline, timeout };
        Self { core: Core { subscription, reader, terminal }, what: What::Run(plan) }
    }

    /// Wait it out and make the reply. Blocks, for as long as the plan's timeout at most.
    pub fn finish(self) -> Reply {
        let started = Instant::now();
        let Self { core, what } = self;
        match what {
            What::Wait(plan) => reply(wait(&core, &plan, started)),
            What::Run(plan) => reply(run::run(&core, &plan, started)),
        }
    }
}

fn reply<T: Serialize>(result: Result<T, Error>) -> Reply {
    match result {
        Ok(done) => Reply::ok(done),
        Err(error) => error.into(),
    }
}

/// What ended a stream's wait, as the reply says it: the app quitting, the setting going off.
fn closed(reason: CloseReason) -> Error {
    match reason {
        CloseReason::NotAllowed => reads_off(),
        CloseReason::Quitting => Error::new(ErrorCode::Failed, "Wormhole is quitting"),
        _ => Error::new(ErrorCode::Failed, "the wait was ended"),
    }
}

/// The terminal went: closed, or its shell ended.
fn gone(terminal: i64, why: &str) -> Error {
    Error::new(ErrorCode::NotFound, format!("terminal {terminal} {why}")).because(Reason::Exited)
}

/// A wait that ran out of time (`took` is the timeout it was given), and what was the case then.
fn timed_out(took: Duration, what: String) -> Error {
    Error::new(ErrorCode::Failed, format!("timed out after {} s: {what}", took.as_secs().max(1)))
        .because(Reason::Timeout)
}

/// What an event says to a `wait` with `plan`: the condition it meets, or that the terminal is gone.
fn judge(plan: &WaitPlan, terminal: i64, event: &Event) -> Option<Result<Hit, Error>> {
    match &event.body {
        Body::Status { status, .. } if plan.statuses.contains(status) => Some(Ok(Hit::Status(*status))),
        Body::Block { phase: BlockPhase::Finished, block, exit_code, .. } if plan.block_done => {
            Some(Ok(Hit::BlockDone { block: *block, exit_code: *exit_code }))
        }
        Body::TerminalClosed => Some(Err(gone(terminal, "was closed"))),
        // Its shell ended: a state nobody asked to wait for will not come again.
        Body::Status { status: TerminalState::Exited, .. }
            if !plan.statuses.contains(&TerminalState::Exited) =>
        {
            Some(Err(gone(terminal, "has ended")))
        }
        _ => None,
    }
}

fn wait(core: &Core, plan: &WaitPlan, started: Instant) -> Result<Waited, Error> {
    let deadline = started + plan.timeout;
    let mut look = Look::new(plan, started);
    if let Some(hit) = look.first(core)? {
        return Ok(waited(core, hit, started));
    }
    loop {
        let now = Instant::now();
        if now >= deadline {
            return Err(timed_out(plan.timeout, format!("still waiting for {}", describe(plan))));
        }
        let wake = look.due().map_or(deadline, |due| due.min(deadline));
        match core.subscription.next(wake.saturating_duration_since(now)) {
            Next::Event(published) => {
                if let Some(done) = judge(plan, core.terminal, &published.event) {
                    return done.map(|hit| waited(core, hit, started));
                }
            }
            Next::Changed => look.changed(),
            Next::Lagged(_) | Next::Idle => {}
            Next::Closed(reason) => return Err(closed(reason)),
        }
        if let Some(hit) = look.step(core, Instant::now())? {
            return Ok(waited(core, hit, started));
        }
    }
}

/// The conditions, for the words of a timeout.
fn describe(plan: &WaitPlan) -> String {
    let mut parts = Vec::new();
    if !plan.statuses.is_empty() {
        let words: Vec<&str> = plan.statuses.iter().map(|s| s.word()).collect();
        parts.push(format!("the state to be {}", words.join(" or ")));
    }
    if plan.text.is_some() {
        parts.push("the text to appear".into());
    }
    if plan.gone.is_some() {
        parts.push("the text to go".into());
    }
    if plan.stable.is_some() {
        parts.push("the output to go quiet".into());
    }
    if plan.block_done {
        parts.push("a command to finish".into());
    }
    parts.join(" or ")
}

fn waited(core: &Core, hit: Hit, started: Instant) -> Waited {
    let mut waited = Waited {
        terminal: core.terminal,
        met: Met::Status,
        after_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        status: None,
        matched: None,
        block: None,
        exit_code: None,
        revision: core.reader.upgrade().map(|reader| reader.revision()),
    };
    match hit {
        Hit::Status(status) => (waited.met, waited.status) = (Met::Status, Some(status)),
        Hit::Text(found) => (waited.met, waited.matched) = (Met::Text, Some(matched_text(&found))),
        Hit::Gone => waited.met = Met::Gone,
        Hit::Stable => waited.met = Met::Stable,
        Hit::BlockDone { block, exit_code } => {
            (waited.met, waited.block, waited.exit_code) = (Met::BlockDone, Some(block), exit_code);
        }
    }
    waited
}
