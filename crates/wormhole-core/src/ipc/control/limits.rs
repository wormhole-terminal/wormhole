//! How often a caller may ask. A script in a loop, or an agent that keeps notifying, must not
//! flood the user with banners, type endlessly into other terminals or open terminals until the
//! window is full: each kind of request gets a budget per caller (the tab it runs in, or
//! "outside" for a shell elsewhere) over a sliding minute. Looking at the list (`list`, `status`) is
//! free; reading text is not.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// The requests that have a budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Limited {
    Notify,
    SendText,
    NewTerminal,
    /// `read` and `blocks`: each costs the socket thread a copy of the screen under the terminal's lock,
    /// so a loop of them is slowed down (ten a second keeps the thread mostly free for hooks).
    Read,
    /// `send-keys`: a program driven from a script takes bursts of them.
    SendKeys,
    /// `mouse`.
    Mouse,
    /// A stream of events (`events`): each one holds a thread and a place among the few streams there
    /// are, so how often one is started is held down too.
    Subscribe,
    /// The welcome's choices of a theme, a colour or a font size: a person stepping through a
    /// list sends many, and each is a small settings change.
    WelcomeLook,
    /// The welcome's Install for an agent's hooks (a file write).
    WelcomeInstall,
    /// The welcome's notification choices (a switch, the sound, Allow): a few presses, and the first
    /// one that turns a banner on may show macOS's own question.
    WelcomeNotify,
    /// The welcome's example band in the sidebar going up and coming down: once for each time the
    /// sidebar step is entered or left, so stepping back and forth sends many.
    WelcomeDemo,
    /// The welcome's Import of another terminal's settings (reads its files, one settings update): a
    /// press, so a few, even with a retry.
    WelcomeImport,
    /// The welcome's end, which is only ever sent once.
    WelcomeFinish,
}

impl Limited {
    /// How many a caller gets per [`WINDOW`].
    pub const fn budget(self) -> usize {
        match self {
            Limited::Notify => 10,
            Limited::SendText => 30,
            Limited::NewTerminal => 5,
            Limited::Read => 600,
            Limited::SendKeys => 240,
            Limited::Mouse => 240,
            Limited::Subscribe => 30,
            Limited::WelcomeLook => 240,
            Limited::WelcomeInstall => 12,
            Limited::WelcomeNotify => 30,
            Limited::WelcomeDemo => 120,
            Limited::WelcomeImport => 6,
            Limited::WelcomeFinish => 4,
        }
    }
}

/// The sliding window the budgets apply to.
pub(super) const WINDOW: Duration = Duration::from_secs(60);

/// Who asked: a tab's id, or `None` for a shell outside the app.
type Caller = Option<i64>;

/// Each caller's recent requests of each kind.
#[derive(Debug, Default)]
pub struct RateLimits {
    recent: HashMap<(Limited, Caller), VecDeque<Instant>>,
}

impl RateLimits {
    /// Whether `caller` may make a `kind` request at `now`; if so it counts against the budget.
    pub fn allow(&mut self, kind: Limited, caller: Caller, now: Instant) -> bool {
        let recent = self.recent.entry((kind, caller)).or_default();
        while recent.front().is_some_and(|at| now.saturating_duration_since(*at) >= WINDOW) {
            recent.pop_front();
        }
        if recent.len() >= kind.budget() {
            return false;
        }
        recent.push_back(now);
        true
    }

    /// How long until `caller` may make a `kind` request again, for the error's words.
    pub fn retry_after(&self, kind: Limited, caller: Caller, now: Instant) -> Duration {
        self.recent
            .get(&(kind, caller))
            .and_then(|recent| recent.front())
            .map(|oldest| WINDOW.saturating_sub(now.saturating_duration_since(*oldest)))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{Limited, RateLimits, WINDOW};

    #[test]
    fn each_caller_has_a_budget_per_minute_and_per_kind() {
        let mut limits = RateLimits::default();
        let t0 = Instant::now();
        for n in 0..Limited::NewTerminal.budget() {
            assert!(limits.allow(Limited::NewTerminal, Some(1), t0 + Duration::from_secs(n as u64)), "{n}");
        }
        assert!(
            !limits.allow(Limited::NewTerminal, Some(1), t0 + Duration::from_secs(10)),
            "the budget is spent"
        );
        // Another caller, and another kind for the same caller, have their own.
        assert!(limits.allow(Limited::NewTerminal, Some(2), t0 + Duration::from_secs(10)));
        assert!(limits.allow(Limited::NewTerminal, None, t0 + Duration::from_secs(10)), "a shell outside");
        assert!(limits.allow(Limited::Notify, Some(1), t0 + Duration::from_secs(10)));
        // The window slides: the oldest request falls out of it after a minute.
        let retry = limits.retry_after(Limited::NewTerminal, Some(1), t0 + Duration::from_secs(10));
        assert_eq!(retry, Duration::from_secs(50));
        assert!(!limits.allow(Limited::NewTerminal, Some(1), t0 + WINDOW - Duration::from_millis(1)));
        assert!(limits.allow(Limited::NewTerminal, Some(1), t0 + WINDOW));
        assert!(!limits.allow(Limited::NewTerminal, Some(1), t0 + WINDOW), "only one fell out");
        assert!(limits.allow(Limited::NewTerminal, Some(1), t0 + WINDOW + Duration::from_secs(1)));
    }

    #[test]
    fn the_budgets_are_small_enough_to_notice_a_loop() {
        assert!(Limited::Notify.budget() <= 10);
        assert!(Limited::NewTerminal.budget() <= 5);
        assert!(Limited::SendText.budget() <= 30 && Limited::Subscribe.budget() <= 30);
        assert!(Limited::WelcomeInstall.budget() <= 12 && Limited::WelcomeNotify.budget() <= 30);
        assert!(Limited::WelcomeDemo.budget() <= 120 && Limited::WelcomeFinish.budget() <= 4);
        assert!(Limited::WelcomeImport.budget() <= 6);
        assert_eq!(RateLimits::default().retry_after(Limited::Notify, None, Instant::now()), Duration::ZERO);
    }
}
