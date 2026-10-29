//! The notice chip as data and rules, without GPUI: what a notice says (`message`) and when one is shown,
//! swapped, dropped or taken down (`machine`) and where the chip goes (`place`). A small chip drops from the top
//! edge of a terminal to say a copy happened (or, for a warning, stays until it is clicked); a native panel draws it (`platform::notice`), and the app's `notice` module drives that panel
//! from the machine here, with the clock it holds.
//!
//! The words are not here: a notice is its kind and its numbers, and the app turns it into a sentence from the
//! catalogs (`wormhole-text`). It never holds what was copied, only how much, so a password or a screen being
//! shared can't leak through it. The timings are `view::motion::notice`.

mod buttons;
mod machine;
mod message;
mod place;
mod reason;

pub use buttons::{Buttons, Press, Reaction};
pub use machine::{ChipRules, Effect, Phase};
pub use message::{Copied, Driven, Driver, Failed, Glyph, Notice, Refused, Warning};
pub use place::{Placed, Spot, WINDOW_MARGIN, place};
pub use reason::{REASON_MAX, reason};
