//! What a window does when asked from outside it: the control protocol's requests about its
//! terminals (`app::control` finds the window and calls these).
//!
//! Part of the window: it must not use `app::control` or `notifications`, which call it from above.
//!
//! - `terminals`: the list, a terminal's facts, notices, text and new terminals.

mod terminals;

pub(in crate::window) use terminals::agent_id;
