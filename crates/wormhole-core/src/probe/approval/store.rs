//! `probe-approvals.json`: what the user approved, per Probe.
//!
//! One entry per approved Probe: the fingerprint it was approved under, the version, every
//! permission the manifest declared when the sheet was shown and the ones that were granted, and when.
//! The file is the app's own state in the data folder, so it is written atomically and owner-only
//! like `trusted-projects.json`, and read under a cap. A file that is too big or does not parse
//! reads as nothing approved (the safe side); it is set aside on the next save, not before, so a
//! failed read never costs the file.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::snapshot::Snapshot;
use crate::probe::manifest::is_probe_id;
use crate::probe::validate::DeclaredPermission;
use crate::storage::files::{self, LinkPolicy};
use crate::storage::paths::DataRoot;

/// Far more than 256 Probes with the largest manifests could fill; a bigger file is not ours.
pub(crate) const MAX_BYTES: u64 = 8 * 1024 * 1024;
const FILE_VERSION: u32 = 1;

/// One permission as a stored string, `process:git` or `open:acme.atlassian.net`. The words of a
/// permission the user can read; what a later step compares to ask "was this granted".
pub fn permission_id(permission: &DeclaredPermission) -> String {
    format!("{}:{}", permission.key, permission.detail)
}

/// What was approved for one Probe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Approval {
    /// The folder's fingerprint (`probe::fingerprint`) at approval.
    pub fingerprint: String,
    /// The manifest's version at approval, for the page.
    pub version: String,
    /// Every permission the manifest declared then, as [`permission_id`]s: what a later change is
    /// compared with.
    pub declared: Vec<String>,
    /// The ones the user left on; a declared one missing here was refused.
    pub granted: Vec<String>,
    /// Unix seconds.
    pub approved_at: u64,
}

impl Approval {
    /// Whether the user granted `permission`.
    pub fn grants(&self, permission: &DeclaredPermission) -> bool {
        let id = permission_id(permission);
        self.granted.contains(&id)
    }
}

#[derive(Serialize, Deserialize)]
struct FileFormat {
    version: u32,
    #[serde(default)]
    probes: BTreeMap<String, Approval>,
}

/// The approvals, in memory. [`super::approve`] and the calls beside it load, change and save it
/// under a lock; readers just [`load`](Self::load).
#[derive(Debug, Clone, Default)]
pub struct ApprovalStore {
    entries: BTreeMap<String, Approval>,
    path: PathBuf,
    /// The file was there and could not be used; the next save sets it aside.
    damaged: bool,
}

impl ApprovalStore {
    /// The approvals of `root`'s data folder; none when the file is missing, too big or broken.
    pub fn load(root: &DataRoot) -> Self {
        Self::load_capped(root.probe_approvals_file(), MAX_BYTES)
    }

    pub(crate) fn load_capped(path: PathBuf, cap: u64) -> Self {
        match read(&path, cap) {
            Ok(Some(entries)) => Self { entries, path, damaged: false },
            Ok(None) => Self { entries: BTreeMap::new(), path, damaged: false },
            Err(why) => {
                tracing::warn!("{} is not used: {why}; no Probe is approved", path.display());
                Self { entries: BTreeMap::new(), path, damaged: true }
            }
        }
    }

    pub fn get(&self, id: &str) -> Option<&Approval> {
        self.entries.get(id)
    }

    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Whether the file could not be used when it was loaded.
    pub fn is_damaged(&self) -> bool {
        self.damaged
    }

    /// The Probe's read-only copy, when it is approved. It has not been checked, and the Probe may have
    /// changed: later steps ask [`runnable`](super::runnable) instead.
    pub(super) fn snapshot(&self, root: &DataRoot, id: &str) -> Option<Snapshot> {
        Snapshot::of(root, id, self.entries.get(id)?)
    }

    pub(super) fn insert(&mut self, id: &str, approval: Approval) -> Option<Approval> {
        self.entries.insert(id.to_string(), approval)
    }

    pub(super) fn remove(&mut self, id: &str) -> Option<Approval> {
        self.entries.remove(id)
    }

    pub(super) fn save(&mut self) -> io::Result<()> {
        if self.damaged {
            set_aside(&self.path);
        }
        let file = FileFormat { version: FILE_VERSION, probes: self.entries.clone() };
        let json = serde_json::to_string_pretty(&file).map_err(io::Error::other)?;
        // The user's own data folder: a symlinked file there is their setup, and a new one is 0600.
        files::write_atomic(&self.path, json.as_bytes(), LinkPolicy::Follow)?;
        self.damaged = false;
        Ok(())
    }
}

fn read(path: &Path, cap: u64) -> Result<Option<BTreeMap<String, Approval>>, String> {
    let text = match files::read_to_string_capped(path, cap, LinkPolicy::Follow) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.to_string()),
    };
    let file: FileFormat = serde_json::from_str(&text).map_err(|err| err.to_string())?;
    if file.version != FILE_VERSION {
        return Err(format!("version {} is not known", file.version));
    }
    // A key becomes a folder name under `probes-approved/`: only an id is let through.
    Ok(Some(file.probes.into_iter().filter(|(id, _)| is_probe_id(id)).collect()))
}

/// `<name>.bad`, replacing an older one. Best effort.
fn set_aside(path: &Path) {
    let mut aside = path.as_os_str().to_owned();
    aside.push(".bad");
    if let Err(err) = fs::rename(path, &aside)
        && err.kind() != io::ErrorKind::NotFound
    {
        tracing::warn!("setting aside {}: {err}", path.display());
    }
}
