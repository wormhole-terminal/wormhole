//! How the terminal list moves. Each motion is a file of its own beside the list's: `list` holds what is
//! under way and wraps the lines in it, `plan` decides it when the list changes, `insert` and `fold` say
//! which lines came and went as a terminal opening or closing and as a group's fold, `leaving` keeps the
//! rows that went drawn until their motion is over, `drag` opens the gap a row dragged over the list has
//! the lines make for it, `join` previews the split a row dropped on the middle of another makes, and `pose` draws a line in the pose core's
//! `view::motion::pose` gives it, which is also what the Settings previews draw. The sidebar reaches
//! them through this module.

mod drag;
mod fold;
mod insert;
mod join;
mod leaving;
mod list;
mod plan;
mod pose;

pub(crate) use join::Snap;
pub(super) use join::draw_bracket;
pub(super) use list::{ItemKey, ListMotion};
