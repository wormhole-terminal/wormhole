//! What the window does when the list's rows are dropped, pinned, shown beside each other: the parts of
//! the sidebar's gestures that change terminals and panes, not the list. The list works out where a
//! drop would land (`Sidebar::terminal_drop_plan`); the window moves the terminal or joins its split.

use gpui_kit::*;
use wormhole_core::view::panes::{DropError, TabPanes, Zone};
use wormhole_core::view::sidebar::Drop;
#[cfg(any(test, feature = "devctl"))]
use wormhole_core::view::sidebar::Part;

use crate::window::Workspace;
use crate::window::sidebar::Sidebar;
use crate::window::sidebar::terminal_drag::{DragOrigin, Over};

impl Workspace {
    /// A terminal was dropped on what the pointer was over last (`over`): it moves, joins a split or
    /// leaves its own. A pane's header dropped where no row could take it still leaves its split,
    /// unless it was dropped on a terminal of that same split.
    pub(super) fn finish_terminal_drop(
        &mut self,
        dragged: i64,
        origin: DragOrigin,
        over: Option<Over>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.terminal_drag = None;
        let result = over.map(|over| self.drop_terminal(dragged, over, window, cx));
        match result {
            Some(Err(error)) => tracing::debug!(?error, "a drop on a row was refused"),
            Some(Ok(Some(_))) => {}
            Some(Ok(None)) | None if origin == DragOrigin::PaneHeader => {
                if let Some(Over::Row { id, .. }) = over {
                    self.leave_unless_beside(dragged, id, cx)
                } else if let Some(ix) = self.find_session(dragged) {
                    self.leave_split(ix, cx);
                    self.mark_dirty(cx);
                }
            }
            Some(Ok(None)) | None => {}
        }
        cx.notify();
    }

    /// A header was dropped on the row of terminal `onto`, which took nothing: its terminal leaves
    /// its split unless `onto` is in the same one.
    fn leave_unless_beside(&mut self, dragged: i64, onto: i64, cx: &mut Context<Self>) {
        let (Some(from), Some(to)) = (self.find_session(dragged), self.find_session(onto)) else { return };
        if self.sessions[from].split.is_some() && self.sessions[from].split != self.sessions[to].split {
            self.leave_split(from, cx);
            self.mark_dirty(cx);
        }
    }

    /// Drop terminal `dragged` on `over`: it moves (with the rest of its split, unless it leaves),
    /// joins `onto`'s split or leaves its own. `Ok(None)` when nothing would change; an error when
    /// a join was refused (for want of room, mostly) and nothing changed.
    fn drop_terminal(
        &mut self,
        dragged: i64,
        over: Over,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Option<Drop>, DropError> {
        let input = self.sidebar_input(self.sidebar.read(cx).width(), cx);
        let listing = self.sidebar.read(cx).listing_of(&input, cx);
        let Some(drop) = Sidebar::terminal_drop_plan(&input, &listing, dragged, over) else {
            return Ok(None);
        };
        match drop {
            Drop::Move { landing, leave_split, with_mates } => {
                if leave_split && let Some(ix) = self.find_session(dragged) {
                    self.leave_layout(ix);
                }
                self.with_sidebar(cx, |sidebar, cx| {
                    sidebar.land_terminal(dragged, landing, with_mates, cx);
                    sidebar.flash(dragged);
                });
            }
            Drop::Join { target } => {
                let Some(joiner) = self.find_session(dragged) else { return Ok(None) };
                let target = self.sessions[target].id;
                self.join_split(joiner, target, Zone::Right, window, cx)?;
            }
        }
        self.mark_dirty(cx);
        Ok(Some(drop))
    }

    /// A drop of `dragged` on the `part` of terminal `onto`'s row (devctl, the tests).
    #[cfg(any(test, feature = "devctl"))]
    pub(crate) fn row_drop(
        &mut self,
        dragged: i64,
        onto: i64,
        part: Part,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Option<Drop>, DropError> {
        self.drop_terminal(dragged, Over::Row { id: onto, part }, window, cx)
    }

    /// The terminals "Show beside…" offers for the terminal `id`, in the list's order: every other
    /// one that isn't in its split already. None when its pane has no room for another beside it.
    pub(in crate::window) fn beside_choices(&self, id: i64, cx: &App) -> Vec<(i64, SharedString)> {
        let Some(ix) = self.find_session(id) else { return Vec::new() };
        let layout = match self.split_of(ix) {
            Some(split) => split.layout.clone(),
            None => TabPanes::single(id),
        };
        let room = self.pane_room();
        let split = self.sessions[ix].split;
        self.session_order(cx)
            .into_iter()
            .filter(|&other| other != ix && (split.is_none() || self.sessions[other].split != split))
            .filter(|&other| layout.can_join(id, Zone::Right, self.sessions[other].id, room).is_ok())
            .map(|other| (self.sessions[other].id, self.sessions[other].name(cx)))
            .collect()
    }

    /// Terminal `other` shows beside terminal `id`, to the right of its pane: the two are a split
    /// now (the same as dropping one row on the other). A refusal for want of room changes nothing.
    pub(in crate::window) fn show_beside(
        &mut self,
        other: i64,
        id: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ix) = self.find_session(other) else { return };
        if let Err(error) = self.join_split(ix, id, Zone::Right, window, cx) {
            tracing::debug!(?error, "Show beside was refused");
        }
    }

    /// Pin a terminal to the top of its group, or let it go back to its place (saved). A split is
    /// pinned or not as a whole: its members all take the new state.
    pub(crate) fn toggle_pin(&mut self, ix: usize, cx: &mut Context<Self>) {
        let Some(session) = self.sessions.get(ix) else { return };
        let pinned = !self.split_pinned(ix);
        let members = match session.split {
            Some(split) => self.members(split),
            None => vec![ix],
        };
        for member in members {
            self.sessions[member].pinned = pinned;
        }
        self.mark_dirty(cx);
    }
}
