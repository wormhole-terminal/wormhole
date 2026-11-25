//! Update channels: which builds a person updates to.
//!
//! Stable is the tagged releases. Unstable is also a build of every commit on main that passed CI
//! (`.github/workflows/release.yml`, versioned `X.Y.Z-dev.N`), published as one rolling GitHub
//! prerelease. An Unstable user reads both feeds and takes the newer, so a stable release that
//! overtakes the last main build is still offered without CI keeping the two feeds in step.
//!
//! Beta is the Swift app's own channel: one rolling prerelease (`beta`) that only that app reads, so
//! it never sees the GPUI app's Stable or Unstable builds, which are other programs. It is baked
//! into a build, never a setting: a person on the beta has no channel to choose.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    #[default]
    Stable,
    Unstable,
    /// Not a value of the `update_channel` setting, which stays two-valued: `skip` keeps it out of
    /// `config.json` both ways (a file saying `"beta"` is an unknown value, as any other, and
    /// writing one is an error rather than a file nothing can read back).
    #[serde(skip)]
    Beta,
}

impl UpdateChannel {
    /// The feed's path below the base URL; CI publishes to exactly this key.
    pub fn feed_path(self) -> &'static str {
        match self {
            Self::Stable => "stable/latest.json",
            Self::Unstable => "unstable/latest.json",
            Self::Beta => "beta/latest.json",
        }
    }

    /// The feed's path below a GitHub repository's `releases/`, where the release workflow publishes
    /// it: the latest stable release's `latest.json`, or the rolling `unstable` prerelease's.
    pub fn github_feed_path(self) -> &'static str {
        match self {
            Self::Stable => "latest/download/latest.json",
            Self::Unstable => "download/unstable/latest.json",
            Self::Beta => "download/beta/latest.json",
        }
    }

    /// The channel's name, as the feed's path and the update check's user agent spell it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Unstable => "unstable",
            Self::Beta => "beta",
        }
    }

    /// The feeds a check reads, its own first. Beta reads only its own: Stable's and Unstable's
    /// are the GPUI app's releases, not builds of this app.
    pub fn feeds(self) -> &'static [UpdateChannel] {
        match self {
            Self::Stable => &[Self::Stable],
            Self::Unstable => &[Self::Unstable, Self::Stable],
            Self::Beta => &[Self::Beta],
        }
    }

    /// Whether a prerelease may be offered. Stable never moves someone onto one, even if its
    /// (unsigned) feed names one.
    pub fn takes_prereleases(self) -> bool {
        matches!(self, Self::Unstable | Self::Beta)
    }
}

#[cfg(test)]
mod tests {
    use super::UpdateChannel::{self, Beta, Stable, Unstable};

    #[test]
    fn beta_has_a_feed_of_its_own_on_the_download_server_and_on_github() {
        assert_eq!(Beta.feed_path(), "beta/latest.json");
        assert_eq!(Beta.github_feed_path(), "download/beta/latest.json");
        assert_eq!(Beta.name(), "beta");
        // The GPUI app's channels are untouched.
        assert_eq!(
            (Stable.feed_path(), Unstable.feed_path()),
            ("stable/latest.json", "unstable/latest.json")
        );
    }

    #[test]
    fn beta_reads_only_its_own_feed_and_takes_prereleases() {
        assert_eq!(Beta.feeds(), [Beta]);
        assert!(Beta.takes_prereleases());
        assert_eq!(Unstable.feeds(), [Unstable, Stable]);
        assert!(!Stable.takes_prereleases());
    }

    #[test]
    fn beta_is_not_a_value_of_the_setting() {
        assert!(serde_json::from_str::<UpdateChannel>("\"beta\"").is_err(), "a file saying beta");
        assert!(serde_json::to_string(&Beta).is_err(), "writing it");
        assert_eq!(serde_json::to_string(&Unstable).unwrap(), "\"unstable\"");
        assert_eq!(serde_json::from_str::<UpdateChannel>("\"stable\"").unwrap(), Stable);
    }
}
