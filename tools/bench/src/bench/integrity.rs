//! Did the terminal really parse everything it was sent?
//!
//! A throughput number is the time until the terminal answers the cursor report that follows the
//! payload. A terminal that read all the bytes and threw most of them away (a ring buffer overwritten
//! each tick, a queue that sheds load) still answers that question, and fast: the question is in the
//! newest bytes, which it kept. So the answer at the end says nothing about the bytes before it.
//!
//! The check asks along the way. At *checkpoints* spread through the payload, where no sequence,
//! character or grapheme is open, the writer adds `ESC [ 5 n` (DSR 5, "are you all right?"; the answer is
//! `ESC [ 0 n`). At the end the answers are counted, and so is how many of them had arrived when the
//! cursor report did. A terminal that dropped a stretch containing a checkpoint never answers it, and the
//! run is reported as dropped input instead of a speed; one whose answers come after the cursor report
//! (it answers from another thread, say) is not called dropped, but its number cannot be verified.
//!
//! What it can and cannot prove is in the README ("Is the number real?"); the short version: it
//! catches a terminal that loses input, by design or under load, with a certainty that grows with the
//! length of what was lost; it is not a proof against a terminal written to pass it.
//!
//! Modules: `scan` (where it is safe to ask), `plan` (the checkpoints and how the writes carry them),
//! `seed` (where their places are drawn from), `probe` (is the terminal a fair witness), `outcome` (the
//! verdict and its place in the results).

mod outcome;
mod plan;
mod probe;
mod scan;
mod seed;

use std::io::{self, Write};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::workloads::Payload;
use crate::term::Input;
pub use outcome::{Check, Level, Why, summary};
pub use plan::{DEFAULT_EVERY, Plan, chunks, write_chunk};
pub use seed::{MAX_SEED, Slot};

/// Fewer checkpoints than this and a run is not called verified, only unverified ("too few"): a
/// handful of answers is no evidence. A run that lost one is still called dropped.
pub const ENOUGH: u64 = 16;

/// The bytes a checkpoint adds to what is sent, and that its answer adds to what is received.
pub const QUERY_BYTES: u64 = plan::QUERY.len() as u64;

/// How long a terminal gets, after the last byte has been answered, to send answers that are still on
/// their way. A terminal that answers in order has none outstanding; one that answers from another
/// thread may be a little behind. Paid only when an answer is missing.
const SETTLE: Duration = Duration::from_secs(3);
/// How long a run that missed answers is given in a second look, and in a repeat of the run: a terminal that
/// answers slowly, or limits its answers under load, must not be accused of dropping input for that.
const PATIENCE: Duration = Duration::from_secs(10);

/// Whether the terminal is a fair witness (`probe`).
#[derive(Clone, Debug, PartialEq, Eq)]
enum Fitness {
    /// Not asked yet.
    Unknown,
    Fit,
    Unfit(Why),
}

/// Plans, sends and counts the checkpoints of one `bench` invocation.
pub struct Verifier {
    /// The distance between checkpoints; `None`: the check is off.
    every: Option<usize>,
    fit: Fitness,
    settle: Duration,
    patience: Duration,
    /// The invocation's seed; each plan's is derived from it and the run it is for (`seed`).
    seed: u64,
}

impl Verifier {
    /// `seed` replays the checkpoints of an earlier invocation (`--verify-seed`); by default it comes
    /// from the clock, so that no run can be told by its checkpoints from another.
    pub fn new(every: Option<usize>, seed: Option<u64>) -> Self {
        let clock = || {
            let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64);
            seed::from_clock(nanos)
        };
        let seed = seed.map_or_else(clock, seed::given);
        Self { every, fit: Fitness::Unknown, settle: SETTLE, patience: PATIENCE, seed }
    }

    #[cfg(test)]
    pub(crate) fn for_test(every: usize) -> Self {
        Self {
            every: Some(every),
            fit: Fitness::Unknown,
            settle: Duration::from_millis(60),
            patience: Duration::from_millis(500),
            seed: 99,
        }
    }

    /// The invocation's seed, for the results and `--verify-seed`; `None` when the check is off.
    pub fn seed(&self) -> Option<u64> {
        self.every.map(|_| self.seed)
    }

    /// How long a first look at the answers waits for the ones still missing.
    pub fn settle(&self) -> Duration {
        self.settle
    }

    /// How long the repeat of a run waits.
    pub fn patience(&self) -> Duration {
        self.patience
    }

    /// Why the check can't run, `None` when it can. The first call asks the terminal (see `probe`).
    pub fn unchecked(&mut self, input: &mut Input, out: &mut impl Write) -> io::Result<Option<Why>> {
        if self.every.is_none() {
            return Ok(Some(Why::Off));
        }
        if self.fit == Fitness::Unknown {
            self.fit = match probe::probe(input, out, self.settle)? {
                None => Fitness::Fit,
                Some(why) => Fitness::Unfit(why),
            };
        }
        Ok(match &self.fit {
            Fitness::Unfit(why) => Some(why.clone()),
            _ => None,
        })
    }

    /// The checkpoints for the run `slot` of `payload`. Empty when the check can't run. The places are a
    /// function of the invocation's seed and the slot, so a replay with the same seed puts every run
    /// where it was, whatever else ran.
    pub fn plan(&self, payload: &Payload, usable: bool, slot: Slot<'_>) -> Plan {
        match self.every {
            Some(every) if usable => Plan::new(payload, every, seed::for_slot(self.seed, slot)),
            _ => Plan::none(),
        }
    }

    /// Call before the run's first byte: answers still on their way from before are not this run's.
    pub fn begin(&mut self, input: &mut Input) {
        input.take_status_replies();
    }

    /// Call after the run was timed (so waiting here costs the measurement nothing): counts the answers,
    /// waiting up to `wait` for stragglers, and only when some are missing. `asked` is how many
    /// queries the run really wrote, not how many were planned. When the verdict is "dropped" the count
    /// is kept, for `await_stragglers`.
    pub fn finish(&mut self, input: &mut Input, asked: u64, wait: Duration) -> Check {
        let answered = input.wait_status_replies(asked, wait);
        // `sync` found the cursor report the run ended with; without it nothing is known about the order.
        let in_time = input.status_replies_before_report().unwrap_or(0);
        let check = Check::judge(asked, answered, in_time);
        if !check.is_dropped() {
            input.take_status_replies();
        }
        check
    }

    /// For a run that came out dropped: waits on, up to `PATIENCE`, for the answers still missing, so
    /// that none of them is counted in the repeat of the run. A terminal that answers late may still
    /// have them all by then, but the run's number is not kept: its time ended before they came.
    pub fn await_stragglers(&mut self, input: &mut Input, asked: u64) {
        input.wait_status_replies(asked, self.patience);
        input.take_status_replies();
    }
}

/// The verdict on a run that was repeated because the first try was dropped: dropped again stays
/// dropped; verified now is *unverified*, not verified, because the first try lost answers and the
/// terminal can't be told from one that is slow or uneven (the numbers are the repeat's).
pub fn after_retry(first: &Check, second: Check) -> Check {
    match (first, second) {
        (Check::Dropped { answered, checkpoints }, Check::Verified { .. }) => {
            Check::Unchecked(Why::Retried { answered: *answered, checkpoints: *checkpoints })
        }
        (_, other) => other,
    }
}

#[cfg(test)]
pub(crate) mod fake;
#[cfg(test)]
mod tests;
