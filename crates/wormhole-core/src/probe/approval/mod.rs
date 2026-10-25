//! Approving a Probe: what the user said yes to, the state that gives each Probe, and the copy it
//! will run from (`repo-docs/docs/probes.md`, sections 2 and 6.1).
//!
//! "On" is only a stored fact here: the fingerprint of the folder that was shown, which permissions
//! were granted, and a read-only copy of exactly those bytes. Nothing runs a Probe yet; the later
//! steps read these and call [`Snapshot::verify`] right before every spawn.
//!
//! - `store`: `probe-approvals.json`, the approvals and their grants.
//! - `state`: [`ProbeState`], a scanned Probe combined with the store.
//! - `request`: what the approval sheet shows, and the [`Confirmed`] token that approving takes.
//! - `snapshot`: the read-only copy, its re-hash and its removal.
//! - `list`: the Probes of the data folder as rows for a page.
//!
//! **What a later step calls before it runs a Probe is [`runnable`], and nothing else**: it requires the
//! live state `On` (approved, the folder as approved, the copy there) and re-hashes the copy. The copy
//! of a Probe that is changed or invalid is not handed out by anything else.
//!
//! The mutating calls ([`approve`], [`turn_off`], [`sweep`]) read the store afresh under one lock, so
//! two windows cannot overwrite each other's approvals. Must not use anything above `storage`.

mod list;
mod request;
#[cfg(test)]
mod review_tests;
mod snapshot;
mod state;
mod store;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;
use std::fmt;
use std::io;
use std::sync::Mutex;

pub use list::{Entry, Listed, Listing, ReadOneError, list, read_one};
pub use request::{Changes, Confirmed, Grants, NeedsTouchId, NotApprovable, Proof, Request};
pub use snapshot::{Snapshot, VerifyError};
pub use state::ProbeState;
pub use store::{Approval, ApprovalStore, permission_id};

use crate::probe::validate::validate;
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
    /// The confirmation is for another request than the one approved.
    Unconfirmed,
}

impl fmt::Display for ApproveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Snapshot(err) => write!(f, "could not copy the Probe's files: {err}"),
            Self::Store(err) => write!(f, "could not save the approval: {err}"),
            Self::Unconfirmed => f.write_str("the confirmation is not for this request"),
        }
    }
}

impl std::error::Error for ApproveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Snapshot(err) | Self::Store(err) => Some(err),
            Self::Unconfirmed => None,
        }
    }
}

fn locked() -> std::sync::MutexGuard<'static, ()> {
    // A panic while holding it left files in a state `sweep` repairs; the lock itself is fine.
    WRITING.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Approve exactly what `request` showed, with the grants frozen in `confirmed` (which only
/// [`Request::confirm`] makes, so the Touch ID rule is kept here and not by each caller), at `now` (Unix
/// seconds). The copy is made from the bytes the request holds, never from the folder again, so a file
/// edited while the sheet was open shows as `Changed` afterwards instead of being approved unseen.
///
/// All or nothing: the new copy is staged and swapped in, the approval is saved, and only then is the old
/// copy deleted. If the approval cannot be saved the old copy is put back and the old approval stays.
pub fn approve(
    root: &DataRoot,
    request: &Request,
    confirmed: &Confirmed,
    now: u64,
) -> Result<(), ApproveError> {
    if !confirmed.is_for(request) {
        return Err(ApproveError::Unconfirmed);
    }
    let _lock = locked();
    let mut store = ApprovalStore::load(root);
    let staged = snapshot::stage(root, request.folder()).map_err(ApproveError::Snapshot)?;
    let committed = staged.commit().map_err(ApproveError::Snapshot)?;
    store.insert(request.id(), request.approval(confirmed.grants(), now));
    match store.save() {
        Ok(()) => {
            committed.finish();
            Ok(())
        }
        Err(err) => {
            committed.rollback();
            Err(ApproveError::Store(err))
        }
    }
}

/// Turn `id` off, or forget what is stored for it: its approval and its copy are gone. Nothing to confirm,
/// and nothing happens for a Probe that was not on.
pub fn turn_off(root: &DataRoot, id: &str) -> Result<(), ApproveError> {
    let _lock = locked();
    let mut store = ApprovalStore::load(root);
    if store.remove(id).is_some() {
        store.save().map_err(ApproveError::Store)?;
    }
    snapshot::remove(root, id).map_err(ApproveError::Snapshot)
}

/// Tidy what no longer has a Probe. With `present` (the folder names a scan that saw the whole Probes folder
/// found, broken ones included) approvals of the rest are forgotten too; with `None` only leftovers go:
/// copies no approval owns and what an interrupted approval left (an old copy of an approved Probe is put
/// back). Never forgets an approval on the word of a scan that did not see everything.
pub fn sweep(root: &DataRoot, present: Option<&BTreeSet<String>>) -> Result<(), ApproveError> {
    let _lock = locked();
    let mut store = ApprovalStore::load(root);
    if let Some(present) = present {
        let gone: Vec<String> = store.ids().filter(|id| !present.contains(*id)).map(str::to_string).collect();
        for id in &gone {
            store.remove(id);
        }
        if !gone.is_empty() {
            store.save().map_err(ApproveError::Store)?;
        }
    }
    snapshot::tidy(root, &store).map_err(ApproveError::Snapshot)
}

/// Why a Probe may not run now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotRunnable {
    /// There is no approval of it (or it is no Probe id).
    NotApproved,
    /// The folder is not what was approved, or the approved copy is gone.
    Changed,
    /// The folder is invalid now; the first reason.
    Invalid(String),
    /// It has code, which nothing runs yet.
    Unsupported,
    /// The copy is not what was approved.
    Copy(VerifyError),
}

impl fmt::Display for NotRunnable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotApproved => f.write_str("the Probe is not approved"),
            Self::Changed => f.write_str("the Probe changed since it was approved"),
            Self::Invalid(reason) => write!(f, "the Probe is invalid: {reason}"),
            Self::Unsupported => f.write_str("the Probe has code, which is not supported yet"),
            Self::Copy(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for NotRunnable {}

/// The one gate before a Probe's program or script is started: the folder to run it from, when the Probe
/// is `On` as it stands now and its approved copy re-hashes to what was approved. Reads the Probe's folder
/// and its copy (at most 4 MiB), so call it off the main thread, right before the spawn.
pub fn runnable(root: &DataRoot, id: &str) -> Result<Snapshot, NotRunnable> {
    let store = ApprovalStore::load(root);
    let copy = store.snapshot(root, id).ok_or(NotRunnable::NotApproved)?;
    let folder = list::read_live(root, id).map_err(|err| NotRunnable::Invalid(err.to_string()))?;
    let report = validate(&folder);
    match ProbeState::of(root, &folder, &report, &store) {
        ProbeState::On => {}
        ProbeState::Off => return Err(NotRunnable::NotApproved),
        ProbeState::Changed => return Err(NotRunnable::Changed),
        ProbeState::Invalid(reason) => return Err(NotRunnable::Invalid(reason)),
        ProbeState::Unsupported => return Err(NotRunnable::Unsupported),
    }
    copy.verify().map_err(NotRunnable::Copy)?;
    Ok(copy)
}
