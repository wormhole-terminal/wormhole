//! The text size easing to a new one (core `view::motion::text_zoom`).

use gpui_kit::*;

use crate::terminal_view::TerminalView;

impl TerminalView {
    /// Draw the text this many points over the size in Settings (⌘+, ⌘-, ⌘0). With the Motion feature the
    /// size eases there over 120 ms (cut under Reduce Motion), a second press going on from the size the
    /// first reached, and the chip says the grid's size once it has arrived.
    pub fn set_font_zoom(&mut self, zoom: f32, cx: &mut Context<Self>) {
        if self.font_zoom == zoom {
            return;
        }
        let now = cx.background_executor().now();
        if self.scaling_on(cx) {
            self.scaling.zoom = (!cx.reduce_motion()).then(|| match self.scaling.zoom {
                Some(under_way) => under_way.retarget(now, zoom),
                None => wormhole_core::view::motion::text_zoom::Ease::new(self.font_zoom, zoom, now),
            });
            self.scaling.readout_due = Some(now);
        } else {
            self.scaling.zoom = None;
        }
        self.font_zoom = zoom;
        cx.notify();
    }

    /// Take the window's zoom as a terminal that is new here (made in this window, or moved into it) finds
    /// it: at once, with nothing said about the size.
    pub fn adopt_font_zoom(&mut self, zoom: f32, cx: &mut Context<Self>) {
        self.scaling.zoom = None;
        if self.font_zoom != zoom {
            self.font_zoom = zoom;
            cx.notify();
        }
    }

    /// The points over Settings the text is drawn at in this frame: the zoom's, or where the ease has got
    /// to, which asks for another frame until it has arrived.
    pub(in crate::terminal_view) fn shown_zoom(&mut self, window: &Window, cx: &App) -> f32 {
        let Some(ease) = self.scaling.zoom else { return self.font_zoom };
        let sample = ease.at(cx.background_executor().now());
        if sample.done {
            self.scaling.zoom = None;
            return self.font_zoom;
        }
        window.request_animation_frame();
        sample.size
    }
}
