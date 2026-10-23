//! Where macOS draws a window's traffic lights, and what has to clear them.

use gpui_kit::*;

/// The traffic lights start 12 pt in, on the line the sidebar's "Terminals" starts on (the user's
/// call over the board's 20, which left the lights hanging right of everything below them). They
/// are assumed to be three 12 pt ones 8 apart (x 12 to 64); later macOS versions draw them larger, so the span is measured (`platform::window`)
/// by whatever has to clear the real lights.
pub(crate) const LIGHTS_X: f32 = 12.0;
pub(crate) const ASSUMED_LIGHTS_SPAN: f32 = 52.0;
/// The height of AppKit's frame round each light, which is what its position places.
const LIGHT_FRAME: f32 = 14.0;

/// Where macOS draws the traffic lights (the close button's frame): from the 12 pt inset,
/// centred in a top row `row` tall. Every window of the app puts them in a title row; a workspace
/// with a taller tab bar moves them down to its middle.
pub(crate) fn traffic_light_position(row: f32) -> Point<Pixels> {
    point(px(LIGHTS_X), px((row - LIGHT_FRAME) / 2.0))
}

/// Where the traffic lights end in this window, from the left edge: for whatever has to clear
/// the real lights (the Studio's title, the sidebar's collapse button).
pub(crate) fn lights_end(window: &Window) -> f32 {
    LIGHTS_X + crate::platform::window::span(window).unwrap_or(ASSUMED_LIGHTS_SPAN)
}
