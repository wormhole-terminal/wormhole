//! Approving a Probe: what the user said yes to, the state that gives each Probe, and the copy it
//! will run from (`repo-docs/docs/probes.md`, sections 2 and 6.1).
//!
//! "On" is only a stored fact here: the fingerprint of the folder that was shown, which permissions
//! were granted, and a read-only copy of exactly those bytes. Nothing runs a Probe yet; the later
//! steps read these and call [`Snapshot::verify`] right before every spawn.
//!
//! - `store`: `probe-approvals.json`, the approvals and their grants, each with its seal.
//! - `seal` and `key`: the HMAC that makes an entry read as approved only when Wormhole wrote it, and
//!   where its key comes from (a key per approval, in the login Keychain; a dev-only file in a development
//!   build). What the seal does not stop is in `key`'s notes.
//! - `state`: [`ProbeState`], a scanned Probe combined with the store.
//! - `request`: what the approval sheet shows, and the [`Confirmed`] token that approving takes.
//! - `snapshot`: the read-only copy, its re-hash and its removal.
//! - `list`: the Probes of the data folder as rows for a page.
//!
//! **What a later step calls before it runs a Probe is [`runnable`], and nothing else**: it requires the
//! live state `On` (approved, the folder as approved, the copy there) and re-hashes the copy. The copy
//! of a Probe that is changed or invalid is not handed out by anything else.
//!
//! **An approval counts only with Wormhole's seal.** [`approve`] is the one call that seals, and it takes a
//! [`Confirmed`], so a seal comes only from the Touch ID or click path. An entry that is unsealed, or whose
//! seal does not match, reads as `Off` and is never [`runnable`]; when the key cannot be read at all, every
//! entry reads as `Off` and [`Listing::key_problem`] says why. Nothing is deleted on either account. An
//! approval that is withdrawn or replaced has its key deleted, so copying the old entry back gains nothing.
//!
//! **Threads.** The key is read in [`list`], [`read_one`], [`approve`] and [`runnable`], and reading it may
//! block or show the Keychain's prompt: call those off the main thread, as the app does. [`turn_off`] and
//! [`sweep`] never read one (they only delete the keys of what they forget), so tidying up never prompts.
//!
//! The mutating calls ([`approve`], [`turn_off`], [`sweep`]) read the store afresh under one lock, so
//! two windows cannot overwrite each other's approvals. Must not use anything above `storage`.

mod key;
mod list;
mod request;
#[cfg(test)]
mod review_tests;
mod seal;
#[cfg(test)]
mod seal_tests;
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
pub use store::{Approval, ApprovalStore, KeyProblem, permission_id};

use key::{KeySource as _, SystemKeys};

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
    /// The key the approval is sealed with could not be made (the Keychain said no).
    Key(anyhow::Error),
    /// A development build on the real data folder, where it keeps no keys: set `WORMHOLE_DATA_DIR`.
    InRealDataFolder,
}

impl fmt::Display for ApproveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Snapshot(err) => write!(f, "could not copy the Probe's files: {err}"),
            Self::Store(err) => write!(f, "could not save the approval: {err}"),
            Self::Unconfirmed => f.write_str("the confirmation is not for this request"),
            Self::Key(err) => write!(f, "could not get the key to seal the approval with: {err:#}"),
            Self::InRealDataFolder => f.write_str(
                "a development build keeps no Probe approvals in the real data folder (set WORMHOLE_DATA_DIR)",
            ),
        }
    }
}

impl std::error::Error for ApproveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Snapshot(err) | Self::Store(err) => Some(err),
            Self::Unconfirmed | Self::InRealDataFolder => None,
            Self::Key(err) => Some(err.as_ref()),
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
/// All or nothing: a new key is made, the new copy is staged and swapped in, the approval is saved, and only
/// then are the old copy and the old approval's key deleted. If the approval cannot be saved the old copy is
/// put back, the old approval stays and the new key is deleted.
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
    // The entries as stored: the others keep the seals they have (only this one is sealed now), and the
    // key comes first, so a Keychain that says no changes nothing, not even a copy.
    let mut store = ApprovalStore::load_unchecked(root);
    let (account, seal) = store.new_key(&SystemKeys).map_err(|err| match KeyProblem::of(&err) {
        KeyProblem::InRealDataFolder => ApproveError::InRealDataFolder,
        KeyProblem::Unreadable => ApproveError::Key(err),
    })?;
    // From here a failure leaves the new key to no one: it is deleted again.
    let dir = store.dir();
    let drop_key = |err: ApproveError| {
        if let Err(forget) = SystemKeys.forget(&dir, &account) {
            tracing::warn!("deleting the key of an approval that was not saved: {forget:#}");
        }
        err
    };
    let staged =
        snapshot::stage(root, request.folder()).map_err(|err| drop_key(ApproveError::Snapshot(err)))?;
    let committed = staged.commit().map_err(|err| drop_key(ApproveError::Snapshot(err)))?;
    store.insert_sealed(request.id(), request.approval(confirmed.grants(), now), account.clone(), &seal);
    match store.save() {
        Ok(()) => {
            committed.finish();
            store.retire(&SystemKeys);
            Ok(())
        }
        Err(err) => {
            committed.rollback();
            Err(drop_key(ApproveError::Store(err)))
        }
    }
}

/// Turn `id` off, or forget what is stored for it: its approval and its copy are gone. Nothing to confirm,
/// and nothing happens for a Probe that was not on.
pub fn turn_off(root: &DataRoot, id: &str) -> Result<(), ApproveError> {
    let _lock = locked();
    let mut store = ApprovalStore::load_unchecked(root);
    if store.remove(id).is_some() {
        store.save().map_err(ApproveError::Store)?;
    }
    // Its key goes once the file no longer names it (and any a write before left behind).
    store.retire(&SystemKeys);
    snapshot::remove(root, id).map_err(ApproveError::Snapshot)
}

/// Tidy what no longer has a Probe. With `present` (the folder names a scan that saw the whole Probes folder
/// found, broken ones included) approvals of the rest are forgotten too; with `None` only leftovers go:
/// copies no approval owns and what an interrupted approval left (an old copy of an approved Probe is put
/// back). Never forgets an approval on the word of a scan that did not see everything.
pub fn sweep(root: &DataRoot, present: Option<&BTreeSet<String>>) -> Result<(), ApproveError> {
    let _lock = locked();
    let mut store = ApprovalStore::load_unchecked(root);
    if let Some(present) = present {
        let gone: Vec<String> = store.ids().filter(|id| !present.contains(*id)).map(str::to_string).collect();
        for id in &gone {
            store.remove(id);
        }
        if !gone.is_empty() {
            store.save().map_err(ApproveError::Store)?;
        }
    }
    store.retire(&SystemKeys);
    key::remove_stray_dev_keys(&store.dir());
    snapshot::tidy(root, &store).map_err(ApproveError::Snapshot)
}

/// Why a Probe may not run now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotRunnable {
    /// There is no approval of it (or it is no Probe id).
    NotApproved,
    /// There is an approval, but this Wormhole did not seal it (or it was changed since).
    Unsealed,
    /// The key the approvals are checked with could not be read.
    KeyUnavailable,
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
            Self::Unsealed => f.write_str("the Probe's approval was not made by this Wormhole"),
            Self::KeyUnavailable => f.write_str("the key the approvals are checked with can't be read"),
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
/// and its copy (at most 4 MiB), and the key (which may prompt), so call it off the main thread, right
/// before the spawn.
pub fn runnable(root: &DataRoot, id: &str) -> Result<Snapshot, NotRunnable> {
    let store = ApprovalStore::load(root);
    if store.key_unavailable() {
        return Err(NotRunnable::KeyUnavailable);
    }
    if store.is_unsealed(id) {
        return Err(NotRunnable::Unsealed);
    }
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
