//! A terminal and its window's geometry: the pane's size window operations (XTWINOPS) are sums on, and
//! telling the program where the window is and whether it is in the Dock.

use std::time::{Duration, Instant};

use gpui_kit::*;
use wormhole_term::WindowGeometry;

use super::TerminalView;

impl TerminalView {
    /// The pane's size, which window operations are sums on: what the last resize asked for while
    /// the layout has not caught up with it.
    pub fn pane_size(&self) -> crate::window_ops::PaneSize {
        let layout = self.layout.get();
        let mut pane = crate::window_ops::PaneSize {
            cell: (f32::from(layout.cell.width), f32::from(layout.cell.height)),
            cols: layout.cols,
            rows: layout.rows,
            frame: layout.frame,
        };
        if let Some((cols, rows)) = self.size_pending() {
            (pane.cols, pane.rows) = (cols, rows);
        }
        pane
    }

    /// The size the window was last asked to give the pane, while the layout has not shown it yet.
    fn size_pending(&self) -> Option<(usize, usize)> {
        let layout = self.layout.get();
        match self.size_asked {
            Some((crate::window_ops::SizeGoal::Set { cols, rows }, before, at))
                if before == (layout.cols, layout.rows) && at.elapsed() < Duration::from_secs(1) =>
            {
                Some((cols, rows))
            }
            _ => None,
        }
    }

    /// Tell the engine where the window is when that changed since it was last told.
    pub(super) fn share_window(&mut self, geometry: Option<WindowGeometry>) {
        /// How long AppKit gets to follow a minimise or a restore before it is believed over the
        /// request (a window that cannot be minimised stays as it is).
        const SETTLE: Duration = Duration::from_secs(3);
        let Some(mut geometry) = geometry else { return };
        // The engine has worked out the window's size from the resize (it grows by what the text
        // area does); the window and its frames still show the size before, which would undo that.
        if self.size_pending().is_some() {
            return;
        }
        if let Some((want, at)) = self.minimise_asked {
            if geometry.iconified == want || at.elapsed() > SETTLE {
                self.minimise_asked = None;
            } else {
                geometry.iconified = want;
            }
        }
        if self.window_given == Some(geometry) {
            return;
        }
        self.window_given = Some(geometry);
        if let Some(pty) = &self.pty {
            pty.set_window(geometry);
        }
    }

    /// The workspace has carried out a program's window operation (`minimised` when it says the
    /// window is, or is no longer, in the Dock). The window takes a moment to follow a resize, and
    /// may not at all: after it has had that, the engine is brought back to the pane's size.
    pub fn window_op_done(&mut self, done: crate::window_ops::Applied, cx: &mut Context<Self>) {
        let minimised = done.minimised;
        match done.size {
            crate::window_ops::SizeGoal::Unchanged => {}
            goal => {
                let layout = self.layout.get();
                self.size_asked = Some((goal, (layout.cols, layout.rows), Instant::now()));
            }
        }
        if let Some(minimised) = minimised {
            self.minimise_asked = Some((minimised, Instant::now()));
        }
        if let (Some(minimised), Some(given)) = (minimised, self.window_given) {
            // No frame is drawn while the window is minimised, so nothing else would say so.
            let told = WindowGeometry { iconified: minimised, ..given };
            self.window_given = Some(told);
            if let Some(pty) = &self.pty {
                pty.set_window(told);
            }
        }
        self.window_sync = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(400)).await;
            this.update(cx, |this, _| {
                if let Some(pty) = &this.pty {
                    pty.sync_size();
                }
            })
            .ok();
        }));
    }
}
