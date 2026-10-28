//! The folder picker at the prompt, as data: type a path to walk the folders below the terminal's, pick one
//! and its path is put at the cursor (the picker never changes directory).
//!
//! `target` says what the typed text means (which folder to list, what narrows it, how the path was
//! written), `rows` which subfolders it shows and where it matched, `collate` the order `ls` lists them in,
//! `insert` the shell-ready text a pick makes, `place` where the card goes against the cursor, and `picker`
//! all of it as one state the window drives. A folder is read with `storage::files::list_folders` by the
//! window, off the UI thread; nothing here touches the disk.

mod collate;
mod insert;
mod picker;
mod place;
mod rows;
mod target;

pub use collate::collate;
pub use insert::{inserted, shell_form};
pub use picker::Picker;
pub use place::{Card, GAP, MARGIN, Placed, place};
pub use rows::{Row, keeps, matches, rows};
pub use target::{Target, resolve};
