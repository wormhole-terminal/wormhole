//! Several windows: the registry they share (`registry`), a window's frame (`frame`) and a workspace's part
//! in them (`workspace_windows`: made in a window, restored from its record, terminals taken out and in).

mod frame;
mod registry;
mod workspace_windows;

pub use frame::{DisplayRecord, FrameKind, FrameRecord, cascade_frame, fit_frame};
pub use registry::{PlannedWindow, SavedWindow, Windows};
pub use workspace_windows::{Departure, TakenOut};

#[cfg(test)]
mod tests;
