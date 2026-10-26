//! The notice chip as data and rules, without GPUI: what a notice says (`message`) and when one is shown,
//! swapped, dropped or taken down (`machine`). A small chip drops from the top of a window to say a copy
//! happened; a native panel draws it (`platform::notice`), and the app's `notice` module drives that panel
//! from the machine here, with the clock it holds.
//!
//! The words are not here: a notice is its kind and its numbers, and the app turns it into a sentence from the
//! catalogs (`wormhole-text`). It never holds what was copied, only how much, so a password or a screen being
//! shared can't leak through it. The timings are `view::motion::notice`.

mod machine;
mod message;

pub use machine::{Effect, Notices, Phase};
pub use message::{Copied, Glyph, Notice};
