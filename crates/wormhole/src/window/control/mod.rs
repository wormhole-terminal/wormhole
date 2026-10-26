//! What a window does when asked from outside it: the control protocol's requests about its
//! terminals (`app::control` finds the window and calls these).
//!
//! Part of the window: it must not use `app::control` or `notifications`, which call it from above.
//!
//! - `terminals`: the list, a terminal's facts, notices, text and new terminals;
//! - `reader`: a terminal's screen as a handle another thread reads (`read`, `blocks`, `output`);
//! - `input`: keys and the mouse.

mod input;
mod reader;
mod terminals;

pub(crate) use reader::ReaderFacts;
pub(in crate::window) use terminals::agent_id;
