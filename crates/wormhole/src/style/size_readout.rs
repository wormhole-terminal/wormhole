//! The chip in the middle of a terminal that says its size ("120 × 40") while its window is resized and after
//! its text is zoomed. Tooltip colours (the same tokens as [`readout`](super::readout), which stands beside
//! a dragged handle), body text whose digits keep their width, and a chip's corners. Its fading is the
//! caller's: it passes the opacity (core `view::motion::size_readout` times it).

use gpui_kit::prelude::*;
use gpui_kit::*;

use super::scale::TextRole;
use crate::settings::UiColors;

/// A terminal's size on a chip, `opacity` (0.0..=1.0) shown. It takes no clicks: the terminal below it still
/// gets them.
#[track_caller]
pub fn size_readout(ui: UiColors, text: impl Into<SharedString>, opacity: f32) -> Div {
    div()
        .flex_none()
        .px_3()
        .py_1()
        .rounded(ui.radius_chip)
        .bg(ui.tooltip)
        .text_color(ui.tooltip_text)
        .text_body()
        .tabular_nums()
        .whitespace_nowrap()
        .opacity(opacity)
        .child(text.into())
}
