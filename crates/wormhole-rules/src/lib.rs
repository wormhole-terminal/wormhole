//! Checks over the repository's sources and scripts, read as text. The tests are in `tests/`; this file
//! only gives Cargo a target to hang them on.
//!
//! They live outside the app crate because Cargo builds a package's binaries for its integration tests:
//! in `crates/wormhole/tests/` they made every test run compile the whole app a second time, as a binary
//! none of them runs.
