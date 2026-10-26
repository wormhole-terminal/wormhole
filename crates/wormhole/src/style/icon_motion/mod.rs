//! Icon buttons answering a click with their own drawing: a toggle's strip filling and draining with
//! its column. The icon never moves around; only what is inside it changes, and it always comes to
//! rest at the icon's usual look for its state. `UiColors::icon_motion` (the Animate icons setting)
//! turns them all off; under Reduce Motion GPUI shows each animation's last frame, which is that rest.

mod strip;

pub use strip::{StripFill, StripSide};
