//! What the saved text asks of the operating system, behind one trait: tests use a fake that
//! records the calls (and never reaches the system), and the calls can move to the app's platform
//! edge as they are.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, PoisonError};

use anyhow::{Context as _, Result, anyhow};

use super::cipher::Key;
use super::keys;
use crate::platform::backup;

/// The operating system's part in keeping the saved text.
pub trait Platform: Send + Sync {
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

/// Whether this build keeps the key in a file (`keys::DEV_KEY_FILE`) instead of the Keychain: a
/// debug build, or one with the `dev-key` feature (devctl builds, release-profile dev builds).
pub(crate) fn uses_dev_key() -> bool {
    cfg!(debug_assertions) || cfg!(feature = "dev-key")
}

/// The real thing: the Keychain, or the development key file (see [`uses_dev_key`]).
pub struct System;

/// Each account's key, or why there is none, once per process: another store of the same profile,
/// or a profile opened later, never asks the Keychain (or the user) again.
static KEYS: Mutex<Option<HashMap<String, Result<Key, String>>>> = Mutex::new(None);

impl Platform for System {
    fn text_key(&self, dir: &Path, account: &str) -> Result<Key> {
        let mut cache = KEYS.lock().unwrap_or_else(PoisonError::into_inner);
        let cache = cache.get_or_insert_with(HashMap::new);
        let found = cache.entry(account.to_string()).or_insert_with(|| {
            let key = if uses_dev_key() { keys::dev_key_file(dir) } else { keys::keychain(account) };
            key.map_err(|err| format!("{err:#}"))
        });
        found.clone().map_err(|err| anyhow!(err))
    }

    fn forget_key(&self, dir: &Path, account: &str) -> Result<()> {
        if let Some(cache) = KEYS.lock().unwrap_or_else(PoisonError::into_inner).as_mut() {
            cache.remove(account);
        }
        if uses_dev_key() { keys::forget_dev_key_file(dir) } else { keys::forget_keychain(account) }
    }

    fn exclude_from_backup(&self, path: &Path) -> Result<()> {
        backup::exclude(path).with_context(|| format!("excluding {} from backups", path.display()))
    }
}
