//! What a window does when the Probes that are on, or what their hooks set, changed
//! (`probe_runtime`): its terminals draw their rows again and its status bar is built again.
//!
//! Both are cached views, which replay what they drew until told otherwise, so a change that comes from
//! outside a draw has to reach them as a notify (`window::cached`).

use gpui_kit::*;

use super::Workspace;

impl Workspace {
    /// Rules, a status item or a marked block changed: say so to everything that draws them.
    pub(crate) fn probes_changed(&mut self, cx: &mut Context<Self>) {
        for session in &self.sessions {
            session.terminal().update(cx, |_, cx| cx.notify());
        }
        self.redraw_status_bar_if_stale(cx);
        cx.notify();
    }
}
