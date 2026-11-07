//! The updater's values as Swift reads them (`AuxModel/Update`).

/// Which feeds a check reads. `Preview` is core's `UpdateChannel::Unstable` (its rename is pending: the feed
/// path stays `unstable/latest.json` until the release workflow moves).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum UpdateChannelRecord {
    Stable,
    Preview,
}

/// Where and what to update: the release build's facts, given by the app (a dev build has no `base_url`).
#[derive(Debug, Clone, uniffi::Record)]
pub struct UpdateSetup {
    /// `<base>/<channel>/latest.json` is the feed.
    pub base_url: String,
    /// The minisign public key (the `.pub` file's text or its key line).
    pub public_key: String,
    pub current_version: String,
    pub bundle_id: String,
    pub team_id: Option<String>,
    /// The `.app` to replace; `None` for a build that does not run from one (it can check, not install).
    pub app_bundle: Option<String>,
    /// `codesign --verify` on the new bundle: on for a release, off only for fixtures.
    pub verify_codesign: bool,
    /// A GitHub `…/releases` URL read when the feed does not answer.
    pub fallback_url: Option<String>,
    /// Where the check schedule and the skipped version are kept; `None` is the data folder's own.
    pub schedule_file: Option<String>,
    pub channel: UpdateChannelRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct UpdateReleaseRecord {
    pub version: String,
    /// Markdown.
    pub notes: String,
    /// RFC 3339 as published.
    pub published: Option<String>,
    pub is_security: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum UpdatePhaseRecord {
    Idle,
    Checking,
    UpToDate,
    Available { release: UpdateReleaseRecord },
    /// A newer version exists but needs a newer macOS (said), or this build cannot update itself.
    Unsupported { reason: String },
    Downloading { received: u64, expected: u64 },
    Installed { version: String },
    Failed { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct UpdateStateRecord {
    pub phase: UpdatePhaseRecord,
    pub channel: UpdateChannelRecord,
    /// Unix seconds of the last check that reached a feed.
    pub last_checked: Option<u64>,
    /// What a change of channel did to a build of the channel left: the version, and whether it was called
    /// off (`removed`) or stays installed.
    pub channel_note: Option<UpdateChannelNote>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct UpdateChannelNote {
    pub version: String,
    pub stays_installed: bool,
}
