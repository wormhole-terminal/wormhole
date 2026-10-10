//! When to check automatically, and which version the user chose to skip.
//!
//! Pure bookkeeping: the caller supplies "now" and persists this struct (it's serde) wherever it
//! keeps settings, so it's trivially testable and survives restarts.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use semver::Version;
use serde::{Deserialize, Serialize};

use super::UpdateStatus;

/// Background re-check interval while the app stays open.
pub const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
/// At launch we check unless we checked very recently, so quitting and reopening a few times in a
/// row doesn't hit the feed each time.
pub const LAUNCH_MIN_GAP: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateSchedule {
    /// Unix seconds of the last successful check.
    #[serde(default)]
    pub last_check: Option<u64>,
    /// Automatic checks stay quiet about exactly this version; a newer one is announced again.
    #[serde(default)]
    pub skipped_version: Option<Version>,
}

impl UpdateSchedule {
    /// Whether to check right after launch.
    pub fn due_at_launch(&self, now: SystemTime) -> bool {
        self.elapsed(now).is_none_or(|e| e >= LAUNCH_MIN_GAP)
    }

    /// Whether the periodic timer should check now.
    pub fn is_due(&self, now: SystemTime) -> bool {
        self.elapsed(now).is_none_or(|e| e >= CHECK_INTERVAL)
    }

    /// How long until [`is_due`](Self::is_due) turns true, for arming a timer.
    pub fn next_check_in(&self, now: SystemTime) -> Duration {
        self.elapsed(now).map_or(Duration::ZERO, |e| CHECK_INTERVAL.saturating_sub(e))
    }

    /// Call after a check that reached the server (whatever the answer), not after network
    /// errors, so an offline launch retries on the next opportunity.
    pub fn record_check(&mut self, now: SystemTime) {
        self.last_check = Some(unix_secs(now));
    }

    pub fn skip(&mut self, version: &Version) {
        self.skipped_version = Some(version.clone());
    }

    pub fn is_skipped(&self, version: &Version) -> bool {
        self.skipped_version.as_ref().is_some_and(|s| s.cmp_precedence(version).is_eq())
    }

    /// Whether an *automatic* check should show this result. A check the user asked for
    /// ("Check for Updates…") should show everything, skipped or not.
    pub fn should_notify(&self, status: &UpdateStatus) -> bool {
        match status {
            UpdateStatus::Available(release) => !self.is_skipped(&release.version),
            UpdateStatus::UpToDate | UpdateStatus::Unsupported { .. } => false,
        }
    }

    /// `None` when there's no usable last check, including a clock that went backwards (the
    /// stored time is in the future), which would otherwise suppress checks indefinitely.
    fn elapsed(&self, now: SystemTime) -> Option<Duration> {
        let last = self.last_check?;
        unix_secs(now).checked_sub(last).map(Duration::from_secs)
    }
}

fn unix_secs(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use semver::Version;

    use super::{CHECK_INTERVAL, UpdateSchedule};
    use crate::update::UpdateStatus;
    use crate::update::test_support::sample_release;

    fn at(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    #[test]
    fn first_launch_checks() {
        let s = UpdateSchedule::default();
        assert!(s.due_at_launch(at(1_000_000)));
        assert!(s.is_due(at(1_000_000)));
        assert_eq!(s.next_check_in(at(1_000_000)), Duration::ZERO);
    }

    #[test]
    fn interval_and_launch_gap() {
        let mut s = UpdateSchedule::default();
        s.record_check(at(1_000_000));
        // Reopened 10 minutes later: skip; 2 hours later: check.
        assert!(!s.due_at_launch(at(1_000_600)));
        assert!(s.due_at_launch(at(1_007_200)));
        // Periodic: only after 6 hours.
        assert!(!s.is_due(at(1_007_200)));
        assert_eq!(s.next_check_in(at(1_007_200)), CHECK_INTERVAL - Duration::from_secs(7200));
        assert!(s.is_due(at(1_000_000 + 6 * 3600)));
        assert_eq!(s.next_check_in(at(1_000_000 + 7 * 3600)), Duration::ZERO);
    }

    #[test]
    fn clock_going_backwards_triggers_a_check() {
        let mut s = UpdateSchedule::default();
        s.record_check(at(2_000_000));
        assert!(s.is_due(at(1_000_000)));
        assert!(s.due_at_launch(at(1_000_000)));
    }

    #[test]
    fn skipped_version_is_quiet_until_a_newer_one() {
        let mut s = UpdateSchedule::default();
        let v020 = UpdateStatus::Available(sample_release("0.2.0"));
        assert!(s.should_notify(&v020));
        s.skip(&Version::new(0, 2, 0));
        assert!(!s.should_notify(&v020));
        assert!(s.should_notify(&UpdateStatus::Available(sample_release("0.2.1"))));
        assert!(!s.should_notify(&UpdateStatus::UpToDate));
    }

    #[test]
    fn round_trips_through_json() {
        let mut s = UpdateSchedule::default();
        s.record_check(at(42));
        s.skip(&Version::new(1, 2, 3));
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<UpdateSchedule>(&json).unwrap(), s);
        assert_eq!(serde_json::from_str::<UpdateSchedule>("{}").unwrap(), UpdateSchedule::default());
    }
}
