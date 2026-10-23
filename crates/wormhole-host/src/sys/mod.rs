//! What the host asks of the system directly: its own hygiene at start-up, the readiness pipe,
//! signals, a connection's code signature, the QoS class of its threads. Each concern is a neutral
//! file here (plain functions and types, no OS names) that calls `os::…`; the code that talks to
//! the system is in `unix/` (POSIX), `macos/` (QoS, Darwin priority, the Security framework) and
//! `linux/` (what macOS does here and Linux has no equivalent of yet). Each FFI call sits in a small
//! function of its own with the reason it is sound.
//!
//! The user, the peer check, sockets and file modes are `wormhole_term::sys`'s: the host links that
//! crate and never core.

pub(crate) mod codesign;
pub(crate) mod fs;
pub(crate) mod process;
pub(crate) mod ready;
pub(crate) mod signals;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(unix)]
mod unix;
