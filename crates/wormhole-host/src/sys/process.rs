//! The host's own process: its start-up hygiene, its environment, the QoS of its threads.

use super::unix::process as unix;

#[cfg(target_os = "linux")]
use super::linux::process as os;
#[cfg(target_os = "macos")]
use super::macos::process as os;

/// A host's start-up hygiene (`session-host.md`, section 5 and spike 2): out of every folder, a
/// private umask, no standard streams, no core files (one would hold scrollback), and no
/// background policy inherited from the app that spawned it.
pub fn hygiene() {
    unix::leave_folders_and_cores();
    os::clear_background_policy();
    unix::null_stdio();
    set_user_interactive();
}

/// Put the calling thread at the highest QoS class (spike 2: an unset class may land on the
/// efficiency cores). For the main thread, the PTY's thread and the connections' thread.
pub fn set_user_interactive() {
    os::set_user_interactive();
}

/// Drop the whole environment the host was started with: the program gets exactly the one its
/// `Spawn` carries, and the host itself needs none. Call before any other thread exists.
pub fn clear_environment() {
    unix::clear_environment();
}
