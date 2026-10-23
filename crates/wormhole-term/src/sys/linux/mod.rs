//! Linux's half of the edge: `/proc` and `SO_PEERCRED`. Nothing builds it yet (macOS is the only
//! target), so it is kept as it was when it was written.

pub(super) mod process;
pub(super) mod socket;
