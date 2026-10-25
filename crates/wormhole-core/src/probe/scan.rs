//! The Probes in the data folder: `<data dir>/probes/<id>/`, each read the way [`ProbeFolder::read`]
//! reads one, and listed whether it reads or not so that a broken one can be shown with its reason.
//!
//! Finding a folder is not trusting it. Nothing is enabled, approved or run here, and a folder that
//! appears (an agent wrote it, a `git pull` brought it) is only listed.

use std::fmt;
use std::fs;
use std::path::Path;

use super::manifest::is_probe_id;
use super::read::{ProbeFolder, ReadError};
use crate::storage::paths::DataRoot;

/// Entries of the Probes folder, of any kind: more is not a list of Probes.
pub const MAX_ENTRIES: usize = 256;

/// One folder of the Probes folder.
#[derive(Debug)]
pub struct Scanned {
    /// The folder's name.
    pub name: String,
    pub probe: Result<ProbeFolder, ScanError>,
}

/// Why a folder is listed but is no usable Probe.
#[derive(Debug)]
pub enum ScanError {
    /// The name is not an id, so it cannot be a Probe's folder.
    BadName,
    /// The entry is a link. A Probe is a copy in the data folder; loading one from elsewhere is a
    /// deliberate act of its own, never something a link in here can do.
    Link,
    /// `probe.id` is not the folder's name.
    IdMismatch {
        id: String,
    },
    Read(ReadError),
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadName => f.write_str("the folder's name is not a Probe id"),
            Self::Link => f.write_str("is a link; a Probe in the data folder is a folder of its own"),
            Self::IdMismatch { id } => write!(f, "probe.toml says its id is `{id}`, not the folder's name"),
            Self::Read(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for ScanError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read(err) => Some(err),
            _ => None,
        }
    }
}

/// The Probes folder holds more than [`MAX_ENTRIES`] entries: not a list of Probes, and listing it
/// would read as many folders. Nothing is listed, rather than the first few in directory order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TooManyEntries;

impl fmt::Display for TooManyEntries {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the Probes folder has more than {MAX_ENTRIES} entries")
    }
}

impl std::error::Error for TooManyEntries {}

/// The Probes of `root`'s data folder.
pub fn scan_data(root: &DataRoot) -> Result<Vec<Scanned>, TooManyEntries> {
    scan(&root.probes_dir())
}

/// The folders of `dir`, sorted by name. A missing `dir` is no Probes; files and `.`-folders in it
/// are not Probes and are left out, but every entry counts against [`MAX_ENTRIES`].
pub fn scan(dir: &Path) -> Result<Vec<Scanned>, TooManyEntries> {
    let Ok(entries) = fs::read_dir(dir) else { return Ok(Vec::new()) };
    let entries: Vec<_> = entries.filter_map(Result::ok).take(MAX_ENTRIES + 1).collect();
    if entries.len() > MAX_ENTRIES {
        return Err(TooManyEntries);
    }
    let mut found: Vec<Scanned> = entries
        .into_iter()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                return None;
            }
            let kind = fs::symlink_metadata(entry.path()).ok()?.file_type();
            if kind.is_symlink() {
                return Some(Scanned { name, probe: Err(ScanError::Link) });
            }
            kind.is_dir().then(|| load(name, &entry.path()))
        })
        .collect();
    found.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(found)
}

pub(crate) fn load(name: String, path: &Path) -> Scanned {
    let probe = if !is_probe_id(&name) {
        Err(ScanError::BadName)
    } else {
        ProbeFolder::read(path).map_err(ScanError::Read).and_then(|folder| {
            if folder.manifest.probe.id == name {
                Ok(folder)
            } else {
                Err(ScanError::IdMismatch { id: folder.manifest.probe.id })
            }
        })
    };
    Scanned { name, probe }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;
    use crate::platform::fs::symlink;

    fn manifest(id: &str) -> String {
        format!(
            "[probe]\nid = \"{id}\"\nname = \"N\"\ndescription = \"d\"\nauthor = \"a\"\nversion = \"0.1.0\"\n\
             wormhole = \">=1.3\"\napi = 1\n"
        )
    }

    fn install(dir: &Path, folder: &str, id: &str) {
        fs::create_dir_all(dir.join(folder)).expect("mkdir");
        fs::write(dir.join(folder).join("probe.toml"), manifest(id)).expect("write");
    }

    #[test]
    fn the_probes_folder_is_in_the_data_folder() {
        let root = DataRoot::new("/data");
        assert_eq!(root.probes_dir(), Path::new("/data/probes"));
    }

    #[test]
    fn a_missing_folder_has_no_probes() {
        let tmp = TempDir::new().expect("tempdir");
        assert!(scan(&tmp.path().join("probes")).expect("scans").is_empty());
        let root = DataRoot::new(tmp.path());
        assert!(scan_data(&root).expect("scans").is_empty());
    }

    #[test]
    fn each_folder_is_listed_sorted_with_its_probe_or_its_reason() {
        let tmp = TempDir::new().expect("tempdir");
        let dir = tmp.path();
        install(dir, "zeta", "zeta");
        install(dir, "alpha", "alpha");
        install(dir, "moved", "someone-else");
        install(dir, "Not An Id", "x");
        fs::create_dir(dir.join("empty")).expect("mkdir");
        fs::create_dir(dir.join(".staging")).expect("mkdir");
        fs::write(dir.join("README"), "hi").expect("write");
        symlink(dir.join("alpha"), dir.join("linked")).expect("link");

        let found = scan(dir).expect("scans");
        let names: Vec<_> = found.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Not An Id", "alpha", "empty", "linked", "moved", "zeta"]);
        let by = |name: &str| &found.iter().find(|s| s.name == name).expect(name).probe;
        assert!(matches!(by("Not An Id"), Err(ScanError::BadName)));
        assert!(by("alpha").is_ok());
        assert!(by("zeta").is_ok());
        assert!(matches!(by("empty"), Err(ScanError::Read(ReadError::NoManifest))));
        assert!(matches!(by("linked"), Err(ScanError::Link)));
        assert!(matches!(by("moved"), Err(ScanError::IdMismatch { id }) if id == "someone-else"));
    }

    #[test]
    fn a_folder_with_too_many_entries_is_not_listed_at_all() {
        let tmp = TempDir::new().expect("tempdir");
        for ix in 0..MAX_ENTRIES {
            fs::write(tmp.path().join(format!("f{ix}")), "").expect("write");
        }
        assert!(scan(tmp.path()).expect("exactly at the cap").is_empty(), "files are no Probes");
        fs::write(tmp.path().join("one-more"), "").expect("write");
        assert_eq!(scan(tmp.path()).map(|found| found.len()), Err(TooManyEntries));
    }

    #[test]
    fn a_broken_manifest_is_listed_with_the_reason() {
        let tmp = TempDir::new().expect("tempdir");
        fs::create_dir(tmp.path().join("broken")).expect("mkdir");
        fs::write(tmp.path().join("broken/probe.toml"), "[probe\n").expect("write");
        let found = scan(tmp.path()).expect("scans");
        let reason = found[0].probe.as_ref().expect_err("broken").to_string();
        assert!(reason.contains("probe.toml"), "{reason}");
    }
}
