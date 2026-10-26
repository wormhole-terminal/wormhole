//! Which size the PTY is told in a frame, and which grid is drawn (core `view::motion::live_resize`).

use std::time::Duration;

use gpui_kit::*;
use wormhole_core::view::motion::live_resize::Step;
use wormhole_term::GridSize;

use crate::platform;
use crate::terminal_view::TerminalView;

impl TerminalView {
    /// The frame's pane makes a grid of `wanted`; tell the PTY if it is to be told now, and answer with the
    /// columns and rows to draw: `wanted`'s, or the PTY's own while it is held back.
    ///
    /// Held back while the text zoom moves (the PTY is told once, when it arrives; the Motion feature) and
    /// during a live resize (at most every 100 ms, and once more when it ends; the Live resize feature).
    /// Otherwise, and without the features, every size is told at once, as it always was.
    pub(in crate::terminal_view) fn scale_grid(
        &mut self,
        wanted: GridSize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (usize, usize) {
        let asked = (usize::from(wanted.cols), usize::from(wanted.rows));
        let (live_on, motion_on) = (self.live_on(cx), self.motion_on(cx));
        if !live_on && !motion_on {
            self.resize(wanted, cx);
            return asked;
        }
        let Some(told) = self.pty.as_ref().map(|pty| pty.size()) else { return asked };
        let held = (usize::from(told.cols), usize::from(told.rows));
        if motion_on && self.scaling.zoom.is_some() {
            return held;
        }
        let now = cx.background_executor().now();
        // Whether a drag is on is what the window's notifications said, not asked of AppKit each frame.
        let live = live_on && platform::live_resize::in_progress(window);
        let differs = told != wanted;
        let step = if live_on {
            self.scaling.gate.step(now, live, differs)
        } else {
            Step { apply: differs, wake_in: None }
        };
        self.note_size(asked, live, window, cx);
        if step.apply {
            self.resize(wanted, cx);
        }
        self.wake_in(step.wake_in, cx);
        if step.apply || !differs { asked } else { held }
    }

    /// Draw this terminal again after `after` without anyone asking: the size held back is due then.
    fn wake_in(&mut self, after: Option<Duration>, cx: &mut Context<Self>) {
        let Some(after) = after else {
            self.scaling.wake = None;
            return;
        };
        self.scaling.wake = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(after).await;
            this.update(cx, |_, cx| cx.notify()).ok();
        }));
    }
}
