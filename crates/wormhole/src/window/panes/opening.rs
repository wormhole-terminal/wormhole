//! A split opening (⌘D, ⇧⌘D) as the window draws it (core `view::motion::split`): the new pane's edge springs
//! from the far end of the pane that was split to its place, and that pane is clipped back to what the edge
//! leaves it. Nothing about a terminal's size moves with it.
//!
//! The layout is the new one from the first moment (the model, the sidebar and the keyboard know the new pane
//! at once), but the window keeps drawing the layout *before* it until the spring is at rest: the pane that was
//! split in the room it had, in the terminal it had, clipped (a lone terminal) or covered (a pane of a split)
//! by the new pane's box, which slides over it from the end of that room, whole, in its final size. Then the
//! new layout replaces the picture, and the pane that was split is laid out at its new size once: the one
//! resize its terminal gets, at the end. The new pane's terminal is made at the size it has in the new layout
//! when it is first drawn and is not resized again. A frame where the picture would be wrong (the layout
//! changed, the window was resized, a pane was zoomed) draws the layout as it is, which is a cut.
//!
//! A pane that is covered by the new one and not clipped (a pane of a split) shows through a translucent
//! terminal under it for the length of the spring; a lone terminal never does.

use std::time::Duration;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::motion::split::{self, Opening as Plan};
use wormhole_core::view::panes::{Axis, PaneId, TabPanes};

use crate::settings::{settings_in, ui_in};
use crate::style::{self, Edges as _};
use crate::window::Workspace;
use crate::window::panes::splits::Split;

/// How long after the spring is at rest its last frame is kept.
const GRACE: Duration = Duration::from_millis(40);

/// A split that is opening: what it was, what it ends in and for which room, and the timer that ends it.
pub(in crate::window) struct Opening {
    plan: Plan,
    /// The layout the pane that was split was in (`None`: it stood alone).
    before: Option<TabPanes>,
    /// The layout it ends in.
    after: TabPanes,
    new: PaneId,
    /// The area it was planned for: a window resized meanwhile is a cut.
    area: (f32, f32),
    _end: Task<()>,
}

/// How long into the spring `t` (0.0..=1.0) of its animation is.
fn elapsed(duration: Duration, t: f32) -> Duration {
    Duration::from_secs_f32(t.clamp(0.0, 1.0) * duration.as_secs_f32())
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
        if !settings_in(&self.profile, cx).motion_on() || cx.reduce_motion() {
            return;
        }
        let area = self.pane_area_size();
        let Some(split) = self.find_session(new).and_then(|ix| self.sessions[ix].split) else { return };
        let Some(after) = self.split_with(split).map(|split| split.layout.clone()) else { return };
        let Some(plan) = Plan::of(before.as_ref(), &after, new, area.0, area.1) else { return };

        let end = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(split::duration() + GRACE).await;
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
            split.opening = Some(Opening { plan, before, after, new, area, _end: end });
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

    /// Whether the selected terminal stands alone in the picture that is drawn: it is, or a split opens
    /// over it and the terminal is drawn as the lone one it was.
    pub(in crate::window) fn drawn_alone(&self) -> bool {
        match self.active_split() {
            None => true,
            Some(split) => self.opening_of(split).is_some_and(|opening| opening.before.is_none()),
        }
    }

    /// The lone terminal with the new pane opening over it, when a split is opening over it.
    pub(in crate::window) fn render_lone_opening(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let split = self.active_split()?;
        let opening = self.opening_of(split).filter(|opening| opening.before.is_none())?;
        // The pane that was split, not the selected terminal: that is the new one by now.
        let terminal = self.pane_terminal(opening.plan.target)?;
        let (plan, duration) = (opening.plan, split::duration());
        // The terminal keeps the size it has and is cut to what the new pane's edge leaves it.
        let kept = div()
            .absolute()
            .top_0()
            .left_0()
            .overflow_hidden()
            .child(
                div()
                    .w(px(plan.whole.w))
                    .h(px(plan.whole.h))
                    .child(crate::window::cached::view(&terminal, StyleRefinement::default().size_full())),
            )
            .with_animation(
                ("split-opening-keep", opening.new as u64),
                Animation::new(duration),
                move |d, t| {
                    let kept = plan.kept_at(split::travelled(elapsed(duration, t)));
                    d.w(px(kept.w)).h(px(kept.h))
                },
            );
        Some(
            div()
                .absolute()
                .size_full()
                .child(kept)
                .child(self.new_pane_over(split, opening, cx))
                .into_any_element(),
        )
    }

    /// The panes as they were before the split, with the new pane opening over the one that was split.
    pub(in crate::window) fn render_nested_opening(
        &self,
        split: &Split,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let opening = self.opening_of(split)?;
        let before = opening.before.as_ref()?;
        let was = Split { id: split.id, layout: before.clone(), opening: None };
        let body = self.render_node(&was, was.layout.tree().root(), &mut Vec::new(), &before.panes(), cx);
        Some(
            div()
                .id("panes")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .child(body)
                .child(self.new_pane_over(split, opening, cx))
                .into_any_element(),
        )
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
        let pane = self.render_pane(split, opening.new, &opening.after.panes(), cx);
        let new_pane = plan.new_pane;
        let boxed = div()
            .absolute()
            .w(px(new_pane.w - gap_x))
            .h(px(new_pane.h - gap_y))
            // A pane without a surface of its own is on the terminal's colour, and a hairline marks its edge.
            .when(!gaps, |d| d.bg(ui.background).rule_left(ui))
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
