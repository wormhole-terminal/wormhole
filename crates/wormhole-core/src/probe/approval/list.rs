//! The Probes of the data folder as rows for a page: each with its state, and enough about it to
//! show without keeping its files.

use std::collections::BTreeSet;
use std::fs;

use super::request::{NotApprovable, Request};
use super::snapshot::VerifyError;
use super::state::ProbeState;
use super::store::ApprovalStore;
use crate::probe::manifest::is_probe_id;
use crate::probe::read::ProbeFolder;
use crate::probe::scan::{Coverage, ScanError, Scanned, TooManyEntries, load, scan_covering};
use crate::probe::validate::{Summary, validate};
use crate::storage::paths::DataRoot;

/// What a row shows about a Probe whose manifest read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub summary: Summary,
    pub files: usize,
    /// What it is approved under now, or would be.
    pub fingerprint: String,
}

/// One folder of the Probes folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The folder's name.
    pub folder: String,
    /// `None` when the manifest could not be read: the state says why.
    pub probe: Option<Listed>,
    pub state: ProbeState,
    /// An approval of it is stored, whatever its state: it can be forgotten.
    pub approved: bool,
    /// What is stored for it was not sealed by this Wormhole (or was changed since): it is off, "not
    /// approved by this Wormhole". Not set when the key could not be read ([`Listing::key_unavailable`]).
    pub unsealed: bool,
    /// The approved copy is not what was approved (the state is `Changed` then).
    pub copy_problem: Option<VerifyError>,
}

impl Entry {
    /// Whether the page offers to forget the stored approval: there is one, and the Probe is not simply `On`
    /// (turning it off is the switch's).
    pub fn can_forget(&self) -> bool {
        self.approved && !self.state.is_on()
    }
}

/// The Probes folder as it was read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    pub entries: Vec<Entry>,
    pub coverage: Coverage,
    /// The key the approvals are sealed with could not be read (denied, locked, cancelled), so every
    /// Probe is off for now. Nothing was deleted.
    pub key_unavailable: bool,
}

impl Listing {
    /// The folder names present, for [`sweep`](super::sweep) to forget approvals of the rest: only when the
    /// whole folder was seen. A folder that is missing or could not be fully read says nothing about which
    /// Probes are gone, so it gives none and `sweep` removes leftovers only.
    pub fn present(&self) -> Option<BTreeSet<String>> {
        (self.coverage == Coverage::Whole)
            .then(|| self.entries.iter().map(|entry| entry.folder.clone()).collect())
    }
}

/// Every folder of `root`'s Probes folder with its state. Reads each Probe once and keeps none of the
/// files, and re-hashes the copy of each Probe that is on; the call reads up to 4 MiB per Probe, so run
/// it off the main thread.
pub fn list(root: &DataRoot) -> Result<Listing, TooManyEntries> {
    let store = ApprovalStore::load(root);
    let scan = scan_covering(&root.probes_dir())?;
    Ok(Listing {
        entries: scan.found.into_iter().map(|scanned| entry(root, scanned, &store)).collect(),
        coverage: scan.coverage,
        key_unavailable: store.key_unavailable(),
    })
}

fn entry(root: &DataRoot, scanned: Scanned, store: &ApprovalStore) -> Entry {
    let approved = store.contains(&scanned.name);
    let unsealed = store.is_unsealed(&scanned.name);
    match scanned.probe {
        Err(err) => Entry {
            folder: scanned.name,
            probe: None,
            state: ProbeState::Invalid(err.to_string()),
            approved,
            unsealed,
            copy_problem: None,
        },
        Ok(folder) => {
            let report = validate(&folder);
            let mut state = ProbeState::of(root, &folder, &report, store);
            let info = &folder.manifest.probe;
            // On means the copy is what was approved; a copy that is not makes it need approving again.
            let copy_problem = (state == ProbeState::On)
                .then(|| store.snapshot(root, &info.id).and_then(|copy| copy.verify().err()))
                .flatten();
            if copy_problem.is_some() {
                state = ProbeState::Changed;
            }
            let listed = Listed {
                id: info.id.clone(),
                name: info.name.clone(),
                version: info.version.clone(),
                description: info.description.clone(),
                author: info.author.clone(),
                summary: report.summary,
                files: folder.files.len(),
                fingerprint: folder.fingerprint(),
            };
            Entry { folder: scanned.name, probe: Some(listed), state, approved, unsealed, copy_problem }
        }
    }
}

/// Why one Probe could not be read for its sheet.
#[derive(Debug)]
pub enum ReadOneError {
    Unreadable(ScanError),
    NotApprovable(NotApprovable),
}

impl std::fmt::Display for ReadOneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreadable(err) => err.fmt(f),
            Self::NotApprovable(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for ReadOneError {}

/// The Probe `id` read from its folder, once.
pub(super) fn read_live(root: &DataRoot, id: &str) -> Result<ProbeFolder, ScanError> {
    if !is_probe_id(id) {
        return Err(ScanError::BadName);
    }
    let path = root.probes_dir().join(id);
    // A link in the Probes folder is never a Probe (`scan` lists it with that reason).
    if fs::symlink_metadata(&path).is_ok_and(|meta| meta.file_type().is_symlink()) {
        return Err(ScanError::Link);
    }
    load(id.to_string(), &path).probe
}

/// Read the Probe `id` once, from its folder, for the approval sheet. The request holds those
/// bytes, and approving it copies them.
pub fn read_one(root: &DataRoot, id: &str) -> Result<Request, ReadOneError> {
    let folder = read_live(root, id).map_err(ReadOneError::Unreadable)?;
    // The earlier approval the sheet diffs against is only one that is sealed.
    Request::new(folder, &ApprovalStore::load(root)).map_err(ReadOneError::NotApprovable)
}
