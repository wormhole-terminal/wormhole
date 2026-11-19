//! Several windows: the registry they share (`registry`), a window's frame (`frame`) and a workspace's part
//! in them (`workspace_windows`: made in a window, restored from its record, terminals taken out and in) and what
//! letting go of a terminal outside its window means (`tear_off`).

mod claims;
mod frame;
mod registry;
mod rehome;
mod reissue;
mod tear_off;
mod workspace_windows;

pub use frame::{
    DisplayRecord, FrameKind, FrameRecord, WindowAnimationKind, WindowMinSize, cascade_frame, fit_frame,
    window_animation, window_min_size,
};
pub(crate) use registry::next_terminal_id;
pub use registry::{PlannedWindow, SavedWindow, Windows};
pub use tear_off::{
    TearGrow, TearLanding, TearPlaced, TearRect, TearWindow, tear_off_grow, tear_off_grow_ms,
    tear_off_landing, tear_off_place,
};
pub use workspace_windows::{Departure, TakenOut};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod texts_tests;
