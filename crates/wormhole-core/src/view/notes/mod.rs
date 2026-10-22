//! The rules of a terminal's notes and next steps, without GPUI: the paragraph and title rules (`paragraphs`), the next
//! steps as a list that edits itself (`tasks`), the Notes tab's rows and what the keys can be on
//! (`rows`), what an item's editor does with a key (`keys`) and what closing a terminal says is
//! lost (`stake`). The app draws and wires what these decide.

pub mod keys;
pub mod paragraphs;
pub mod rows;
pub mod stake;
pub mod tasks;
