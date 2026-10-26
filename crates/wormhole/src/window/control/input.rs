//! Keys and pointer events from the control line, typed into a terminal of this window.

use gpui_kit::*;
use wormhole_core::ipc::control::MouseOp;
use wormhole_core::ipc::control::keys::Stroke;

use crate::terminal_view::MouseSent;
use crate::window::Workspace;

impl Workspace {
    /// Press `strokes` in `tab`'s terminal, each encoded with the modes the program has when it goes
    /// out; how many were sent, or `None` when the terminal can't be reached.
    pub(crate) fn control_send_keys(
        &mut self,
        tab: i64,
        strokes: &[Stroke],
        cx: &mut Context<Self>,
    ) -> Option<usize> {
        let terminal = self.pane_terminal(tab)?;
        Some(terminal.update(cx, |terminal, cx| terminal.send_strokes(strokes, cx)))
    }

    /// Send a pointer event to `tab`'s terminal.
    pub(crate) fn control_mouse(
        &mut self,
        tab: i64,
        op: &MouseOp,
        cx: &mut Context<Self>,
    ) -> Option<MouseSent> {
        let terminal = self.pane_terminal(tab)?;
        Some(terminal.update(cx, |terminal, _| terminal.send_mouse(op)))
    }
}
