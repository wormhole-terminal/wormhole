//! How the terminal list moves. Each motion is a file of its own beside the list's: `list` plans what
//! moves when the list changes and springs the lines that moved to their places, `insert` and `fold` say
//! which lines came and went as a terminal opening or closing and as a group's fold, `leaving` keeps the
//! rows that went drawn until their motion is over, and `row` draws a line in the pose core's
//! `view::motion::pose` gives it, which is also what the Settings previews draw. The sidebar reaches
//! them through this module.

mod fold;
mod insert;
mod leaving;
mod list;
mod row;

pub(super) use list::{ItemKey, ListMotion};
