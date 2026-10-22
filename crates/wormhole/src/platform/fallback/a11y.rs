//! `platform::a11y` with no screen reader to post to: GPUI has no live regions, so an
//! announcement goes to devctl's log only, and buttons are not ⇥ stops.

pub fn keyboard_navigation() -> bool {
    false
}

pub fn post(_text: &str, _interrupt: bool) {}

#[cfg(feature = "devctl")]
pub fn wake_tree(_: &gpui_kit::Window) {}
