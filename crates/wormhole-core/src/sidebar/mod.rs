//! How the terminal list on the left is organised and worded: grouping into folder bands, the
//! order the user gives it and the pins on top of it, the Show filter and search, the ⌘1–9
//! hints, what a row and a band say (on screen and to VoiceOver), what a terminal is called
//! (`names`), what a plain terminal's command means for its row (`command`), which part of a band
//! gives way first, and how the members of a split sit in the list and what a drop on a row does
//! to them (`split`, `drop`). Pure data, so it is tested here without GPUI; the app only
//! draws it.

pub mod command;
mod drop;
mod filter;
mod git;
mod group;
pub mod hints;
mod listing;
pub mod names;
pub mod order;
mod split;
pub mod spoken;
pub mod text;

pub use drop::{Part, RowDrop, resolve_row_drop};
pub use filter::{Filter, RowState, Searchable, Show, hidden_label};
pub use git::{GroupGit, SessionGit, group_git};
pub use group::{Group, Grouping, NO_FOLDER, display_path, group_by_folder, place_in_folder};
pub use listing::{Band, Entry, ListInput, Listing, NUMBERED, WindowSidebar, folder_key};
pub use split::{Bracket, SplitId, SplitMarks, Splits, gather_splits, split_of};
