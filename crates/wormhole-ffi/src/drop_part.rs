//! Which part of a list line the pointer is over (`wormhole_core::view::sidebar::Part`): the top quarter, the middle half or
//! the bottom quarter, held by the edge the pointer came from while the Motion feature is on (`Part::at_from`).

use wormhole_core::view::sidebar::Part;

/// The part of a line a pointer `fraction` of the way down it is over; `previous` is the part it was over a moment ago on the
/// same line (nil: none, the plain edges).
#[uniffi::export]
pub fn drop_part_at(fraction: f32, previous: Option<Part>) -> Part {
    Part::at_from(fraction, previous)
}
