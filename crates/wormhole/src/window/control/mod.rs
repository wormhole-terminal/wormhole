//! What a window does when asked from outside it: the control protocol's requests about its
//! terminals (`app::control` finds the window and calls these).
//!
//! Part of the window: it must not use `app::control` or `notifications`, which call it from above.
//!
//! - `terminals`: the list, a terminal's facts, notices, text and new terminals;
//! - `reader`: a terminal's screen as a handle another thread reads (`read`, `blocks`, `output`);
//! - `input`: keys and the mouse;
//! - `run`: typing a command at a prompt;
//! - `structure`: splitting, selecting, naming and asking to close a terminal.

mod input;
mod reader;
mod run;
mod structure;
mod terminals;

pub(crate) use reader::ReaderFacts;
pub(crate) use structure::SplitPlan;
pub(crate) use terminals::TabFacts;
pub(in crate::window) use terminals::agent_id;
