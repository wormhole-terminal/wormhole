//! Core's edge to the operating system: the quit signal (`quit_signal`), the editors and apps a file
//! can be opened in (`open_in`) and how one opens a file at a line (`open_file`). The files
//! (`fs`), processes (`process`), sockets (`socket`) and users (`user`) are the terminal layer's
//! `sys`, shared with the session host and re-exported here with what only core adds, so core has
//! one place to look. Core's other OS code moves here in time; the app's own edge is its
//! `platform` module.
//!
//! The lowest tier: nothing here uses another domain of core.

pub mod fs;
pub mod open_file;
pub mod open_in;
pub mod process;
pub mod quit_signal;
pub mod socket;
pub mod user;
