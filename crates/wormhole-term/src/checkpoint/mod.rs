//! Checkpoints: what a session host keeps on disk of a terminal so that the terminal can come back after a
//! restart of the Mac (the `RestoreAfterRestart` feature; `repo-docs/docs/session-host.md` section 8).
//!
//! A host writes them while it runs; the app reads one when the host is gone and starts a fresh shell with
//! the screen drawn above it. Nothing here starts a process or talks to the app.
//!
//! - [`manifest`]: the generation, the time, which key sealed the screen and how long it is, as `key=value`
//!   lines. Nothing of what the person did: no folder, no title, no shell.
//! - [`seal`]: the screen, sealed with ChaCha20-Poly1305 under a key the app gives the host (a per-terminal
//!   subkey of the profile's saved-text key). There is no unsealed screen and no checkpoint without a key: a
//!   host that was not given one writes nothing, and one told to stop writes nothing more.
//! - [`store`]: the directory `<data dir>/sessions.noindex/<id>/`, `0700` with `0600` files, generations
//!   written so a crash leaves the last whole one, the newest two kept, read newest first.
//! - [`vt`]: the screen as VT bytes, primary screen and content only, and the filter that lets such bytes
//!   back in; `replay`: drawing a seed (a checkpoint's screen, or saved text) before a shell starts, and the
//!   divider that ends it.
//!
//! The host prunes its own old generations and temporary files and nothing else: it never removes a checkpoint
//! directory, because a restart ends the shell and the host together and only the app knows a terminal was
//! closed on purpose. What is never stored: the program's environment (a restore derives its own), the
//! alternate screen, modes, the folder, the title and the shell (a restore takes the folder and the shell from
//! the terminal's saved record, through the project's trust, `ProjectHub::enabled`).

pub mod manifest;
pub(crate) mod replay;
pub mod seal;
pub mod store;
pub mod vt;

use std::path::{Path, PathBuf};

pub use manifest::{Manifest, ManifestError, ScreenFile};
pub(crate) use replay::is_divider_line;
pub use replay::{divider, text_seed};
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
