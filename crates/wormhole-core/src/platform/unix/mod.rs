//! The POSIX half of core's edge: the same calls on macOS and Linux.

pub(super) mod fs;
pub(super) mod process;
pub(super) mod shell;
pub(super) mod signal;
pub(super) mod tty;
