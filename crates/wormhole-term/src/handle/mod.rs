//! The methods of [`Pty`](crate::Pty), one file for each concern: starting and closing it
//! (`lifecycle`), its size (`size`), input (`input`), what the screen shows (`view`), selecting
//! (`selection`), the text of the screen and the history (`scrollback`), what the program reports
//! (`program`) and the command blocks (`commands`). The struct is the crate root's, with its two
//! ways of running a program; the hosted ones' methods are `client`'s.

mod commands;
mod input;
mod lifecycle;
mod program;
mod scrollback;
mod selection;
mod size;
mod view;

pub use self::input::KeySent;
pub use self::scrollback::ScrollbackSource;
