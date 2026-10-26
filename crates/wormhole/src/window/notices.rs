//! What the window tells the notice chips about itself (`notice::register`): its profile, whose switches and
//! theme a chip follows, and where its selected terminal is, for a notice that is not about a terminal of its
//! own (a path copied from the sidebar, the palette, git). Told as a closure that is asked again each time a
//! chip is said or placed, so the answers are the window's of that moment.

use std::rc::Rc;

use gpui_kit::*;

use super::Workspace;
use crate::notice::Facts;

impl Workspace {
    /// Tell the chips who this window is, once, as it opens.
    pub(super) fn tell_notices(&self, window: &Window, cx: &mut Context<Self>) {
        let this = cx.weak_entity();
        let home = move |cx: &App| {
            this.read_with(cx, |ws, cx| Facts {
                profile: ws.profile.clone(),
                selected: Some(ws.active_terminal().read(cx).pane_bounds()),
            })
            .ok()
        };
        crate::notice::register(window, Rc::new(home), cx);
    }
}
