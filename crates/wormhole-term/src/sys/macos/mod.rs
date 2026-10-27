//! Apple's half of the edge: libproc and sysctl for processes and their sockets, `getpeereid`, the Security framework
//! for code signatures.

pub(super) mod codesign;
mod ports;
mod procargs;
pub(super) mod process;
mod signature;
pub(super) mod socket;
