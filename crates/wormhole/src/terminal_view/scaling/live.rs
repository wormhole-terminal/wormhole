//! Which size the PTY is told in a frame, and which grid is drawn (core `view::motion::live_resize`).

use std::time::Duration;

use gpui_kit::*;
use wormhole_term::GridSize;

use crate::platform;
use crate::terminal_view::{TerminalEvent, TerminalView};

impl TerminalView {
    /// The frame's pane makes a grid of `wanted`; tell the PTY if it is to be told now, and answer with the
    /// columns and rows to draw: `wanted`'s, or the PTY's own while it is held back.
    ///
    /// Held back during a live resize (at most every 100 ms, and once more when it ends). Otherwise, and
    /// without the Motion feature, every size is told at once, as it always was.
    pub(in crate::terminal_view) fn scale_grid(
        &mut self,
        wanted: GridSize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (usize, usize) {
        let asked = (usize::from(wanted.cols), usize::from(wanted.rows));
        if !self.scaling_on(cx) {
            self.resize(wanted, cx);
            return asked;
        }
        let Some(told) = self.pty.as_ref().map(|pty| pty.size()) else { return asked };
        let held = (usize::from(told.cols), usize::from(told.rows));
        let now = cx.background_executor().now();
        let live = platform::window::in_live_resize(window);
        let step = self.scaling.gate.step(now, live, told != wanted);
        self.note_size(asked, live, window, cx);
        if step.apply {
            self.resize(wanted, cx);
        }
        self.wake_in(step.wake_in, cx);
        if step.ended {
            // The window may now take off the strip a window off the cell grid leaves.
            cx.emit(TerminalEvent::LiveResizeEnded);
        }
        if step.apply || told == wanted { asked } else { held }
    }

    /// Draw this terminal again after `after` without anyone asking: the size held back is due then, and a
    /// live resize may have ended.
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
