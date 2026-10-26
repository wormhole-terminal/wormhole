//! Secrets the system keeps for the app: a generic password under a service and an account (the
//! macOS login Keychain). What is kept, when it is made and what a failure means is the caller's
//! (`storage::secret_key`).
//!
//! Failures are `io::Error`s whose kind says what the caller may act on: `AlreadyExists` when
//! [`add`] finds an item (another program made it first), `Unsupported` when the system has no secret
//! store; anything else (denied, cancelled, locked) carries the system's words.

use std::io;

use zeroize::Zeroizing;

#[cfg(target_os = "linux")]
use super::linux::keychain as os;
#[cfg(target_os = "macos")]
use super::macos::keychain as os;

/// The secret under `service` and `account`; `None` when there is none.
pub fn find(service: &str, account: &str) -> io::Result<Option<Zeroizing<Vec<u8>>>> {
    os::find(service, account)
}

/// Keep `secret` under `service` and `account`, unless something is kept there already
/// (`AlreadyExists`): an item that is there is never overwritten.
pub fn add(service: &str, account: &str, secret: &[u8]) -> io::Result<()> {
    os::add(service, account, secret)
}

/// Throw the secret away; one that isn't there is fine.
pub fn delete(service: &str, account: &str) -> io::Result<()> {
    os::delete(service, account)
}
