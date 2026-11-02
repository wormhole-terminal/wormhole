//! When to read a repository after file events: not at every one. A build writing into an unignored
//! folder sends thousands, and a read is three git processes.

use std::time::{Duration, Instant};

/// A read follows this long after the last event: a save, a checkout or a `git add` arrives as a
/// handful of events, and the read should see all of them.
pub const QUIET: Duration = Duration::from_millis(150);

/// While events keep coming a read still happens after this long, so a long build shows progress...
pub const MAX_WAIT: Duration = Duration::from_secs(1);

/// ...and two reads are at least this far apart, so it does not turn into a read per event.
pub const MIN_GAP: Duration = Duration::from_secs(1);

/// The state of one repository's pending read. It owns no clock: callers pass `now`, and sleep until
/// [`deadline`](Self::deadline).
#[derive(Clone, Copy, Debug, Default)]
pub struct Debounce {
    /// The first and the last event since the last read.
    pending: Option<(Instant, Instant)>,
    read_at: Option<Instant>,
}

impl Debounce {
    pub fn new() -> Self {
        Self::default()
    }

    /// An event that matters arrived.
    pub fn event(&mut self, now: Instant) {
        self.pending = Some(match self.pending {
            Some((first, _)) => (first, now),
            None => (now, now),
        });
    }

    /// Whether a read is waiting for its time.
    pub fn pending(&self) -> bool {
        self.pending.is_some()
    }

    /// When the waiting read is due: [`QUIET`] after the last event, but no later than [`MAX_WAIT`]
    /// after the first, and never within [`MIN_GAP`] of the previous read. `None` with nothing waiting.
    pub fn deadline(&self) -> Option<Instant> {
        let (first, last) = self.pending?;
        let due = (last + QUIET).min(first + MAX_WAIT);
        Some(self.read_at.map_or(due, |read| due.max(read + MIN_GAP)))
    }

    /// Whether to read now; if so the read counts as taken.
    pub fn take(&mut self, now: Instant) -> bool {
        if self.deadline().is_none_or(|due| now < due) {
            return false;
        }
        self.pending = None;
        self.read_at = Some(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(start: Instant, ms: u64) -> Instant {
        start + Duration::from_millis(ms)
    }

    #[test]
    fn a_read_waits_for_quiet() {
        let t = Instant::now();
        let mut d = Debounce::new();
        assert_eq!(d.deadline(), None);
        assert!(!d.take(at(t, 10_000)), "nothing happened, nothing to read");
        d.event(at(t, 0));
        assert!(!d.take(at(t, 149)));
        d.event(at(t, 100));
        assert!(!d.take(at(t, 249)), "a later event moves the deadline");
        assert!(d.take(at(t, 250)));
        assert!(!d.pending());
        assert!(!d.take(at(t, 5_000)), "taken once");
    }

    #[test]
    fn events_that_never_stop_still_read_once_a_second() {
        let t = Instant::now();
        let mut d = Debounce::new();
        let mut reads = Vec::new();
        for ms in (0..=5_000).step_by(20) {
            d.event(at(t, ms));
            if d.take(at(t, ms)) {
                reads.push(ms);
            }
        }
        assert_eq!(reads.first(), Some(&1_000), "the first waits one MAX_WAIT");
        assert!(reads.windows(2).all(|w| w[1] - w[0] >= 1_000), "never closer than MIN_GAP: {reads:?}");
        assert!(reads.len() >= 4, "and not starved: {reads:?}");
    }

    #[test]
    fn a_read_right_after_another_waits_out_the_gap() {
        let t = Instant::now();
        let mut d = Debounce::new();
        d.event(at(t, 0));
        assert!(d.take(at(t, 150)));
        d.event(at(t, 200));
        assert_eq!(d.deadline(), Some(at(t, 1_150)), "a second after the last read, not 350 ms");
        assert!(!d.take(at(t, 1_149)));
        assert!(d.take(at(t, 1_150)));
    }

    #[test]
    fn a_quiet_spell_resets_the_gap() {
        let t = Instant::now();
        let mut d = Debounce::new();
        d.event(at(t, 0));
        assert!(d.take(at(t, 150)));
        d.event(at(t, 60_000));
        assert_eq!(d.deadline(), Some(at(t, 60_150)));
    }
}
