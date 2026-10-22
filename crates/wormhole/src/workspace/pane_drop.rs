//! Dropping a terminal on the panes: a sidebar row onto a pane's edge joins the
//! split on that side, a pane's header onto another pane's edge moves it there. The terminal under
//! the pointer swallows the mouse, so while a drag is in flight an overlay lies over the panes' area
//! (and only then): it hears the drag move and the drop, lights the side the pointer is nearest
//! (the share of the pane the terminal would take, with what it will do written on it) and shows the
//! other sides that would work as faint dashed outlines. A side that would leave a pane under its
//! minimum size shows nothing and takes nothing. Which pane and side is core's (`drop_target`), and
//! whether it works `TabPanes::can_join` / `can_move_to`; the overlay draws no animation, so Reduce
//! Motion has nothing to hold still.

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::panes::{DropError, PaneId, Rect, TabPanes, Zone, drop_target, hint_rect, preview_rect};
use wormhole_core::sidebar::SplitId;

use super::Workspace;
use super::terminal_drag::{DraggedTerminal, PaneHover};
use crate::settings::ui_in;
use crate::style;

/// What a hint on a zone says a drop will do.
fn hint_text(zone: Zone, moving: bool, target: &str) -> String {
    match (moving, zone) {
        (false, Zone::Left) => "Show on the left".to_string(),
        (false, Zone::Right) => "Show on the right".to_string(),
        (false, Zone::Top) => "Show above".to_string(),
        (false, Zone::Bottom) => "Show below".to_string(),
        (true, Zone::Left) => format!("Place left of {target}"),
        (true, Zone::Right) => format!("Place right of {target}"),
        (true, Zone::Top) => format!("Place above {target}"),
        (true, Zone::Bottom) => format!("Place below {target}"),
    }
}

/// `zone` (an absolute element) laid at `rect`, in the panes' area's own coordinates.
fn at(zone: Div, rect: Rect) -> Div {
    zone.left(px(rect.x)).top(px(rect.y)).w(px(rect.w)).h(px(rect.h))
}

impl Workspace {
    /// The panes on screen as rectangles in the panes' area: the selected split's, or the
    /// selected terminal alone filling it.
    pub(super) fn drop_rects(&self) -> Vec<(PaneId, Rect)> {
        let (w, h) = self.pane_area_size();
        match self.active_split() {
            Some(split) => split.layout.shown(w, h),
            None => vec![(self.active_session().id, Rect::new(0.0, 0.0, w, h))],
        }
    }

    /// Whether the dragged terminal is a pane of the selected split already: dropping it on a
    /// pane's edge then moves it, else it joins.
    fn is_moving(&self, dragged: i64) -> bool {
        self.active_split().is_some_and(|split| split.layout.contains(dragged))
    }

    /// Whether terminal `dragged` could be dropped on `zone` of pane `target`, with the room there is.
    pub(super) fn can_drop_on_pane(&self, dragged: i64, target: PaneId, zone: Zone) -> bool {
        let room = self.pane_room();
        match self.active_split() {
            Some(split) if split.layout.contains(dragged) => {
                split.layout.can_move_to(dragged, target, zone, room).is_ok()
            }
            Some(split) => split.layout.can_join(target, zone, dragged, room).is_ok(),
            None => TabPanes::single(self.active_session().id).can_join(target, zone, dragged, room).is_ok(),
        }
    }

    /// The pane edge a drag of `dragged` is over at `point` (the panes' area's own coordinates), if
    /// a drop there would work.
    fn pane_hover_at(&self, dragged: i64, point: (f32, f32)) -> Option<PaneHover> {
        let (target, zone) = drop_target(&self.drop_rects(), point)?;
        self.can_drop_on_pane(dragged, target, zone).then_some(PaneHover { target, zone })
    }

    /// `position` (the window's coordinates) in the panes' area, or `None` outside it.
    fn in_pane_area(&self, position: Point<Pixels>) -> Option<(f32, f32)> {
        self.pane_area.contains(&position).then(|| {
            (f32::from(position.x - self.pane_area.origin.x), f32::from(position.y - self.pane_area.origin.y))
        })
    }

    fn on_pane_drag_move(
        &mut self,
        event: &DragMoveEvent<DraggedTerminal>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(dragged) = self.terminal_drag.as_ref().map(|drag| drag.id) else { return };
        let point = self.in_pane_area(event.event.position);
        let hover = point.and_then(|point| self.pane_hover_at(dragged, point));
        // The sidebar's rows hear every move too: over the panes none of them is the target.
        let cleared = point.is_some() && self.sidebar.over.take().is_some();
        let Some(drag) = self.terminal_drag.as_mut() else { return };
        if drag.pane != hover || cleared {
            drag.pane = hover;
            cx.notify();
        }
    }

    fn on_pane_drop(&mut self, dragged: &DraggedTerminal, window: &mut Window, cx: &mut Context<Self>) {
        self.terminal_drag = None;
        let hover = self
            .in_pane_area(window.mouse_position())
            .and_then(|point| self.pane_hover_at(dragged.id, point));
        if let Some(PaneHover { target, zone }) = hover
            && let Err(error) = self.drop_on_pane(dragged.id, target, zone, window, cx)
        {
            tracing::debug!(?error, "a drop on a pane was refused");
        }
        cx.notify();
    }

    /// Terminal `dragged` is dropped on `zone` of pane `target`: one of the selected split's panes
    /// moves there, any other terminal joins the split (made if the target stands alone) and leaves
    /// its own, whichever it was dragged by. It takes the keyboard. Refused, with nothing changed,
    /// for want of room (`DropError::TooSmall`), a drop that would change nothing, or a terminal or
    /// pane that isn't there.
    pub(crate) fn drop_on_pane(
        &mut self,
        dragged: i64,
        target: PaneId,
        zone: Zone,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), DropError> {
        let ix = self.find_session(dragged).ok_or(DropError::Unknown)?;
        if !self.is_moving(dragged) {
            return self.join_split(ix, target, zone, window, cx);
        }
        let room = self.pane_room();
        let split_id = self.sessions[self.active].split.ok_or(DropError::Unknown)?;
        let split = self.split_mut(split_id).ok_or(DropError::Unknown)?;
        split.layout.move_to(dragged, target, zone, room)?;
        self.follow_layout(split_id);
        self.dirty = true;
        let at = self.find_session(dragged).unwrap_or(ix);
        self.activate(at, window, cx);
        Ok(())
    }

    /// The split's terminals take the places in the list its panes have in reading order: the
    /// sidebar follows the layout when a pane is moved. Only its members' places are
    /// swapped; the rest of the list stays where it is.
    fn follow_layout(&mut self, id: SplitId) {
        let Some(panes) = self.split_with(id).map(|split| split.layout.panes()) else { return };
        if self.members(id).len() != panes.len() {
            return;
        }
        let ids = self.session_ids();
        self.sidebar.view.reorder_terminals(&ids, &panes);
    }

    /// The overlay over the panes while a terminal is dragged (and nothing otherwise): the
    /// surface's size, as for the panes.
    pub(super) fn render_drop_overlay(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let drag = self.terminal_drag.as_ref().filter(|_| cx.has_active_drag())?;
        let ui = ui_in(&self.profile, cx);
        let mut zones: Vec<AnyElement> = Vec::new();
        if let Some(PaneHover { target, zone }) = drag.pane
            && let Some((_, pane)) = self.drop_rects().into_iter().find(|(id, _)| *id == target)
        {
            // The sides that would work too, faint; the one under the pointer over them.
            for other in Zone::ALL.into_iter().filter(|&other| other != zone) {
                if let Some(band) = hint_rect(pane, other, zone)
                    && self.can_drop_on_pane(drag.id, target, other)
                {
                    zones.push(
                        style::pane_drop_zone(ui, style::ZoneLook::Hint, other)
                            .map(|d| at(d, band))
                            .into_any_element(),
                    );
                }
            }
            let share = preview_rect(pane, zone);
            let name =
                self.find_session(target).map_or_else(SharedString::default, |ix| self.sessions[ix].name(cx));
            zones.push(
                style::pane_drop_zone(ui, style::ZoneLook::Active, zone)
                    .map(|d| at(d, share))
                    .child(style::drop_hint(ui, hint_text(zone, self.is_moving(drag.id), &name)))
                    .into_any_element(),
            );
        }
        Some(
            div()
                .id("pane-drop")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                // The terminals under it hear nothing while a terminal is on its way.
                .occlude()
                .on_drag_move::<DraggedTerminal>(cx.listener(Self::on_pane_drag_move))
                .on_drop::<DraggedTerminal>(cx.listener(Self::on_pane_drop))
                .children(zones)
                .into_any_element(),
        )
    }
}
