//! Several windows: the registry they share (`registry`), a window's frame (`frame`) and a workspace's part
//! in them (`workspace_windows`: made in a window, restored from its record, terminals taken out and in) and what
//! letting go of a terminal outside its window means (`tear_off`).

mod frame;
mod registry;
mod tear_off;
mod workspace_windows;

pub use frame::{DisplayRecord, FrameKind, FrameRecord, WindowMinSize, cascade_frame, fit_frame, window_min_size};
pub use registry::{PlannedWindow, SavedWindow, Windows};
pub use tear_off::{TearLanding, TearRect, TearWindow, tear_off_landing, tear_off_window};
pub use workspace_windows::{Departure, TakenOut};

#[cfg(test)]
mod tests;
