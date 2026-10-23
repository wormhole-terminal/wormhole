//! The POSIX half of the edge: the same calls on macOS and Linux.

pub(super) mod detached;
pub(super) mod fs;
pub(super) mod host;
pub(super) mod process;
pub(super) mod socket;
pub(super) mod user;
