//! The text size after ⌘+, ⌘− and ⌘0, and what a terminal new to a zoomed window finds.

use gpui_kit::*;

use crate::terminal_view::TerminalView;

impl TerminalView {
    /// Draw the text this many points over the size in Settings (⌘+, ⌘-, ⌘0). With the Motion feature the
    /// chip says the grid's size once the new size has been laid out.
    pub fn set_font_zoom(&mut self, zoom: f32, cx: &mut Context<Self>) {
        if self.font_zoom == zoom {
            return;
        }
        if self.scaling_on(cx) {
            self.scaling.readout_due = Some(cx.background_executor().now());
        }
        self.font_zoom = zoom;
        cx.notify();
    }

    /// Take the window's zoom as a terminal that is new here (made in this window, or moved into it) finds
    /// it: at once, with nothing said about the size.
    pub fn adopt_font_zoom(&mut self, zoom: f32, cx: &mut Context<Self>) {
        if self.font_zoom != zoom {
            self.font_zoom = zoom;
            cx.notify();
        }
    }
}
