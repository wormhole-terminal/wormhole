//! The anonymous count: how many copies of Wormhole run, without telling one from another.
//!
//! An update check already reaches the feed, so it says a little more: the user agent names the
//! version, channel, chip and macOS major version (each shared by thousands of Macs), and the first
//! check of a UTC day and of a UTC month carries `Wormhole-Count: day` / `month`. The server adds
//! up those flags: a day's `day` flags are that day's running copies, a month's `month` flags the
//! month's, with no ID, no stored address and nothing that links two checks of one Mac. With the
//! count off (`anonymous_count`), a check sends what it always did: the version alone.
//!
//! Counting starts [`GRACE`] after a version that counts first runs (`UpdateSchedule::count_due`),
//! on a new Mac and after an update alike, so there is time to find the switch first.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Datelike};
use semver::Version;

use super::{UpdateChannel, UpdaterConfig};

/// How long after it first runs a copy waits before it is counted.
pub(crate) const GRACE: Duration = Duration::from_secs(24 * 60 * 60);

/// The request header that carries the flags.
pub(crate) const HEADER: &str = "Wormhole-Count";

/// Which periods this check is the first of. Both false still sends the detailed user agent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Count {
    pub day: bool,
    pub month: bool,
}

impl Count {
    /// The header's value, `None` when this check is the first of nothing.
    pub(crate) fn header(self) -> Option<&'static str> {
        match (self.day, self.month) {
            (true, true) => Some("day, month"),
            (true, false) => Some("day"),
            (false, true) => Some("month"),
            (false, false) => None,
        }
    }
}

/// The user agent of a feed request: the version alone while the count is off.
pub(crate) fn user_agent(config: &UpdaterConfig) -> String {
    match config.count {
        Some(_) => counted_user_agent(
            &config.current_version,
            config.channel,
            &config.platform,
            config.system_version.as_ref(),
        ),
        None => plain_user_agent(&config.current_version),
    }
}

/// The user agent of a check while the count is off: the version alone, as before the count.
pub fn plain_user_agent(version: &Version) -> String {
    format!("Wormhole-Updater/{version}")
}

/// The user agent of a counted check, which Settings → Updates shows as what this Mac sends.
/// `platform` is the feed's key (`darwin-aarch64`, [`super::current_platform`]).
pub fn counted_user_agent(
    version: &Version,
    channel: UpdateChannel,
    platform: &str,
    system_version: Option<&Version>,
) -> String {
    let arch = platform.rsplit('-').next().unwrap_or("unknown");
    let macos = system_version.map_or_else(|| "unknown".to_string(), |v| v.major.to_string());
    format!("Wormhole-Updater/{version} ({}; {arch}; macOS {macos})", channel.name())
}

/// The UTC day (days since 1970) and month (`year * 12 + month - 1`) of `now`.
pub(crate) fn periods(now: SystemTime) -> (u32, u32) {
    let secs = now.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let day = u32::try_from(secs / 86_400).unwrap_or(u32::MAX);
    let month = i64::try_from(secs)
        .ok()
        .and_then(|secs| DateTime::from_timestamp(secs, 0))
        .and_then(|t| u32::try_from(t.year()).ok().map(|year| year * 12 + t.month0()))
        .unwrap_or(0);
    (day, month)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use semver::Version;

    use super::{Count, periods, user_agent};
    use crate::update::UpdateChannel;
    use crate::update::test_support::test_config;

    #[test]
    fn the_header_names_the_periods_this_check_is_the_first_of() {
        assert_eq!(Count { day: true, month: true }.header(), Some("day, month"));
        assert_eq!(Count { day: true, month: false }.header(), Some("day"));
        assert_eq!(Count { day: false, month: true }.header(), Some("month"));
        assert_eq!(Count::default().header(), None);
    }

    #[test]
    fn the_user_agent_says_more_only_while_counting() {
        let mut config = test_config("1.3.0", "");
        assert_eq!(user_agent(&config), "Wormhole-Updater/1.3.0");
        config.count = Some(Count::default());
        assert_eq!(user_agent(&config), "Wormhole-Updater/1.3.0 (stable; aarch64; macOS 14)");
        config.channel = UpdateChannel::Unstable;
        config.platform = "darwin-x86_64".into();
        config.system_version = None;
        assert_eq!(user_agent(&config), "Wormhole-Updater/1.3.0 (unstable; x86_64; macOS unknown)");
        config.system_version = Some(Version::new(26, 1, 0));
        assert!(user_agent(&config).ends_with("macOS 26)"));
    }

    #[test]
    fn days_and_months_are_utc_calendar_ones() {
        // 2026-10-31 23:59:59 and 2026-11-01 00:00:00 UTC.
        let last = UNIX_EPOCH + Duration::from_secs(1_793_491_199);
        let next = last + Duration::from_secs(1);
        let (d1, m1) = periods(last);
        let (d2, m2) = periods(next);
        assert_eq!((d2 - d1, m1, m2), (1, 2026 * 12 + 9, 2026 * 12 + 10));
        assert_eq!(periods(next + Duration::from_secs(3600)), (d2, m2));
    }
}
