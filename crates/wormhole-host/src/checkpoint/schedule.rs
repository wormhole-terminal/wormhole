//! When the host writes a checkpoint: a pure schedule on a clock the caller gives, so its rules are tested
//! on a virtual one.
//!
//! - **Idle writes nothing.** A checkpoint is due only after the program printed something since the last
//!   one was asked for.
//! - **Quiet for [`QUIET`], or [`MOST`] since the first new output**, whichever comes first: a prompt that
//!   settles is saved three seconds later; a build that scrolls for ten minutes is saved every half minute.
//! - **A budget of [`BUDGET`] a [`PER`]** (a token bucket that refills evenly and starts full): a terminal
//!   that prints a screenful every second all day writes no more than that. What a write cost is charged
//!   when it is done; while the bucket is empty nothing is due. A flush (the host is about to end) ignores
//!   the budget: it is one write, and the last.

use std::time::{Duration, Instant};

/// Output must have stopped this long for a checkpoint to be due.
pub const QUIET: Duration = Duration::from_secs(3);
/// While output goes on, a checkpoint is due this long after the first output since the last one.
pub const MOST: Duration = Duration::from_secs(30);
/// How many bytes the host may write in [`PER`].
pub const BUDGET: u64 = 64 * 1024 * 1024;
/// The period of [`BUDGET`].
pub const PER: Duration = Duration::from_secs(60 * 60);
/// The bucket must hold this much for a write to be due: about what one write costs.
const MIN_TOKENS: f64 = 1024.0 * 1024.0;

#[derive(Debug)]
pub struct Schedule {
    /// The output total last seen, so a new total means output.
    seen: u64,
    /// When the first output since the last request came, and when the latest did.
    dirty_since: Option<Instant>,
    last_output: Instant,
    tokens: f64,
    refilled: Instant,
}

impl Schedule {
    pub fn new(now: Instant) -> Self {
        Self { seen: 0, dirty_since: None, last_output: now, tokens: BUDGET as f64, refilled: now }
    }

    /// The program's output total (bytes printed so far) as of `now`.
    pub fn note_output(&mut self, total: u64, now: Instant) {
        if total != self.seen {
            self.seen = total;
            self.dirty_since.get_or_insert(now);
            self.last_output = now;
        }
    }

    /// Whether something was printed since the last checkpoint was asked for.
    pub fn is_dirty(&self) -> bool {
        self.dirty_since.is_some()
    }

    /// Something to write though nothing was printed: the first checkpoint, for the folder and the shell.
    pub fn force(&mut self, now: Instant) {
        self.dirty_since.get_or_insert(now);
        self.last_output = now;
    }

    fn refill(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.refilled).as_secs_f64();
        self.tokens = (self.tokens + elapsed * BUDGET as f64 / PER.as_secs_f64()).min(BUDGET as f64);
        self.refilled = now;
    }

    /// When a checkpoint is next due; `None` when nothing was printed since the last (the host waits to be
    /// woken). A time in the past means due now.
    pub fn due_at(&mut self, now: Instant) -> Option<Instant> {
        let since = self.dirty_since?;
        let by_rule = (self.last_output + QUIET).min(since + MOST);
        self.refill(now);
        let by_budget = if self.tokens >= MIN_TOKENS {
            now
        } else {
            let missing = MIN_TOKENS - self.tokens;
            now + Duration::from_secs_f64(missing * PER.as_secs_f64() / BUDGET as f64)
        };
        Some(by_rule.max(by_budget))
    }

    /// Whether a checkpoint is due at `now`.
    pub fn due(&mut self, now: Instant) -> bool {
        self.due_at(now).is_some_and(|at| at <= now)
    }

    /// A checkpoint was asked for: what was printed so far is in it.
    pub fn requested(&mut self) {
        self.dirty_since = None;
    }

    /// A write cost `bytes`.
    pub fn charge(&mut self, bytes: u64, now: Instant) {
        self.refill(now);
        self.tokens -= bytes as f64;
    }

    /// Bytes the budget still holds, for tests and logs.
    pub fn tokens(&mut self, now: Instant) -> u64 {
        self.refill(now);
        self.tokens.max(0.0) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    #[test]
    fn nothing_printed_nothing_due() {
        let t0 = Instant::now();
        let mut s = Schedule::new(t0);
        assert_eq!(s.due_at(t0 + secs(1000)), None);
        s.note_output(0, t0 + secs(5)); // the same total: no output
        assert!(!s.due(t0 + secs(1000)));
    }

    #[test]
    fn output_that_stops_is_due_after_three_quiet_seconds() {
        let t0 = Instant::now();
        let mut s = Schedule::new(t0);
        s.note_output(100, t0);
        assert_eq!(s.due_at(t0), Some(t0 + QUIET));
        assert!(!s.due(t0 + secs(2)));
        assert!(s.due(t0 + secs(3)));
        // More output pushes it back.
        s.note_output(200, t0 + secs(2));
        assert_eq!(s.due_at(t0 + secs(2)), Some(t0 + secs(5)));
    }

    #[test]
    fn output_that_never_stops_is_due_every_thirty_seconds() {
        let t0 = Instant::now();
        let mut s = Schedule::new(t0);
        let mut total = 0;
        let mut writes = Vec::new();
        for second in 0..=95 {
            let now = t0 + secs(second);
            total += 10;
            s.note_output(total, now);
            if s.due(now) {
                s.requested();
                s.charge(1024, now);
                writes.push(second);
            }
        }
        assert_eq!(writes, [30, 61, 92], "first output at 0, then at 31 (the next output after a write), 62");
    }

    #[test]
    fn an_idle_terminal_writes_nothing_for_hours() {
        let t0 = Instant::now();
        let mut s = Schedule::new(t0);
        s.note_output(1, t0);
        assert!(s.due(t0 + secs(3)));
        s.requested();
        for hour in 1..=10 {
            assert!(!s.due(t0 + secs(hour * 3600)), "hour {hour}");
        }
    }

    #[test]
    fn the_budget_caps_what_is_written_in_an_hour() {
        let t0 = Instant::now();
        let mut s = Schedule::new(t0);
        let mut written = 0u64;
        // A screenful of 4 MiB every second for two hours.
        for second in 0..2 * 3600u64 {
            let now = t0 + secs(second);
            s.note_output(second + 1, now);
            // Quiet is never reached; the rule would write every 30 s, so make it due by output stopping.
            if s.due_at(now).is_some() {
                s.last_output = now - QUIET;
            }
            if s.due(now) {
                s.requested();
                s.charge(4 * 1024 * 1024, now);
                written += 4 * 1024 * 1024;
            }
        }
        let allowed = BUDGET + 2 * BUDGET; // the full bucket it started with, and two hours of refill
        assert!(written <= allowed + 4 * 1024 * 1024, "wrote {written} bytes in two hours");
        assert!(written >= 2 * BUDGET, "and still wrote close to the allowance: {written}");
    }

    #[test]
    fn an_empty_bucket_waits_for_it_to_refill() {
        let t0 = Instant::now();
        let mut s = Schedule::new(t0);
        s.charge(BUDGET, t0);
        s.note_output(1, t0);
        let at = s.due_at(t0 + secs(10)).expect("something to write");
        // The missing 1 MiB less the ten seconds that refilled: of a 64 MiB hour, about 46 seconds.
        let wait = at.duration_since(t0 + secs(10));
        assert!(wait > secs(40) && wait < secs(50), "{wait:?}");
        assert!(!s.due(t0 + secs(30)));
        assert!(s.due(t0 + secs(70)));
    }
}
