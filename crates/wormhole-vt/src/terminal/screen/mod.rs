//! The screen the terminal edits: the cursor and the margins, erasing and inserting, rectangular
//! operations, scrolling, tab stops, the alternate screen, resizing and resetting.

pub(super) mod altscreen;
pub(super) mod cursor;
pub(super) mod edit;
pub(super) mod history;
pub(super) mod line_size;
pub(super) mod rect;
pub(super) mod reset;
pub(super) mod resize;
pub(super) mod scroll;
pub(super) mod status_line;
pub(super) mod tabs;
