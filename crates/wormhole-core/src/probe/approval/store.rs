//! `probe-approvals.json`: what the user approved, per Probe.
//!
//! One entry per approved Probe: the fingerprint it was approved under, the version, every
//! permission the manifest declared when the sheet was shown and the ones that were granted, and when,
//! and the entry's **seal** (`seal`): only Wormhole, holding the approvals' key (`key`), can write an entry
//! that reads as approved. The file also names the key's account, a random id.
//! The file is the app's own state in the data folder, so it is written atomically and owner-only
//! like `trusted-projects.json`, and read under a cap. A file that is too big or does not parse
//! reads as nothing approved (the safe side); it is set aside on the next save, not before, so a
//! failed read never costs the file. So does a file of an older format: nothing has shipped, and
//! nothing is migrated.
//!
//! An entry is **trusted** when its seal checks out; [`ApprovalStore::get`] hands out only those. An
//! entry whose seal is missing or wrong is *unsealed*: kept (the page offers to forget it) and never
//! read as an approval. When the key can't be read at all, every entry reads as not approved and none is
//! touched.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::key::{KeySource, SystemKeys};
use super::seal::SealKey;
use super::snapshot::Snapshot;
use crate::probe::manifest::is_probe_id;
use crate::probe::validate::DeclaredPermission;
use crate::storage::files::{self, LinkPolicy};
use crate::storage::paths::DataRoot;
use crate::storage::secret_key::{is_random_id, random_id};

/// Far more than 256 Probes with the largest manifests could fill; a bigger file is not ours.
pub(crate) const MAX_BYTES: u64 = 8 * 1024 * 1024;
/// The file's format; it is in every seal. 2 added the seal and the account.
const FILE_VERSION: u32 = 2;

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

/// An entry as stored: the approval and the seal Wormhole put on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Stored {
    #[serde(flatten)]
    approval: Approval,
    /// Hex; missing or empty on an entry nobody sealed.
    #[serde(default)]
    seal: String,
}

#[derive(Serialize, Deserialize)]
struct FileFormat {
    version: u32,
    /// The key's account (`key`): a random id, made at the first approval.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    account: Option<String>,
    #[serde(default)]
    probes: BTreeMap<String, Stored>,
}

impl FileFormat {
    fn empty() -> Self {
        Self { version: FILE_VERSION, account: None, probes: BTreeMap::new() }
    }
}

/// What the key said when the entries were checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Checked {
    /// Not asked: nothing to check, or a load that never needs the key ([`ApprovalStore::load_unchecked`]).
    No,
    /// Every entry was checked against the key.
    Yes,
    /// The key could not be read (denied, locked, cancelled): nothing is known, so nothing is approved.
    KeyUnavailable,
}

/// The approvals, in memory. [`super::approve`] and the calls beside it load, change and save it
/// under a lock; readers just [`load`](Self::load).
#[derive(Debug, Clone)]
pub struct ApprovalStore {
    entries: BTreeMap<String, Stored>,
    /// The ids whose seal checked out.
    trusted: BTreeSet<String>,
    account: Option<String>,
    checked: Checked,
    path: PathBuf,
    /// The file was there and could not be used; the next save sets it aside.
    damaged: bool,
}

impl ApprovalStore {
    /// The approvals of `root`'s data folder, each checked against the key; none when the file is
    /// missing, too big or broken. May block on the Keychain: off the main thread.
    pub fn load(root: &DataRoot) -> Self {
        let mut store = Self::load_unchecked(root);
        store.check(&SystemKeys);
        store
    }

    /// The entries as stored, none of them trusted, and the key never asked for: what removing an entry
    /// or tidying up needs, which must not prompt.
    pub(crate) fn load_unchecked(root: &DataRoot) -> Self {
        Self::load_capped(root.probe_approvals_file(), MAX_BYTES)
    }

    pub(crate) fn load_capped(path: PathBuf, cap: u64) -> Self {
        match read(&path, cap) {
            Ok(Some(file)) => Self::of(file, path, false),
            Ok(None) => Self::of(FileFormat::empty(), path, false),
            Err(why) => {
                tracing::warn!("{} is not used: {why}; no Probe is approved", path.display());
                Self::of(FileFormat::empty(), path, true)
            }
        }
    }

    fn of(file: FileFormat, path: PathBuf, damaged: bool) -> Self {
        let account = file.account.filter(|account| is_random_id(account));
        Self { entries: file.probes, trusted: BTreeSet::new(), account, checked: Checked::No, path, damaged }
    }

    /// The folder the key's development file would be in: beside the approvals.
    fn dir(&self) -> &Path {
        self.path.parent().unwrap_or(Path::new(""))
    }

    /// Check every entry's seal. The key is asked for only when there is an entry and an account to
    /// ask under: an empty store, or one nobody sealed (no account), costs the Keychain nothing.
    fn check(&mut self, keys: &dyn KeySource) {
        self.trusted.clear();
        self.checked = Checked::Yes;
        let Some(account) = self.account.as_deref().filter(|_| !self.entries.is_empty()) else { return };
        let key = match keys.find(self.dir(), account) {
            Ok(Some(key)) => SealKey::new(&key),
            // Deleted from the Keychain, or never made: nothing can be sealed with it.
            Ok(None) => return,
            Err(err) => {
                tracing::warn!("the Probe approvals key can't be read, so no Probe is on: {err:#}");
                self.checked = Checked::KeyUnavailable;
                return;
            }
        };
        for (id, stored) in &self.entries {
            if key.verify(FILE_VERSION, id, &stored.approval, &stored.seal) {
                self.trusted.insert(id.clone());
            } else {
                tracing::warn!("the approval of Probe {id} is not sealed by this Wormhole; it is off");
            }
        }
    }

    /// The key to seal a new approval with, made when there is none, and the account it is under
    /// (made too, when the file has none; it is saved with the next [`save`](Self::save)). Asking
    /// may block on the Keychain.
    ///
    /// A key that was deleted from the Keychain is made again under the same account; the approvals the
    /// old one sealed then read as unsealed, and the user approves them again.
    pub(super) fn seal_key(&mut self, keys: &dyn KeySource) -> anyhow::Result<SealKey> {
        let account = match &self.account {
            Some(account) => account.clone(),
            None => {
                let account = random_id()?;
                self.account = Some(account.clone());
                account
            }
        };
        Ok(SealKey::new(&keys.open(self.dir(), &account)?))
    }

    /// The approval of `id`, when it is sealed by this Wormhole's key. An unsealed one is not handed out.
    pub fn get(&self, id: &str) -> Option<&Approval> {
        if !self.trusted.contains(id) {
            return None;
        }
        self.entries.get(id).map(|stored| &stored.approval)
    }

    /// Every stored id, trusted or not.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    /// Whether anything is stored for `id`, trusted or not: what can be forgotten, and what a copy
    /// under `probes-approved/` belongs to.
    pub fn contains(&self, id: &str) -> bool {
        self.entries.contains_key(id)
    }

    /// Whether `id` has an entry that nothing here sealed (or changed since): it reads as not
    /// approved. False when the key could not be read, which is [`key_unavailable`](Self::key_unavailable)'s.
    pub fn is_unsealed(&self, id: &str) -> bool {
        self.checked == Checked::Yes && self.contains(id) && !self.trusted.contains(id)
    }

    /// The key could not be read, so every entry reads as not approved. Nothing was deleted.
    pub fn key_unavailable(&self) -> bool {
        self.checked == Checked::KeyUnavailable
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
        Snapshot::of(root, id, self.get(id)?)
    }

    /// Store `approval` for `id` with `key`'s seal on it. The one place an entry becomes trusted, and
    /// only [`super::approve`] calls it, with a [`Confirmed`](super::Confirmed) in hand.
    pub(super) fn insert_sealed(&mut self, id: &str, approval: Approval, key: &SealKey) {
        let seal = key.seal(FILE_VERSION, id, &approval);
        self.entries.insert(id.to_string(), Stored { approval, seal });
        self.trusted.insert(id.to_string());
        self.checked = Checked::Yes;
    }

    pub(super) fn remove(&mut self, id: &str) -> Option<Approval> {
        self.trusted.remove(id);
        self.entries.remove(id).map(|stored| stored.approval)
    }

    pub(super) fn save(&mut self) -> io::Result<()> {
        if self.damaged {
            set_aside(&self.path);
        }
        let file =
            FileFormat { version: FILE_VERSION, account: self.account.clone(), probes: self.entries.clone() };
        let json = serde_json::to_string_pretty(&file).map_err(io::Error::other)?;
        // The user's own data folder: a symlinked file there is their setup, and a new one is 0600.
        files::write_atomic(&self.path, json.as_bytes(), LinkPolicy::Follow)?;
        self.damaged = false;
        Ok(())
    }
}

fn read(path: &Path, cap: u64) -> Result<Option<FileFormat>, String> {
    let text = match files::read_to_string_capped(path, cap, LinkPolicy::Follow) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.to_string()),
    };
    let mut file: FileFormat = serde_json::from_str(&text).map_err(|err| err.to_string())?;
    if file.version != FILE_VERSION {
        return Err(format!("version {} is not known", file.version));
    }
    // A key becomes a folder name under `probes-approved/`: only an id is let through.
    file.probes.retain(|id, _| is_probe_id(id));
    Ok(Some(file))
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
