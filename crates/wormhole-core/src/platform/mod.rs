//! Core's edge to the operating system: the quit signal (`quit_signal`), the editors and apps a file
//! can be opened in (`open_in`) and how one opens a file at a line (`open_file`). Core's other OS code
//! moves here in time; the app's own edge is its `platform` module.
//!
//! The lowest tier: nothing here uses another domain of core.

pub mod open_file;
pub mod open_in;
pub mod quit_signal;
