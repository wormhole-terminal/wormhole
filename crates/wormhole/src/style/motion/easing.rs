//! How a motion eases: the curves are core's (`view::motion`), because a pose there eases along them
//! too and the curve must be one. [`CubicBezier::easing`] hands one to GPUI's `Animation::with_easing`.

pub use wormhole_core::view::motion::{CubicBezier, EASE_IN, EASE_IN_OUT, EASE_OUT, LINEAR};
