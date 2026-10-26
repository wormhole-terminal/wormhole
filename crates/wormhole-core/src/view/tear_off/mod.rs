//! Tearing a terminal off the window, as geometry without GPUI: where the pointer is on the screens
//! (`screen`), where a window torn off lands and where its grow starts (`place`) and what a release
//! over the screen means (`landing`).
//!
//! Three coordinate spaces meet here, and each type says which it is in:
//! - **global**, AppKit's: points, y up, the origin at the bottom left of the primary screen, so
//!   another screen's origin may be negative (`screen::Screen`);
//! - **display**, GPUI's: the top left of one screen is (0, 0), y down. Every GPUI display has its origin
//!   there, whatever its place in the global space, and a window's bounds are in the space of the
//!   display it is on;
//! - **window**: from the top left of a window's content.
//!
//! The platform edge reads the pointer and the screens from AppKit and hands them over as values; the
//! app only decides with what comes back.

mod landing;
mod place;
mod screen;

pub use landing::{Landing, Occupant, Placed, landing, outside, top_at};
pub use place::{Placement, place};
pub use screen::{Screen, Spot, locate, to_global};
