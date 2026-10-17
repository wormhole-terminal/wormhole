//! Where the files are: the shared ones at the top of the data folder, each profile's below it.

use std::path::{Path, PathBuf};

use super::model::ProfileId;

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
        Self::new(crate::store::data_dir())
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
