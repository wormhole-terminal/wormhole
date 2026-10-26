//! A split opening (⌘D, ⇧⌘D) as the window draws it (core `view::motion::split`): the new pane's edge springs
//! from the far end of the pane that was split to its place, and that pane gives way to it. Nothing about a
//! terminal's size moves with it.
//!
//! The layout is the new one from the first moment (the model, the sidebar and the keyboard know the new pane
//! at once), but the window keeps drawing the layout *before* it until the spring is at rest: the pane that was
//! split, whose box is as wide (or as tall) as the edge leaves it and has its header from the start, and the new
//! pane's box, whole and in its final size, which slides in from the end of the pane's room, clipped to it. The
//! two meet at the edge and never overlap, so nothing shows through anything. The pane that was split keeps the
//! terminal it had: held at the size of its grid, from the top left, under the header and cut at the box's
//! edges, so what moves is its place and never its size. Then the new layout replaces the picture, and that
//! terminal is laid out at its new size once: the one resize it gets, at the end. The new pane's terminal is made
//! the size it has in the new layout when it is first drawn and is not resized again. A frame where the picture
//! would be wrong (the layout changed, the window was resized, a pane was zoomed) draws the layout as it is,
//! which is a cut.
//!
//! The panes of the picture before the split are only a picture: a shield over them takes the pointer, so a
//! divider of the layout that is gone cannot be dragged (its path names a tree that is not the model's now).
//! Only the new pane's box, over the shield, is live.

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::motion::split::{self, Opening as Plan};
use wormhole_core::view::panes::{Axis, Node, PaneId, TabPanes};

use crate::settings::{settings_in, ui_in};
use crate::style::motion::{LAST_FRAME_GRACE, elapsed};
use crate::style::{self, Edges as _};
use crate::window::Workspace;
use crate::window::panes::splits::Split;

/// A split that is opening: what it was, what it ends in and for which room, and the timer that ends it.
pub(in crate::window) struct Opening {
    plan: Plan,
    /// The layout the pane that was split was in: a lone terminal's is one pane.
    before: TabPanes,
    /// The layout it ends in.
    after: TabPanes,
    new: PaneId,
    /// The size the pane that was split's terminal had, in points, which it keeps while the edge moves
    /// (`TerminalView::size_for_grid`).
    hold: (f32, f32),
    /// The area it was planned for: a window resized meanwhile is a cut.
    area: (f32, f32),
    _end: Task<()>,
}

/// What a pane of the layout before the split is drawn with while it opens.
#[derive(Clone, Copy)]
pub(super) struct Stand {
    plan: Plan,
    /// The new pane, which has the keyboard: the one that wears the focused look.
    new: PaneId,
    hold: (f32, f32),
}

impl Stand {
    /// Whether `pane` is the one that was split.
    pub(super) fn is_split(&self, pane: PaneId) -> bool {
        self.plan.target == pane
    }

    /// Whether `pane` wears the focused look: the new pane, and nothing before it.
    pub(super) fn focused(&self, pane: PaneId) -> bool {
        self.new == pane
    }

    /// The size its terminal keeps.
    pub(super) fn hold(&self) -> (f32, f32) {
        self.hold
    }
}

impl Opening {
    /// The pane that was split, for the tests.
    #[cfg(test)]
    pub(in crate::window) fn target_for_test(&self) -> PaneId {
        self.plan.target
    }
}

impl Workspace {
    /// Pane `new` was just added to the selected terminal's split, which was `before` it (`None`: the terminal
    /// stood alone): open it, unless motion is off or reduced, the room is not known yet or the change is not a
    /// pane's room shared with a new one.
    pub(in crate::window) fn begin_opening(
        &mut self,
        before: Option<TabPanes>,
        new: PaneId,
        cx: &mut Context<Self>,
    ) {
        if !self.plays_motion(cx) {
            return;
        }
        let area = self.pane_area_size();
        let Some(split) = self.find_session(new).and_then(|ix| self.sessions[ix].split) else { return };
        let Some(after) = self.split_with(split).map(|split| split.layout.clone()) else { return };
        let Some(plan) = Plan::of(before.as_ref(), &after, new, area.0, area.1) else { return };
        let Some(terminal) = self.pane_terminal(plan.target) else { return };
        // What its terminal measured in the last frame of the layout before, which no frame has changed yet.
        let hold = terminal.read(cx).size_for_grid(cx);
        let before = before.unwrap_or_else(|| TabPanes::single(plan.target));
        let end = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(split::duration() + LAST_FRAME_GRACE).await;
            this.update(cx, |this, cx| {
                let ended = this
                    .split_mut(split)
                    .is_some_and(|split| split.opening.take_if(|opening| opening.new == new).is_some());
                if ended {
                    cx.notify();
                }
            })
            .ok();
        });
        if let Some(split) = self.split_mut(split) {
            split.opening = Some(Opening { plan, before, after, new, hold, area, _end: end });
        }
    }

    /// The opening the selected terminal's split is drawn with, while the picture is still right for it.
    fn opening_of<'a>(&self, split: &'a Split) -> Option<&'a Opening> {
        let opening = split.opening.as_ref()?;
        (split.layout.tree() == opening.after.tree()
            && split.layout.zoomed().is_none()
            && opening.area == self.pane_area_size())
        .then_some(opening)
    }

    /// The panes as they were before the split, with the new pane opening over the room of the one that was
    /// split: the whole of what `split`'s panes are drawn as, while one opens. `None` when none does.
    pub(super) fn render_opening(&self, split: &Split, cx: &mut Context<Self>) -> Option<AnyElement> {
        let opening = self.opening_of(split)?;
        let stand = Stand { plan: opening.plan, new: opening.new, hold: opening.hold };
        let was = Split { id: split.id, layout: opening.before.clone(), opening: None };
        let root = was.layout.tree().root();
        let order = was.layout.panes();
        let mut body = self.render_node(&was, root, &mut Vec::new(), &order, Some(&stand), cx);
        if let Node::Leaf(_) = root {
            // A pane that stood alone has no cell to be sized in: it is the whole of the picture.
            body = self.kept_box(body, &stand, (0.0, 0.0));
        }
        Some(
            div()
                .id("panes")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .child(body)
                // The panes of the layout that is gone are a picture: nothing in them takes the pointer.
                .child(
                    div()
                        .id("opening-shield")
                        .absolute()
                        .size_full()
                        .occlude()
                        .debug_selector(|| "opening-shield".into()),
                )
                .child(self.new_pane_over(split, opening, cx))
                .into_any_element(),
        )
    }

    /// The pane that was split, as wide (or as tall) as the new pane's edge leaves it. `lead` is the gap its
    /// cell puts before it, which is not part of its box.
    pub(super) fn kept_box(&self, pane: AnyElement, stand: &Stand, lead: (f32, f32)) -> AnyElement {
        let (plan, duration) = (stand.plan, split::duration());
        let at = move |s: f32| {
            let kept = plan.kept_at(s);
            (kept.w - lead.0, kept.h - lead.1)
        };
        let (w, h) = at(0.0);
        div()
            .flex_none()
            .w(px(w))
            .h(px(h))
            .debug_selector(|| "split-opening-kept".into())
            .child(pane)
            .with_animation(
                ("split-opening-keep", plan.target as u64),
                Animation::new(duration),
                move |d, t| {
                    let (w, h) = at(split::travelled(elapsed(duration, t)));
                    d.w(px(w)).h(px(h))
                },
            )
            .into_any_element()
    }

    /// The new pane's box, in its final size, sliding in over the room of the pane that was split, which
    /// clips it to that room: its edge is where the spring says, and nothing of it is sized.
    fn new_pane_over(&self, split: &Split, opening: &Opening, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui_in(&self.profile, cx);
        let gaps = settings_in(&self.profile, cx).split_gaps;
        let (plan, duration) = (opening.plan, split::duration());
        // The new pane's box is its room less the gap before it, which the layout puts inside the room.
        let gap = if gaps { style::PANE_GAP } else { 0.0 };
        let (gap_x, gap_y) = match plan.axis {
            Axis::Row => (gap, 0.0),
            Axis::Column => (0.0, gap),
        };
        let pane = self.render_pane(split, opening.new, &opening.after.panes(), None, cx);
        let new_pane = plan.new_pane;
        // With surfaces of their own the pane paints its colour; without, the column does and a hairline marks
        // the pane's edge.
        let boxed = div()
            .absolute()
            .w(px(new_pane.w - gap_x))
            .h(px(new_pane.h - gap_y))
            .when(!gaps, |d| d.rule_left(ui))
            .debug_selector(|| "split-opening-new".into())
            .child(pane)
            .with_animation(
                ("split-opening-in", opening.new as u64),
                Animation::new(duration),
                move |d, t| {
                    let at = plan.new_at(split::travelled(elapsed(duration, t)));
                    d.left(px(at.x + gap_x - plan.whole.x)).top(px(at.y + gap_y - plan.whole.y))
                },
            );
        div()
            .absolute()
            .left(px(plan.whole.x))
            .top(px(plan.whole.y))
            .w(px(plan.whole.w))
            .h(px(plan.whole.h))
            .overflow_hidden()
            .child(boxed)
            .into_any_element()
    }
}
