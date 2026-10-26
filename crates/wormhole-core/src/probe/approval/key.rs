//! Where the key the approvals are sealed with comes from.
//!
//! One key for the whole data folder, kept as `storage::secret_key` keeps any key: the login Keychain in
//! a release build, a file marked dev-only in a development build (and none in the real data folder). Its
//! account is the random id `probe-approvals.json` holds (`store`), the way saved text keeps its own in
//! its database's `meta`.
//!
//! **A forged account gains nothing.** Whoever can write `probe-approvals.json` can name any account in
//! it, but an account only picks which key to check seals against: a key the writer doesn't have (they
//! can't read it without the Keychain asking the user), or none, in which case [`KeySource::find`] says
//! so and every seal fails. A made-up account is never turned into an item by a read.
//!
//! **Reading a key may block**, and shows the system's prompt when the Keychain is locked or another
//! program asks: never call it on the main thread. Every caller here already runs on the background
//! executor (listing, reading a Probe for its sheet, approving, [`runnable`](super::runnable)).
//! [`sweep`](super::sweep) and [`turn_off`](super::turn_off) never ask for it: tidying up must not prompt.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, PoisonError};

use anyhow::Result;

use crate::storage::secret_key::{self, Key, Purpose};

/// The approvals' key: the Keychain item's service, and the development build's file in the data folder.
pub(super) const PURPOSE: Purpose = Purpose {
    what: "Probe approvals key",
    keeps: "Probe approvals",
    keychain_service: "Wormhole Probe approvals",
    dev_key_file: "probe-approvals-key.dev-only",
};

/// How the approvals get their key: [`SystemKeys`]. A test puts its own in front of it (`with_keys`) to
/// make a key fail, change or panic when read.
pub(crate) trait KeySource: Send + Sync {
    /// The key kept under `account`, or `None` when there is none yet. Never makes one, so checking a
    /// seal leaves nothing behind. An error: denied, cancelled, a locked Keychain.
    fn find(&self, dir: &Path, account: &str) -> Result<Option<Key>>;
    /// The same key, made when there is none. Only approving asks for this: a Keychain item whose key
    /// was deleted gets a new one, and the approvals sealed with the old key then read as unsealed, so
    /// the user approves again.
    fn open(&self, dir: &Path, account: &str) -> Result<Key>;
}

/// The Keychain (or the development key file). A key that was read is kept for the process, so the
/// Keychain is asked once, not at every listing; a failure is not kept, so unlocking the Keychain and
/// opening the page again works.
pub(crate) struct SystemKeys;

static KEYS: Mutex<Option<HashMap<String, Key>>> = Mutex::new(None);

fn cached(account: &str) -> Option<Key> {
    KEYS.lock().unwrap_or_else(PoisonError::into_inner).as_ref()?.get(account).cloned()
}

fn keep(account: &str, key: &Key) {
    KEYS.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get_or_insert_with(HashMap::new)
        .insert(account.to_string(), key.clone());
}

impl KeySource for SystemKeys {
    fn find(&self, dir: &Path, account: &str) -> Result<Option<Key>> {
        #[cfg(test)]
        if let Some(keys) = tests_keys::current() {
            return keys.find(dir, account);
        }
        if let Some(key) = cached(account) {
            return Ok(Some(key));
        }
        let found = secret_key::find(&PURPOSE, dir, account)?;
        if let Some(key) = &found {
            keep(account, key);
        }
        Ok(found)
    }

    fn open(&self, dir: &Path, account: &str) -> Result<Key> {
        #[cfg(test)]
        if let Some(keys) = tests_keys::current() {
            return keys.open(dir, account);
        }
        if let Some(key) = cached(account) {
            return Ok(key);
        }
        let key = secret_key::open(&PURPOSE, dir, account)?;
        keep(account, &key);
        Ok(key)
    }
}

/// The seam for tests: a key source that stands in for the system's on this thread, so the public calls
/// ([`list`](super::list), [`approve`](super::approve), [`sweep`](super::sweep), ...) can be run against a key
/// that fails, changes or panics when read.
#[cfg(test)]
pub(super) mod tests_keys {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::KeySource;

    thread_local! {
        static CURRENT: RefCell<Option<Rc<dyn KeySource>>> = const { RefCell::new(None) };
    }

    pub(super) fn current() -> Option<Rc<dyn KeySource>> {
        CURRENT.with(|current| current.borrow().clone())
    }

    /// Run `body` with `keys` as the approvals' key source on this thread.
    pub(in crate::probe) fn with_keys<R>(keys: impl KeySource + 'static, body: impl FnOnce() -> R) -> R {
        struct Restore(Option<Rc<dyn KeySource>>);
        impl Drop for Restore {
            fn drop(&mut self) {
                CURRENT.with(|current| *current.borrow_mut() = self.0.take());
            }
        }
        let _restore = Restore(CURRENT.with(|current| current.borrow_mut().replace(Rc::new(keys))));
        body()
    }
}
