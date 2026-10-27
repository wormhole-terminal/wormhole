//! The terminal layer's OS edge for what is not the PTY: who a user is, the sockets a session host is
//! reached on, a peer's code signature, file modes, processes, where the home and the temporary folders are and this machine's
//! name. Each concern is a neutral file here (plain functions and types, no OS names) that calls
//! `os::…`; the code that talks to the system is in `unix/` (POSIX, real on macOS and Linux), `macos/`
//! (libproc) and `linux/` (`/proc`, `SO_PEERCRED`). A port to another system writes a sibling folder and
//! the neutral files pick it.
//!
//! The shared primitives sit in the lowest crate with OS code: the session host links this crate
//! and never `wormhole-core`, and core re-exports what it uses through its own `platform`, so a
//! user id, a peer check or a file mode is written once for all three.

pub mod codesign;
pub mod detached;
pub mod fs;
pub mod host;
pub mod paths;
pub mod process;
pub mod socket;
pub mod user;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(unix)]
mod unix;
