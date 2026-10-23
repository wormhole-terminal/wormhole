//! Core's OS edge: everything core asks of the operating system, behind one neutral API, so a port
//! writes a sibling folder and nothing else changes. Each concern is a neutral file here (plain
//! functions and types, no OS names) that calls `os::…`: `fs` (modes, identity, executable bits),
//! `paths` (home, data, temp), `process`, `shell` (quoting, `-c` scripts, the login shell), `socket` and
//! `user` (the terminal layer's `sys`, shared with the session host and re-exported with what only core
//! adds), `tty` (the first-run welcome's terminal), `quit_signal`, `keychain`, `backup` (Time Machine),
//! `system` (the macOS version), `open_in` and `open_file` (editors and apps a file can be opened in, and
//! at a line). The code that talks to the system is in `unix/` (POSIX, real on macOS and Linux), `macos/`
//! (the Keychain, extended attributes, the version file) and `linux/` (stand-ins for what macOS has and
//! Linux not yet; nothing builds it, macOS being the only target). The app's own edge is its `platform`
//! module.
//!
//! The lowest tier: nothing here uses another domain of core.

pub mod backup;
pub mod fs;
pub mod keychain;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
pub mod open_file;
pub mod open_in;
pub mod paths;
pub mod process;
pub mod quit_signal;
pub mod shell;
pub mod socket;
pub mod system;
pub mod tty;
#[cfg(unix)]
mod unix;
pub mod user;
