//! What the saved text asks of the operating system, behind one trait: tests use a fake that
//! records the calls (and never reaches the system), and the calls can move to the app's platform
//! edge as they are.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, PoisonError};

use anyhow::{Context as _, Result, anyhow};

use super::keys;
use crate::platform::backup;
use crate::storage::secret_key::{self, Key};

/// The operating system's part in keeping the saved text.
pub(crate) trait Platform: Send + Sync {
    /// The key the text of the profile in `dir` is sealed with, kept under `account` (a random id the
    /// profile's database holds), made and kept on first use (see `keys`). An error (no Keychain,
    /// access denied, a locked Keychain) means the text isn't kept: it is never written unsealed.
    fn text_key(&self, dir: &Path, account: &str) -> Result<Key>;
    /// Throw that key away (the text was erased, or the profile deleted): any copy of the old file
    /// left anywhere (a snapshot, a backup) can't be opened any more.
    fn forget_key(&self, dir: &Path, account: &str) -> Result<()>;
    /// Keep `path` out of backups (Time Machine). A failure is only worth a warning: the file is
    /// still the user's alone.
    fn exclude_from_backup(&self, path: &Path) -> Result<()>;
}

/// The real thing: the Keychain, or the development key file (see [`secret_key::uses_dev_key`]).
pub(crate) struct System;

/// Each account's key, or why there is none, once per process: another store of the same profile,
/// or a profile opened later, never asks the Keychain (or the user) again.
static KEYS: Mutex<Option<HashMap<String, Result<Key, String>>>> = Mutex::new(None);

impl Platform for System {
    fn text_key(&self, dir: &Path, account: &str) -> Result<Key> {
        let mut cache = KEYS.lock().unwrap_or_else(PoisonError::into_inner);
        let cache = cache.get_or_insert_with(HashMap::new);
        let found = cache.entry(account.to_string()).or_insert_with(|| {
            secret_key::open(&keys::PURPOSE, dir, account).map_err(|err| format!("{err:#}"))
        });
        found.clone().map_err(|err| anyhow!(err))
    }

    fn forget_key(&self, dir: &Path, account: &str) -> Result<()> {
        if let Some(cache) = KEYS.lock().unwrap_or_else(PoisonError::into_inner).as_mut() {
            cache.remove(account);
        }
        secret_key::forget(&keys::PURPOSE, dir, account)
    }

    fn exclude_from_backup(&self, path: &Path) -> Result<()> {
        backup::exclude(path).with_context(|| format!("excluding {} from backups", path.display()))
    }
}
