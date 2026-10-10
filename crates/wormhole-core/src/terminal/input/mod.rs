//! Input from the UI turned into bytes for the PTY: key presses, mouse reports
//! and dropped files, and pasting over a selection the program drew.

pub mod app_selection;
pub mod file_drop;
pub mod keys;
pub mod mouse;
