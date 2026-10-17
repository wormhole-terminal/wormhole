//! How the terminal list on the left is organised and worded: the groups the person makes and the
//! order of the list (`arrange`: terminals in no group and groups side by side, `groups`), the
//! pins on top of it, the ⌘1–9 hints, what a row and a group's head say (on screen and to
//! VoiceOver), what a terminal is called (`names`), what a plain terminal's command means for its
//! row (`command`), and how the members of a split sit in the list and what a drop on a row does
//! to them (`split`, `drop`). Pure data, so it is tested here without GPUI; the app only draws it.

mod arrange;
pub mod command;
mod drop;
pub mod group_color;
pub mod groups;
pub mod hints;
mod listing;
pub mod motion;
pub mod names;
pub mod order;
pub mod place;
mod split;
pub mod spoken;
#[cfg(test)]
mod stable_tests;
mod state;
pub mod tabs;
pub mod text;
pub mod urgency;

pub use arrange::Item;
pub use drop::{Part, RowDrop, resolve_row_drop};
pub use group_color::GroupColor;
pub use groups::{Groups, TermGroup, group_key};
pub use listing::{Band, Card, Entry, ListInput, Listing, NUMBERED, WindowSidebar};
pub use order::SidebarNumbers;
pub use place::{Drop, DropInput, Landing, Target, group_drop, terminal_drop};
pub use split::{Bracket, SplitId, SplitMarks, Splits, gather_splits, split_of};
pub use state::RowState;
pub use tabs::TerminalsIn;
pub use text::display_path;
pub use urgency::Urgency;
