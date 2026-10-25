//! The Probes of the data folder as rows for a page: each with its state, and enough about it to
//! show without keeping its files.

use std::fs;

use super::request::{NotApprovable, Request};
use super::state::ProbeState;
use super::store::ApprovalStore;
use crate::probe::manifest::is_probe_id;
use crate::probe::read::ProbeFolder;
use crate::probe::scan::{ScanError, Scanned, TooManyEntries, load, scan_data};
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
}

/// Every folder of `root`'s Probes folder with its state. Reads each Probe once and keeps none of
/// the files; the call reads up to 2 MiB per Probe, so run it off the main thread.
pub fn list(root: &DataRoot) -> Result<Vec<Entry>, TooManyEntries> {
    let store = ApprovalStore::load(root);
    Ok(scan_data(root)?.into_iter().map(|scanned| entry(scanned, &store)).collect())
}

fn entry(scanned: Scanned, store: &ApprovalStore) -> Entry {
    match scanned.probe {
        Err(err) => Entry { folder: scanned.name, probe: None, state: ProbeState::Invalid(err.to_string()) },
        Ok(folder) => {
            let report = validate(&folder);
            let state = ProbeState::of(&folder, &report, store);
            let info = &folder.manifest.probe;
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
            Entry { folder: scanned.name, probe: Some(listed), state }
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

/// Read the Probe `id` once, from its folder, for the approval sheet. The request holds those
/// bytes, and approving it copies them.
pub fn read_one(root: &DataRoot, id: &str) -> Result<Request, ReadOneError> {
    if !is_probe_id(id) {
        return Err(ReadOneError::Unreadable(ScanError::BadName));
    }
    let path = root.probes_dir().join(id);
    // A link in the Probes folder is never a Probe (`scan` lists it with that reason).
    if fs::symlink_metadata(&path).is_ok_and(|meta| meta.file_type().is_symlink()) {
        return Err(ReadOneError::Unreadable(ScanError::Link));
    }
    let folder: ProbeFolder = load(id.to_string(), &path).probe.map_err(ReadOneError::Unreadable)?;
    Request::new(folder, &ApprovalStore::load(root)).map_err(ReadOneError::NotApprovable)
}
