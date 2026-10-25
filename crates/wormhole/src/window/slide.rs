//! The sidebar and the right panel sliding in and out: in [`MOTION_PANEL`] easing out, out
//! in [`MOTION_PANEL_EXIT`] easing in. Anything that resizes the terminal animates a clip and resizes
//! the PTY once, never per frame: a column coming in is drawn over the terminal at its full width
//! and uncovered from its edge, and joins the layout (the terminal reflowing once) when it is in;
//! one going out leaves the layout at once (the terminal reflows then) and is covered up over the
//! terminal. The sidebar's edge carries the terminal with it meanwhile (shifted, not resized), so
//! the content moves with the column instead of jumping when it lands; the right panel's edge cuts
//! the terminal off instead (it reads from the left). Either way the terminal never lies under the
//! sliding column, which paints its own translucent surface over the blur, as it does at rest. The
//! traffic lights and the collapse button never move. Under Reduce Motion both are cuts.
use gpui_kit::*;
use wormhole_core::view::layout::Edge;

use crate::style::motion::{EASE_IN, EASE_OUT, MOTION_PANEL, MOTION_PANEL_EXIT};
use crate::window::Workspace;

/// A column on its way in or out.
pub(super) struct Slide {
    edge: Edge,
    opening: bool,
    /// The column's width at rest: what it is drawn at while it slides.
    width: f32,
    /// Which slide this is, so a later one isn't ended by an earlier one's timer.
    serial: u64,
    _end: Task<()>,
}

/// How far a sliding column is in, frame by frame: what everything that moves with it (the column,
/// the terminal, the title row's padding) animates by, so they stay together.
#[derive(Clone, Copy)]
pub(super) struct SlideMotion {
    pub(super) edge: Edge,
    opening: bool,
    width: f32,
    serial: u64,
}

impl SlideMotion {
    /// The slide's timing: in easing out, out easing in.
    pub(super) fn animation(self) -> Animation {
        let (duration, easing) =
            if self.opening { (MOTION_PANEL, EASE_OUT) } else { (MOTION_PANEL_EXIT, EASE_IN) };
        Animation::new(duration).with_easing(easing.easing())
    }

    /// How many points of the column are in at `t` (eased) of the way.
    pub(super) fn offset(self, t: f32) -> f32 {
        let (from, to) = if self.opening { (0.0, self.width) } else { (self.width, 0.0) };
        from + (to - from) * t
    }

    /// An animation id for `what`, unique to this slide.
    pub(super) fn id(self, what: &'static str) -> (&'static str, u64) {
        (what, self.serial)
    }
}

impl Workspace {
    /// `edge`'s column went from `before` to `after` points wide (a toggle): slide it, unless it
    /// neither came nor went, or motion is reduced.
    pub(super) fn start_slide(&mut self, edge: Edge, before: f32, after: f32, cx: &mut Context<Self>) {
        if (before > 0.0) == (after > 0.0) || cx.reduce_motion() {
            return;
        }
        let opening = after > 0.0;
        let serial = self.slide.as_ref().map_or(0, |slide| slide.serial + 1);
        let duration = if opening { MOTION_PANEL } else { MOTION_PANEL_EXIT };
        let end = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(duration).await;
            this.update(cx, |this, cx| {
                if this.slide.as_ref().is_some_and(|slide| slide.serial == serial) {
                    this.slide = None;
                    cx.notify();
                }
            })
            .ok();
        });
        self.slide = Some(Slide { edge, opening, width: before.max(after), serial, _end: end });
    }

    /// Whether `edge`'s column takes its place in the layout: not while it slides in over the
    /// terminal.
    pub(super) fn in_flow(&self, edge: Edge) -> bool {
        !self.slide.as_ref().is_some_and(|slide| slide.edge == edge && slide.opening)
    }

    /// The slide under way, if any.
    pub(super) fn slide_motion(&self) -> Option<SlideMotion> {
        let slide = self.slide.as_ref()?;
        Some(SlideMotion {
            edge: slide.edge,
            opening: slide.opening,
            width: slide.width,
            serial: slide.serial,
        })
    }

    /// The column sliding over the terminal, at its full width with the part not yet in (or no
    /// longer in) clipped away.
    pub(super) fn render_slide(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let motion = self.slide_motion()?;
        let width = motion.width;
        let column = match motion.edge {
            Edge::Sidebar => self.render_sidebar(width, false, window, cx),
            Edge::Panel => self.render_panel(width, window, cx).into_any_element(),
        };
        // The column keeps its width inside the clip, pinned to the window's edge, so its rows
        // don't reflow as it moves. It paints its own translucent surface, as at rest: the terminal
        // never lies under it (`carry_with_slide`), so the blur shows through from the first frame.
        let pinned = div().absolute().top_0().bottom_0().w(px(width)).child(column);
        let pinned = match motion.edge {
            Edge::Sidebar => pinned.left_0(),
            Edge::Panel => pinned.right_0(),
        };
        let clip = div().absolute().top_0().bottom_0().overflow_hidden().child(pinned);
        let clip = match motion.edge {
            Edge::Sidebar => clip.left_0(),
            Edge::Panel => clip.right_0(),
        };
        Some(
            clip.with_animation(motion.id("slide"), motion.animation(), move |d, t| {
                d.w(px(motion.offset(t)))
            })
            .into_any_element(),
        )
    }

    /// The terminal's column while a column slides, kept out from under it so the sliding column
    /// shows the window's blur and not the terminal. The sidebar's edge carries it (shifted, from
    /// the window's edge to the sidebar's width coming in, back going out); the right panel's edge
    /// cuts it off (its content keeps its width, reading from the left). Only drawn there: its size
    /// (and so the PTY's) changes once, when the column joins or leaves the layout.
    pub(super) fn carry_with_slide(&self, main: Div) -> AnyElement {
        let Some(motion) = self.slide_motion() else {
            return main.into_any_element();
        };
        let at = move |t: f32| px(motion.offset(t));
        let carried = match motion.edge {
            Edge::Sidebar => main
                .with_animation(motion.id("slide-carry"), motion.animation(), move |d, t| d.left(at(t)))
                .into_any_element(),
            // The wrapper ends at the panel's edge and clips there; the negative margin gives the
            // terminal back the width the wrapper lost, so it isn't resized frame by frame.
            Edge::Panel => div()
                .flex()
                .flex_1()
                .h_full()
                .overflow_hidden()
                .child(main.with_animation(motion.id("slide-cut-main"), motion.animation(), move |d, t| {
                    d.mr(-at(t))
                }))
                .with_animation(motion.id("slide-cut"), motion.animation(), move |d, t| d.mr(at(t)))
                .into_any_element(),
        };
        // Clipped to its place, so the part shifted past it doesn't show through a translucent
        // right panel.
        div()
            .flex()
            .flex_1()
            .min_w(px(crate::terminal_view::MIN_WIDTH))
            .h_full()
            .overflow_hidden()
            .child(carried)
            .into_any_element()
    }
}
