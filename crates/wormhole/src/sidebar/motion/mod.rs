//! How the terminal list moves: today only the slide of rows that changed place ([`list`]).
//! Each motion is a file of its own beside it; the sidebar reaches them through this module.

mod list;

pub(super) use list::{ItemKey, ListMotion};
