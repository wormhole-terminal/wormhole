//! The files each terminal changed, kept with the windows so a row's ±N survives a restart (core's `Store::save_claims`,
//! the attribution in `workspace::git_feed`; the setting is `sidebar_changes_remember`). Written whenever a window is
//! saved and its files differ from what was written last; read back when a window is restored. With the setting off
//! nothing is written and what was kept is deleted, once.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use wormhole_core::storage::store::{ClaimRecord, Store};

use super::registry::Windows;
use crate::workspace::Workspace;

/// What a profile's `Windows` holds of it.
pub(crate) struct Claims {
    keeps: AtomicBool,
    cleared: AtomicBool,
    /// The rows last written for each window, so a save that changes nothing writes nothing.
    written: Mutex<HashMap<i64, Vec<ClaimRecord>>>,
}

impl Default for Claims {
    fn default() -> Self {
        // The setting's default is on.
        Self { keeps: AtomicBool::new(true), cleared: AtomicBool::new(false), written: Mutex::default() }
    }
}

#[uniffi::export]
impl Windows {
    /// Keep (or stop keeping, and forget) the files the terminals changed: `sidebar_changes_remember`.
    pub fn set_keeps_changes(&self, keeps: bool) {
        self.claims.keeps.store(keeps, Ordering::Relaxed);
        if keeps {
            self.claims.cleared.store(false, Ordering::Relaxed);
        }
    }
}

impl Windows {
    /// Window `window`'s terminals `ids`: write their files if they changed, or delete every row once while kept no more.
    pub(super) fn keep_claims(&self, store: &mut Store, window: i64, workspace: &Workspace, ids: &[i64]) {
        if !self.claims.keeps.load(Ordering::Relaxed) {
            if !self.claims.cleared.swap(true, Ordering::Relaxed)
                && let Err(err) = store.clear_claims()
            {
                tracing::warn!("forgetting what the terminals had changed failed: {err:#}");
            }
            return;
        }
        let rows = workspace.shared().peek_model(|model| model.git_claims(ids));
        let mut written = self.claims.written.lock().unwrap_or_else(PoisonError::into_inner);
        if written.get(&window) == Some(&rows) {
            return;
        }
        match store.save_claims(ids, &rows) {
            Ok(()) => {
                written.insert(window, rows);
            }
            Err(err) => tracing::warn!("keeping what the terminals changed failed: {err:#}"),
        }
    }

    /// The rows kept for `ids`, for the window that comes back with them (none while not kept).
    pub(crate) fn claims_of(&self, window: i64, ids: &[i64]) -> Vec<ClaimRecord> {
        if !self.claims.keeps.load(Ordering::Relaxed) {
            return Vec::new();
        }
        let rows = self.store_locked().claims(ids).unwrap_or_else(|err| {
            tracing::warn!("reading what the terminals had changed failed: {err:#}");
            Vec::new()
        });
        // Just read: it is what is in the database.
        self.claims.written.lock().unwrap_or_else(PoisonError::into_inner).insert(window, rows.clone());
        rows
    }

    /// `rows` as what is kept for window `window`'s terminals `ids` (a window moved into this profile, or renumbered here).
    pub(super) fn put_claims(&self, window: i64, ids: &[i64], rows: Vec<ClaimRecord>) {
        if !self.claims.keeps.load(Ordering::Relaxed) {
            return;
        }
        match self.store_locked().save_claims(ids, &rows) {
            Ok(()) => {
                self.claims.written.lock().unwrap_or_else(PoisonError::into_inner).insert(window, rows);
            }
            Err(err) => tracing::warn!("keeping what the terminals changed failed: {err:#}"),
        }
    }

    /// Window `window` left this database: what was written for it is no longer here.
    pub(super) fn forget_written_claims(&self, window: i64) {
        self.claims.written.lock().unwrap_or_else(PoisonError::into_inner).remove(&window);
    }
}
