//! Where the files are: the data folder, the shared files at its top and each profile's below it,
//! and the profile id that names a profile's folder there. Profiles themselves (what one has, the
//! list) are `profile`'s; this is only the layout on disk, which the store needs too.

use std::fmt;
use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::platform::paths as platform_paths;

/// The Default profile's id. It always exists and keeps the data folder's own files.
pub const DEFAULT_ID: &str = "default";

const MAX_ID: usize = 32;

/// A profile's id: the folder name under `profiles/`, so only what is safe as one (lower-case
/// letters, digits and dashes, 1 to 32 characters, no dash at either end). Whatever the file says,
/// an id can't lead a path out of the data folder.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProfileId(String);

impl ProfileId {
    pub fn default_profile() -> Self {
        Self(DEFAULT_ID.into())
    }

    pub fn parse(text: &str) -> Result<Self, InvalidProfileId> {
        let ok = !text.is_empty()
            && text.len() <= MAX_ID
            && !text.starts_with('-')
            && !text.ends_with('-')
            && text.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        if ok { Ok(Self(text.into())) } else { Err(InvalidProfileId(text.into())) }
    }

    /// `p<seconds>`, an id that is valid by construction: the last resort when a profile's name
    /// gives no free one.
    pub(crate) fn from_clock(secs: u64) -> Self {
        Self(format!("p{secs}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_default(&self) -> bool {
        self.0 == DEFAULT_ID
    }
}

impl fmt::Display for ProfileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for ProfileId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ProfileId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}

/// Why a text isn't a profile id; it says so to the person who typed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidProfileId(String);

impl fmt::Display for InvalidProfileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\" is not a profile id", self.0)
    }
}

impl std::error::Error for InvalidProfileId {}

/// The running app's data folder: `Wormhole` in Application Support, or the folder
/// `WORMHOLE_DATA_DIR` names (used for isolated test runs).
pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("WORMHOLE_DATA_DIR") {
        return PathBuf::from(dir);
    }
    platform_paths::data_base().unwrap_or_else(|| PathBuf::from(".")).join(data_dir_name())
}

/// The data folder's name in Application Support.
pub(crate) fn data_dir_name() -> &'static str {
    "Wormhole"
}

/// The single-instance lock's file at the top of a data folder: the running instance holds a lock
/// on it (`ipc::instance`).
pub(crate) const INSTANCE_LOCK_FILE: &str = "instance.lock";

/// Whether an instance runs with `dir` as its data folder (it holds the lock there). Taking the
/// lock to find out lets it go again at once.
pub(crate) fn in_use(dir: &Path) -> bool {
    File::open(dir.join(INSTANCE_LOCK_FILE))
        .is_ok_and(|file| matches!(file.try_lock(), Err(TryLockError::WouldBlock)))
}

/// The data folder as a whole. Its own files are shared by every profile; a profile's files are in
/// [`ProfilePaths`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataRoot {
    base: PathBuf,
}

impl DataRoot {
    pub fn new(base: impl Into<PathBuf>) -> Self {
        Self { base: base.into() }
    }

    /// The running app's data folder (`WORMHOLE_DATA_DIR` overrides it, as it does everywhere).
    pub fn current() -> Self {
        Self::new(data_dir())
    }

    pub fn base(&self) -> &Path {
        &self.base
    }

    /// The profile list and which profile is active.
    pub fn profiles_file(&self) -> PathBuf {
        self.base.join("profiles.json")
    }

    /// The folder every profile but Default keeps its files in.
    pub fn profiles_dir(&self) -> PathBuf {
        self.base.join("profiles")
    }

    /// When the app last looked for an update and which version was skipped: the update channel,
    /// the one thing every profile shares besides the app itself.
    pub fn update_schedule_file(&self) -> PathBuf {
        self.base.join("update-schedule.json")
    }

    /// The `wh` command's folder, shared like the app it points at.
    pub fn bin_dir(&self) -> PathBuf {
        self.base.join("bin")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.base.join("logs")
    }

    /// The Probes: one folder each, named by the Probe's id. Shared by every profile, like the
    /// app's own code they extend.
    pub fn probes_dir(&self) -> PathBuf {
        self.base.join("probes")
    }

    /// The copies Probes run from: one read-only folder per approved Probe, made at approval from
    /// the bytes the user was shown, so an edit of `probes/<id>/` afterwards runs nothing.
    pub fn probes_approved_dir(&self) -> PathBuf {
        self.base.join("probes-approved")
    }

    /// What the user approved, per Probe (`probe::approval`).
    pub fn probe_approvals_file(&self) -> PathBuf {
        self.base.join("probe-approvals.json")
    }

    /// One profile's files. Default's are the data folder's own, where they have always been.
    pub fn profile(&self, id: &ProfileId) -> ProfilePaths {
        let dir = if id.is_default() { self.base.clone() } else { self.profiles_dir().join(id.as_str()) };
        ProfilePaths { id: id.clone(), dir }
    }

    pub fn default_profile(&self) -> ProfilePaths {
        self.profile(&ProfileId::default_profile())
    }
}

/// One profile's files, with the names the data folder has always used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfilePaths {
    id: ProfileId,
    dir: PathBuf,
}

impl ProfilePaths {
    pub fn id(&self) -> &ProfileId {
        &self.id
    }

    /// The folder the database, trusted projects and project preferences are in
    /// (`Store::open_at`, `TrustStore::load`, …).
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn config_file(&self) -> PathBuf {
        self.dir.join("config.json")
    }

    pub fn themes_dir(&self) -> PathBuf {
        self.dir.join("themes")
    }

    pub fn fonts_dir(&self) -> PathBuf {
        self.dir.join("fonts")
    }

    pub fn icon_themes_dir(&self) -> PathBuf {
        self.dir.join("icon-themes")
    }
}
