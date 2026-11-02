//! Checkpoints: what a session host keeps on disk of a terminal so that the terminal can come back after a
//! restart of the Mac (the `RestoreAfterRestart` feature; `repo-docs/docs/session-host.md` section 8).
//!
//! A host writes them while it runs; the app reads one when the host is gone and starts a fresh shell with
//! the screen drawn above it. Nothing here starts a process or talks to the app.
//!
//! - [`manifest`]: the folder, the shell's key, the title, the size and the time, as `key=value` lines;
//!   readable without any key.
//! - [`seal`]: the screen, sealed with ChaCha20-Poly1305 under a key the app gives the host (a per-terminal
//!   subkey of the profile's saved-text key). There is no unsealed screen: without a key only the manifest
//!   is written.
//! - [`store`]: the directory `<data dir>/sessions.noindex/<id>/`, `0700` with `0600` files, generations
//!   written so a crash leaves the last whole one, the newest two kept, read newest first.
//! - [`vt`]: the screen as VT bytes, primary screen and content only, and the filter that lets such bytes
//!   back in.
//!
//! What is never stored: the program's environment (a restore derives its own), the alternate screen,
//! modes, and anything while no key is held. The folder and the shell are *not trusted* when read: the app
//! checks them against the project's trust (`ProjectHub::enabled`) at restore.

pub mod capture;
pub mod manifest;
pub mod seal;
pub mod store;
pub mod vt;

use std::path::{Path, PathBuf};

pub use capture::{Capture, capture};
pub use manifest::{Manifest, ManifestError, ScreenFile};
pub use seal::{Key, KeyId};
pub use store::{Issue, KEEP, ReadError, Restored, Snapshot, Store, Written, forget, read_latest};
pub use vt::{Limits, safe_replay};

/// The file layout this build writes.
pub const FORMAT: u32 = 1;
/// The lowest reader level that understands what this build writes (`min_reader=`). A later format that
/// only adds keys keeps it; one that changes what a known key means raises it, and an older reader leaves
/// the checkpoint alone.
pub const MIN_READER: u32 = 1;
/// The reader level of this build.
pub const READER: u32 = 1;

/// `<data dir>/sessions.noindex`: where every terminal's checkpoint directory is. The `.noindex` suffix
/// keeps Spotlight out; the app also keeps the folder out of backups when it makes it.
pub fn sessions_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("sessions.noindex")
}

/// [`sessions_dir`] for the data directory a hosts directory (`<data dir>/hosts`) is in.
pub fn sessions_dir_of_hosts(hosts_dir: &Path) -> PathBuf {
    sessions_dir(hosts_dir.parent().unwrap_or(hosts_dir))
}

#[cfg(test)]
mod tests;
