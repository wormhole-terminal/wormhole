//! Where the keys the approvals are sealed with come from.
//!
//! **A key per approval.** Each entry of `probe-approvals.json` names an account of its own, a random id,
//! and its key is kept as `storage::secret_key` keeps any key: the login Keychain in a release build, a
//! file per account marked dev-only in a development build (and none in the real data folder).
//! Withdrawing or replacing an approval deletes its key (`store`'s retiring), so an old entry copied back
//! into the file, with its copy, finds no key and reads as not approved: no replay.
//!
//! **What this does not stop.** The login Keychain can't tell Wormhole's item from one another program
//! made, so a program that can run commands as the user (`security add-generic-password`) can put a key of
//! its own under an account it writes into the file, and seal an entry with it. The seal keeps out a
//! program that can only write files, as a prompt-injected agent most easily does; one that runs commands
//! as the user can also edit the shell's startup files, and no seal here stops that
//! (`repo-docs/docs/probes.md` 6.1).
//!
//! **Reading a key may block**, and shows the system's prompt when the Keychain is locked or another
//! program asks: never call it on the main thread. Every caller here already runs on the background
//! executor (listing, reading a Probe for its sheet, approving, [`runnable`](super::runnable)).
//! [`sweep`](super::sweep) and [`turn_off`](super::turn_off) never read one, they only delete, so tidying
//! up never asks for access to a key (a locked Keychain may still ask to be unlocked, as a listing does).
//! Nothing is kept in memory between calls, so a key deleted from the Keychain counts at the next read;
//! the price is a read per approval at each listing.

use std::path::Path;

use anyhow::Result;

use crate::storage::secret_key::{self, Key, Purpose};

/// The approvals' keys: the Keychain items' service, and the development build's folder in the data folder.
pub(super) const PURPOSE: Purpose = Purpose {
    what: "Probe approval key",
    keeps: "Probe approvals",
    keychain_service: "Wormhole Probe approvals",
    dev_key_file: "probe-approval-keys.dev-only",
    dev_key_per_account: true,
};

/// How the approvals get their keys: [`SystemKeys`]. A test puts its own in front of it (`with_keys`) to
/// make a key fail, change or panic when read.
pub(crate) trait KeySource: Send + Sync {
    /// The key kept under `account`, or `None` when there is none. Never makes one, so checking a seal
    /// leaves nothing behind. An error: denied, cancelled, a locked Keychain.
    fn find(&self, dir: &Path, account: &str) -> Result<Option<Key>>;
    /// The key under `account`, made when there is none: approving asks for it under a fresh random
    /// account, so it is a new one.
    fn open(&self, dir: &Path, account: &str) -> Result<Key>;
    /// Delete the key under `account`; one that isn't there is fine. Never reads it, so it never asks for access to it.
    fn forget(&self, dir: &Path, account: &str) -> Result<()>;
}

/// The Keychain (or the development key files).
pub(crate) struct SystemKeys;

impl KeySource for SystemKeys {
    fn find(&self, dir: &Path, account: &str) -> Result<Option<Key>> {
        #[cfg(test)]
        if let Some(keys) = tests_keys::current() {
            return keys.find(dir, account);
        }
        secret_key::find(&PURPOSE, dir, account)
    }

    fn open(&self, dir: &Path, account: &str) -> Result<Key> {
        #[cfg(test)]
        if let Some(keys) = tests_keys::current() {
            return keys.open(dir, account);
        }
        secret_key::open(&PURPOSE, dir, account)
    }

    fn forget(&self, dir: &Path, account: &str) -> Result<()> {
        #[cfg(test)]
        if let Some(keys) = tests_keys::current() {
            return keys.forget(dir, account);
        }
        secret_key::forget(&PURPOSE, dir, account)
    }
}

/// Format 2's one key for every approval, in a development build: nothing reads it any more.
const FORMAT_2_DEV_KEY: Purpose =
    Purpose { dev_key_file: "probe-approvals-key.dev-only", dev_key_per_account: false, ..PURPOSE };

/// A release build deletes the development build's key folder it finds beside the approvals: those keys
/// protect nothing. Format 2's key file goes in any build.
pub(super) fn remove_stray_dev_keys(dir: &Path) {
    if !secret_key::uses_dev_key() {
        secret_key::remove_stray_dev_key(&PURPOSE, dir);
    }
    secret_key::remove_stray_dev_key(&FORMAT_2_DEV_KEY, dir);
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
