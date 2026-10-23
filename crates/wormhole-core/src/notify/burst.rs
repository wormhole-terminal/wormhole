//! Keeping the notification count sane: a burst becomes one summary, and a
//! terminal's bell is told at most once a minute.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use super::wording::{Text, summary};
use super::{Kind, NOTIFY_BURST, NOTIFY_BURST_WINDOW};

/// The banners posted lately, to notice a burst.
#[derive(Debug, Default)]
pub struct Bursts {
    recent: VecDeque<Posted>,
}

#[derive(Debug)]
struct Posted {
    at: Instant,
    tab: i64,
    name: String,
    kind: Kind,
}

/// What to post for one more banner.
#[derive(Debug, PartialEq, Eq)]
pub enum Grouped {
    /// Post it on its own.
    Single,
    /// Post (or replace) the summary instead, and take down the burst's own banners.
    Summary { text: Text, retract: Vec<i64> },
}

impl Bursts {
    /// One more banner, about `tab` (the terminal `name`), at `at`. From the [`NOTIFY_BURST`]th
    /// within [`NOTIFY_BURST_WINDOW`] on, the burst is one summary naming every terminal in it,
    /// and every tab in the burst loses its own banner (taking down one that isn't up is fine).
    pub fn add(&mut self, at: Instant, tab: i64, name: &str, kind: Kind) -> Grouped {
        self.recent.retain(|posted| at.saturating_duration_since(posted.at) < NOTIFY_BURST_WINDOW);
        self.recent.push_back(Posted { at, tab, name: name.to_string(), kind });
        if self.recent.len() < NOTIFY_BURST {
            return Grouped::Single;
        }
        let mut names: Vec<String> = Vec::new();
        let mut retract: Vec<i64> = Vec::new();
        for posted in &self.recent {
            if !names.contains(&posted.name) {
                names.push(posted.name.clone());
            }
            if !retract.contains(&posted.tab) {
                retract.push(posted.tab);
            }
        }
        let kinds: Vec<Kind> = self.recent.iter().map(|posted| posted.kind).collect();
        Grouped::Summary { text: summary(&kinds, &names), retract }
    }
}

/// At most one event per key every `every` (a terminal's bell: [`super::BELL_RATE`]).
#[derive(Debug)]
pub struct Throttle {
    every: Duration,
    last: HashMap<i64, Instant>,
}

impl Throttle {
    pub fn new(every: Duration) -> Self {
        Self { every, last: HashMap::new() }
    }

    /// Whether `key` may go now; if so, the clock restarts for it.
    pub fn allow(&mut self, key: i64, at: Instant) -> bool {
        let ok = self.last.get(&key).is_none_or(|last| at.saturating_duration_since(*last) >= self.every);
        if ok {
            self.last.insert(key, at);
        }
        ok
    }

    /// Forget keys that are gone (closed terminals).
    pub fn retain(&mut self, mut keep: impl FnMut(i64) -> bool) {
        self.last.retain(|key, _| keep(*key));
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{Bursts, Grouped, Throttle};
    use crate::notify::{BELL_RATE, Kind};

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    #[test]
    fn three_within_ten_seconds_become_one_summary() {
        let t0 = Instant::now();
        let mut bursts = Bursts::default();
        assert_eq!(bursts.add(t0, 1, "auth tokens", Kind::AgentDone), Grouped::Single);
        assert_eq!(bursts.add(t0 + secs(3), 2, "tests", Kind::CommandDone), Grouped::Single);
        let Grouped::Summary { text, retract } =
            bursts.add(t0 + secs(6), 3, "release notes", Kind::AgentDone)
        else {
            panic!("the third is a summary");
        };
        assert_eq!(text.title, "3 terminals finished");
        assert_eq!(text.body, "auth tokens, tests, release notes");
        assert_eq!(retract, [1, 2, 3], "the burst's own banners go");
        // A fourth joins the summary.
        let Grouped::Summary { text, .. } = bursts.add(t0 + secs(8), 4, "docs", Kind::AgentDone) else {
            panic!("still a burst");
        };
        assert_eq!(text.title, "4 terminals finished");
    }

    #[test]
    fn spread_out_notifications_stay_single() {
        let t0 = Instant::now();
        let mut bursts = Bursts::default();
        for (n, at) in [0, 6, 12, 18, 24].into_iter().enumerate() {
            let grouped = bursts.add(t0 + secs(at), n as i64, "x", Kind::AgentDone);
            assert_eq!(grouped, Grouped::Single, "at {at}s");
        }
    }

    #[test]
    fn one_terminal_twice_is_named_once() {
        let t0 = Instant::now();
        let mut bursts = Bursts::default();
        bursts.add(t0, 1, "build", Kind::CommandDone);
        bursts.add(t0 + secs(1), 2, "prod", Kind::Bell);
        let Grouped::Summary { text, retract } = bursts.add(t0 + secs(2), 1, "build", Kind::CommandDone)
        else {
            panic!("a burst");
        };
        assert_eq!(text.body, "build, prod");
        assert_eq!(text.title, "2 terminals", "a mix of kinds says only how many");
        assert_eq!(retract, [1, 2]);
    }

    #[test]
    fn the_bell_is_told_once_a_minute_per_terminal() {
        let t0 = Instant::now();
        let mut bells = Throttle::new(BELL_RATE);
        assert!(bells.allow(1, t0));
        assert!(!bells.allow(1, t0 + secs(30)));
        assert!(bells.allow(2, t0 + secs(30)), "another terminal has its own minute");
        assert!(bells.allow(1, t0 + secs(60)));
        bells.retain(|key| key != 1);
        assert!(bells.allow(1, t0 + secs(61)), "a forgotten terminal starts afresh");
    }
}
