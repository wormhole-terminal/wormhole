//! Update channels: which builds a person updates to.
//!
//! Stable is the tagged releases. Unstable is also a build of every commit on main that passed CI
//! (`.github/workflows/release.yml`, versioned `X.Y.Z-dev.N`), published as one rolling GitHub
//! prerelease. An Unstable user reads both feeds and takes the newer, so a stable release that
//! overtakes the last main build is still offered without CI keeping the two feeds in step.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    #[default]
    Stable,
    Unstable,
}

impl UpdateChannel {
    /// The feed's path below the base URL; CI publishes to exactly this key.
    pub fn feed_path(self) -> &'static str {
        match self {
            Self::Stable => "stable/latest.json",
            Self::Unstable => "unstable/latest.json",
        }
    }

    /// The feed's path below a GitHub repository's `releases/`, where the release workflow publishes
    /// it: the latest stable release's `latest.json`, or the rolling `unstable` prerelease's.
    pub fn github_feed_path(self) -> &'static str {
        match self {
            Self::Stable => "latest/download/latest.json",
            Self::Unstable => "download/unstable/latest.json",
        }
    }

    /// The channel's name, as the feed's path and the update check's user agent spell it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Unstable => "unstable",
        }
    }

    /// The feeds a check reads, its own first.
    pub fn feeds(self) -> &'static [UpdateChannel] {
        match self {
            Self::Stable => &[Self::Stable],
            Self::Unstable => &[Self::Unstable, Self::Stable],
        }
    }

    /// Whether a prerelease may be offered. Stable never moves someone onto one, even if its
    /// (unsigned) feed names one.
    pub fn takes_prereleases(self) -> bool {
        self == Self::Unstable
    }
}
