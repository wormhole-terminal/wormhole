//! Approving a Probe: what the user said yes to, the state that gives each Probe, and the copy it
//! will run from (`repo-docs/docs/probes.md`, sections 2 and 6.1).
//!
//! "On" is only a stored fact here: the fingerprint of the folder that was shown, which permissions
//! were granted, and a read-only copy of exactly those bytes. Nothing runs a Probe yet; the later
//! steps read these and call [`Snapshot::verify`] right before every spawn.
//!
//! - `store`: `probe-approvals.json`, the approvals and their grants.
//! - `state`: [`ProbeState`], a scanned Probe combined with the store.
//! - `request`: what the approval sheet shows and which confirmation it needs.
//! - `snapshot`: the read-only copy, its re-hash and its removal.
//! - `list`: the Probes of the data folder as rows for a page.
//!
//! The mutating calls ([`approve`], [`turn_off`], [`sweep`]) read the store afresh under one lock, so
//! two windows cannot overwrite each other's approvals. Must not use anything above `storage`.

mod list;
mod request;
mod snapshot;
mod state;
mod store;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;
use std::fmt;
use std::io;
use std::sync::Mutex;

pub use list::{Entry, Listed, ReadOneError, list, read_one};
pub use request::{Changes, Grants, NotApprovable, Request};
pub use snapshot::{Snapshot, VerifyError};
pub use state::ProbeState;
pub use store::{Approval, ApprovalStore, permission_id};

use crate::storage::paths::DataRoot;

/// Held while the approvals file and the copies are changed.
static WRITING: Mutex<()> = Mutex::new(());

/// Why an approval could not be recorded. Nothing was changed.
#[derive(Debug)]
pub enum ApproveError {
    /// The read-only copy could not be made.
    Snapshot(io::Error),
    /// `probe-approvals.json` could not be written.
    Store(io::Error),
}

impl fmt::Display for ApproveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Snapshot(err) => write!(f, "could not copy the Probe's files: {err}"),
            Self::Store(err) => write!(f, "could not save the approval: {err}"),
        }
    }
}

impl std::error::Error for ApproveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Snapshot(err) | Self::Store(err) => Some(err),
        }
    }
}

fn locked() -> std::sync::MutexGuard<'static, ()> {
    // A panic while holding it left files in a state `sweep` repairs; the lock itself is fine.
    WRITING.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Approve exactly what `request` showed, with `grants`, at `now` (Unix seconds). The copy is made
/// from the bytes the request holds, never from the folder again, so a file edited while the sheet
/// was open shows as `Changed` afterwards instead of being approved unseen.
///
/// The caller has already asked for whatever confirmation [`Request::confirmation`] says.
pub fn approve(root: &DataRoot, request: &Request, grants: &Grants, now: u64) -> Result<(), ApproveError> {
    let _lock = locked();
    let mut store = ApprovalStore::load(root);
    snapshot::create(root, request.folder()).map_err(ApproveError::Snapshot)?;
    let before = store.insert(request.id(), request.approval(grants, now));
    if let Err(err) = store.save() {
        match before {
            Some(previous) => {
                store.insert(request.id(), previous);
            }
            None => {
                store.remove(request.id());
                if let Err(err) = snapshot::remove(root, request.id()) {
                    tracing::warn!("removing the copy of Probe {}: {err}", request.id());
                }
            }
        }
        return Err(ApproveError::Store(err));
    }
    Ok(())
}

/// Turn `id` off: its approval and its copy are gone. Nothing to confirm, and nothing happens for a
/// Probe that was not on.
pub fn turn_off(root: &DataRoot, id: &str) -> Result<(), ApproveError> {
    let _lock = locked();
    let mut store = ApprovalStore::load(root);
    if store.remove(id).is_some() {
        store.save().map_err(ApproveError::Store)?;
    }
    snapshot::remove(root, id).map_err(ApproveError::Snapshot)
}

/// Forget what no longer has a Probe: approvals whose folder is gone from `present` (the folder names
/// the scan found, broken ones included), and copies, or leftovers of an interrupted approval, that
/// no approval owns. Call it only after a scan that listed the whole Probes folder.
pub fn sweep(root: &DataRoot, present: &BTreeSet<String>) -> Result<(), ApproveError> {
    let _lock = locked();
    let mut store = ApprovalStore::load(root);
    let gone: Vec<String> = store.ids().filter(|id| !present.contains(*id)).map(str::to_string).collect();
    for id in &gone {
        store.remove(id);
    }
    if !gone.is_empty() {
        store.save().map_err(ApproveError::Store)?;
    }
    snapshot::remove_unowned(root, &store).map_err(ApproveError::Snapshot)
}
