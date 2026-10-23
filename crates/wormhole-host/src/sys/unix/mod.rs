//! The POSIX half of the edge: the same calls on macOS and Linux.

pub(super) mod fs;
pub(super) mod process;
pub(super) mod ready;
pub(super) mod signals;
