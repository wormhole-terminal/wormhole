//! The Files tab as data: the tree of the folder a terminal is in, without GPUI or any I/O.
//!
//! `names` is how a name from the disk is shown and ordered, `dir` one folder's entries sorted and
//! capped, `tree` the folders opened and the rows that makes, and `menu` what the path menu at the top
//! of the tab lists. The disk is read by `storage::files::listing`; the app reads it off the main thread
//! and hands the answer to the tree.

pub mod dir;
pub mod menu;
pub mod names;
pub mod tree;

pub use dir::{Dir, ENTRY_CAP, Entry};
pub use menu::{PathMenu, Pin, PinnedFolder, on_boot_volume};
pub use tree::{Failure, Row, RowKind, Tree};
